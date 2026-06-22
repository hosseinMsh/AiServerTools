use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use parking_lot::RwLock;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Vault locked")]
    VaultLocked,

    #[error("Vault entry not found: {0}")]
    EntryNotFound(String),

    #[error("Policy denied: {0}")]
    PolicyDenied(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// ─── Encryption Module ────────────────────────────────────────────────────────

pub struct EncryptionEngine {
    master_key: RwLock<Option<[u8; 32]>>,
}

impl EncryptionEngine {
    pub fn new() -> Self {
        Self {
            master_key: RwLock::new(None),
        }
    }

    pub fn initialize(&self, key_material: &[u8]) -> Result<(), SecurityError> {
        let mut key = [0u8; 32];
        let salt = b"INFRA_CORE_SALT_2024";

        Argon2::default()
            .hash_password_into(key_material, salt, &mut key)
            .map_err(|e| SecurityError::KeyDerivationFailed(e.to_string()))?;

        *self.master_key.write() = Some(key);
        info!("Encryption engine initialized");
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.master_key.read().is_some()
    }

    pub fn encrypt(&self, plaintext: &[u8], aad: &[u8]) -> Result<String, SecurityError> {
        let key = self.master_key.read().ok_or(SecurityError::VaultLocked)?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

        // Format: nonce (base64) . ciphertext+tag (base64)
        let mut result = String::new();
        result.push_str(&BASE64.encode(nonce_bytes));
        result.push('.');
        result.push_str(&BASE64.encode(&ciphertext));

        Ok(result)
    }

    pub fn decrypt(&self, encrypted: &str, aad: &[u8]) -> Result<Vec<u8>, SecurityError> {
        let key = self.master_key.read().ok_or(SecurityError::VaultLocked)?;

        let parts: Vec<&str> = encrypted.splitn(2, '.').collect();
        if parts.len() != 2 {
            return Err(SecurityError::DecryptionFailed(
                "Invalid encrypted format".into(),
            ));
        }

        let nonce_bytes = BASE64
            .decode(parts[0])
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;
        let ciphertext = BASE64
            .decode(parts[1])
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))?;

        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| SecurityError::DecryptionFailed(e.to_string()))
    }

    pub fn hash_password(password: &str) -> Result<String, SecurityError> {
        let salt = {
            let mut s = [0u8; 16];
            OsRng.fill_bytes(&mut s);
            s
        };

        let mut hash = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut hash)
            .map_err(|e| SecurityError::KeyDerivationFailed(e.to_string()))?;

        let mut result = Vec::with_capacity(48);
        result.extend_from_slice(&salt);
        result.extend_from_slice(&hash);

        Ok(BASE64.encode(&result))
    }

    pub fn verify_password(password: &str, stored: &str) -> Result<bool, SecurityError> {
        let data = BASE64
            .decode(stored)
            .map_err(|e| SecurityError::InvalidInput(e.to_string()))?;

        if data.len() != 48 {
            return Err(SecurityError::InvalidInput("Invalid stored hash".into()));
        }

        let (salt, expected_hash) = data.split_at(16);

        let mut computed_hash = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), salt, &mut computed_hash)
            .map_err(|e| SecurityError::KeyDerivationFailed(e.to_string()))?;

        Ok(computed_hash == expected_hash)
    }
}

impl Default for EncryptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Secure Credential Vault ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: Uuid,
    pub label: String,
    pub kind: CredentialKind,
    pub encrypted_data: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialKind {
    SshKey,
    Password,
    ApiKey,
    Token,
    Certificate,
    Other(String),
}

pub struct CredentialVault {
    encryption: Arc<EncryptionEngine>,
    entries: RwLock<HashMap<Uuid, VaultEntry>>,
}

