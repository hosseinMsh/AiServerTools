use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptFailed(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

// ─── Key Derivation ────────────────────────────────────────────────────────────

pub fn derive_key(password: &[u8], salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password, salt, &mut key)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(key)
}

pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    salt
}

// ─── Symmetric Encryption (AES-256-GCM) ────────────────────────────────────────

pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<String, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptFailed(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptFailed(e.to_string()))?;

    let mut result = String::new();
    result.push_str(&BASE64.encode(nonce_bytes));
    result.push('.');
    result.push_str(&BASE64.encode(&ciphertext));
    Ok(result)
}

pub fn decrypt(encrypted: &str, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    let parts: Vec<&str> = encrypted.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(CryptoError::InvalidFormat("Expected nonce.ciphertext".into()));
    }

    let nonce_bytes = BASE64.decode(parts[0])
        .map_err(|e| CryptoError::DecryptFailed(e.to_string()))?;
    let ciphertext = BASE64.decode(parts[1])
        .map_err(|e| CryptoError::DecryptFailed(e.to_string()))?;

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| CryptoError::DecryptFailed(e.to_string()))
}

// ─── Password Hashing ──────────────────────────────────────────────────────────

pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let salt = generate_salt();
    let mut hash = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt, &mut hash)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

    let mut result = Vec::with_capacity(48);
    result.extend_from_slice(&salt);
    result.extend_from_slice(&hash);
    Ok(BASE64.encode(&result))
}

pub fn verify_password(password: &str, stored: &str) -> Result<bool, CryptoError> {
    let data = BASE64.decode(stored)
        .map_err(|e| CryptoError::InvalidFormat(e.to_string()))?;

    if data.len() != 48 {
        return Err(CryptoError::InvalidFormat("Invalid hash length".into()));
    }

    let (salt, expected_hash) = data.split_at(16);
    let mut computed_hash = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut computed_hash)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

    Ok(computed_hash == expected_hash)
}

// ─── SHA-256 Hashing ──────────────────────────────────────────────────────────

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

// ─── Key Wrapper ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub encrypted: String,
    pub salt: String,
    pub nonce: String,
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key(b"master_password", &generate_salt()).unwrap();
        let plaintext = b"SSH_PRIVATE_KEY: -----BEGIN OPENSSH PRIVATE KEY-----\ntest123";

        let encrypted = encrypt(plaintext, &key).unwrap();
        assert_ne!(encrypted, String::from_utf8_lossy(plaintext));

        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_password_hashing_verify() {
        let hash = hash_password("correct-horse-battery-staple").unwrap();
        assert!(verify_password("correct-horse-battery-staple", &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_sha256_consistency() {
        let hash1 = sha256_hex(b"test data");
        let hash2 = sha256_hex(b"test data");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_derived_key_determinism() {
        let password = b"test_password";
        let salt = [0x42u8; 16];
        let key1 = derive_key(password, &salt).unwrap();
        let key2 = derive_key(password, &salt).unwrap();
        assert_eq!(key1, key2);
    }
}