impl CredentialVault {
    pub fn new(encryption: Arc<EncryptionEngine>) -> Self {
        Self {
            encryption,
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn store(
        &self,
        label: &str,
        kind: CredentialKind,
        plaintext: &str,
        tags: Vec<String>,
    ) -> Result<VaultEntry, SecurityError> {
        if !self.encryption.is_initialized() {
            return Err(SecurityError::VaultLocked);
        }

        let encrypted = self
            .encryption
            .encrypt(plaintext.as_bytes(), label.as_bytes())?;
        let now = chrono::Utc::now().to_rfc3339();

        let entry = VaultEntry {
            id: Uuid::new_v4(),
            label: label.to_string(),
            kind,
            encrypted_data: encrypted,
            created_at: now.clone(),
            updated_at: now,
            tags,
        };

        self.entries.write().insert(entry.id, entry.clone());
        info!("Credential stored: {} (id={})", label, entry.id);
        Ok(entry)
    }

    pub fn retrieve(&self, id: &Uuid) -> Result<String, SecurityError> {
        let entry = self
            .entries
            .read()
            .get(id)
            .cloned()
            .ok_or(SecurityError::EntryNotFound(id.to_string()))?;

        let plaintext = self
            .encryption
            .decrypt(&entry.encrypted_data, entry.label.as_bytes())?;

        String::from_utf8(plaintext)
            .map_err(|_| SecurityError::DecryptionFailed("Decrypted data is not UTF-8".into()))
    }

    pub fn delete(&self, id: &Uuid) -> Result<(), SecurityError> {
        self.entries.write().remove(id);
        info!("Credential deleted: {}", id);
        Ok(())
    }

    pub fn list(&self) -> Vec<VaultEntry> {
        self.entries.read().values().cloned().collect()
    }
}

// ─── Policy Engine ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: Uuid,
    pub name: String,
    pub rules: Vec<PolicyRule>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub action: PolicyAction,
    pub resource: String,
    pub effect: PolicyEffect,
    pub require_confirmation: bool,
    pub allowed_users: Vec<String>,
    pub allowed_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    SshConnect,
    SshExec,
    DockerExec,
    SftpTransfer,
    TunnelCreate,
    ConfigWrite,
    SecretRead,
    AdminAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
    RequireMfa,
    RequireApproval,
}

pub struct PolicyEngine {
    policies: RwLock<Vec<Policy>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
        }
    }

    pub fn add_policy(&self, policy: Policy) {
        self.policies.write().push(policy);
    }

    pub fn evaluate(
        &self,
        action: &PolicyAction,
        resource: &str,
        user_role: &str,
    ) -> Result<PolicyDecision, SecurityError> {
        let policies = self.policies.read();

        for policy in policies.iter().filter(|p| p.enabled) {
            for rule in &policy.rules {
                if rule.action != *action {
                    continue;
                }
                if !resource.starts_with(&rule.resource) {
                    continue;
                }

                if !rule.allowed_roles.is_empty()
                    && !rule.allowed_roles.contains(&user_role.to_string())
                {
                    continue;
                }

                return Ok(PolicyDecision {
                    allowed: rule.effect == PolicyEffect::Allow,
                    require_confirmation: rule.require_confirmation,
                    rule: Some(rule.clone()),
                });
            }
        }

        // Default: deny
        Ok(PolicyDecision {
            allowed: false,
            require_confirmation: true,
            rule: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub require_confirmation: bool,
    pub rule: Option<PolicyRule>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        let mut engine = Self::new();
        // Default security policies
        engine.add_policy(Policy {
            id: Uuid::new_v4(),
            name: "Default: Allow SSH connect, deny destructive actions".into(),
            rules: vec![
                PolicyRule {
                    action: PolicyAction::SshConnect,
                    resource: "*",
                    effect: PolicyEffect::Allow,
                    require_confirmation: false,
                    allowed_users: vec![],
                    allowed_roles: vec!["admin".into(), "operator".into()],
                },
                PolicyRule {
                    action: PolicyAction::SshExec,
                    resource: "*",
                    effect: PolicyEffect::Allow,
                    require_confirmation: false,
                    allowed_users: vec![],
                    allowed_roles: vec!["admin".into(), "operator".into()],
                },
                PolicyRule {
                    action: PolicyAction::DockerExec,
                    resource: "*",
                    effect: PolicyEffect::Allow,
                    require_confirmation: true,
                    allowed_users: vec![],
                    allowed_roles: vec!["admin".into()],
                },
                PolicyRule {
                    action: PolicyAction::SecretRead,
                    resource: "*",
                    effect: PolicyEffect::Allow,
                    require_confirmation: true,
                    allowed_users: vec![],
                    allowed_roles: vec!["admin".into()],
                },
                PolicyRule {
                    action: PolicyAction::AdminAction,
                    resource: "*",
                    effect: PolicyEffect::Deny,
                    require_confirmation: true,
                    allowed_users: vec![],
                    allowed_roles: vec!["admin".into()],
                },
            ],
            enabled: true,
        });
        engine
    }
}

// ─── Security Manager (Facade) ─────────────────────────────────────────────────

pub struct SecurityManager {
    pub encryption: Arc<EncryptionEngine>,
    pub vault: Arc<CredentialVault>,
    pub policy: Arc<PolicyEngine>,
}

impl SecurityManager {
    pub fn new() -> Self {
        let encryption = Arc::new(EncryptionEngine::new());
        let vault = Arc::new(CredentialVault::new(encryption.clone()));
        let policy = Arc::new(PolicyEngine::default());
        Self {
            encryption,
            vault,
            policy,
        }
    }

    pub fn unlock(&self, master_password: &str) -> Result<(), SecurityError> {
        self.encryption.initialize(master_password.as_bytes())?;
        info!("Security manager unlocked");
        Ok(())
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let engine = EncryptionEngine::new();
        engine.initialize(b"test_master_key_12345").unwrap();

        let plaintext = b"Sensitive credential data: api_key=sk-1234567890abcdef";
        let encrypted = engine.encrypt(plaintext, b"test").unwrap();
        assert_ne!(encrypted, String::from_utf8_lossy(plaintext));
        assert!(encrypted.contains('.'));

        let decrypted = engine.decrypt(&encrypted, b"test").unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_vault_store_retrieve() {
        let encryption = Arc::new(EncryptionEngine::new());
        encryption.initialize(b"vault_test_key").unwrap();
        let vault = CredentialVault::new(encryption);

        let entry = vault
            .store(
                "Production DB",
                CredentialKind::Password,
                "s3cret!",
                vec!["prod".into(), "db".into()],
            )
            .unwrap();

        let retrieved = vault.retrieve(&entry.id).unwrap();
        assert_eq!(retrieved, "s3cret!");

        let list = vault.list();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_password_hashing() {
        let hash = EncryptionEngine::hash_password("my_secure_password").unwrap();
        assert!(EncryptionEngine::verify_password("my_secure_password", &hash).unwrap());
        assert!(!EncryptionEngine::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_policy_evaluation() {
        let engine = PolicyEngine::default();

        // Admin allowed to SSH
        let decision = engine
            .evaluate(&PolicyAction::SshConnect, "server:prod-web-01", "admin")
            .unwrap();
        assert!(decision.allowed);

        // Viewer denied admin action
        let decision = engine
            .evaluate(&PolicyAction::AdminAction, "system:config", "viewer")
            .unwrap();
        assert!(!decision.allowed);
    }
}
