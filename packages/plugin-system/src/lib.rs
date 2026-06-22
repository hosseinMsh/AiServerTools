use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(Uuid),

    #[error("Plugin already registered: {0}")]
    PluginAlreadyRegistered(String),

    #[error("Permission denied: plugin '{plugin}' is not allowed to {action}")]
    PermissionDenied { plugin: String, action: String },

    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
}

// ─── Plugin Manifest ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub entry_point: String,
    pub permissions: Vec<PluginPermission>,
    pub hooks: Vec<PluginHook>,
    pub ui_panels: Vec<UiPanelDefinition>,
    pub dependencies: Vec<String>,
    pub min_app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginPermission {
    SshConnect,
    SshExec,
    DockerRead,
    DockerWrite,
    SftpRead,
    SftpWrite,
    TunnelManage,
    SecretRead,
    ConfigWrite,
    NetworkAccess,
    FilesystemRead,
    FilesystemWrite,
    AuditRead,
    UiCustomization,
    Automation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHook {
    pub event: String,
    pub handler: String,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPanelDefinition {
    pub id: String,
    pub title: String,
    pub slot: UiSlot,
    pub component: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UiSlot {
    Sidebar,
    Toolbar,
    Dashboard,
    ServerDetail,
    ContainerDetail,
    TunnelPanel,
    Settings,
}

// ─── Plugin Instance ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub installed_at: String,
    pub last_loaded: Option<String>,
    pub error: Option<String>,
    pub config: serde_json::Value,
}

pub struct PluginInstance {
    pub state: RwLock<PluginState>,
    // In production: WASM runtime, sandboxed host functions, etc.
}

impl PluginInstance {
    pub fn new(manifest: PluginManifest) -> Self {
        Self {
            state: RwLock::new(PluginState {
                manifest,
                enabled: false,
                installed_at: chrono::Utc::now().to_rfc3339(),
                last_loaded: None,
                error: None,
                config: serde_json::json!({}),
            }),
        }
    }

    pub fn load(&self) -> Result<(), PluginError> {
        // TODO: Load WASM module, initialize sandbox, register hooks
        let mut state = self.state.write();
        state.enabled = true;
        state.last_loaded = Some(chrono::Utc::now().to_rfc3339());
        info!("Plugin loaded: {} v{}", state.manifest.name, state.manifest.version);
        Ok(())
    }

    pub fn unload(&self) -> Result<(), PluginError> {
        let mut state = self.state.write();
        state.enabled = false;
        info!("Plugin unloaded: {}", state.manifest.name);
        Ok(())
    }
}

// ─── Plugin Manager ────────────────────────────────────────────────────────────

pub struct PluginManager {
    plugins: Arc<DashMap<Uuid, Arc<PluginInstance>>>,
    permissions: Arc<DashMap<String, Vec<PluginPermission>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(DashMap::new()),
            permissions: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, manifest: PluginManifest) -> Result<Uuid, PluginError> {
        // Check for duplicate names
        for entry in self.plugins.iter() {
            if entry.value().state.read().manifest.name == manifest.name {
                return Err(PluginError::PluginAlreadyRegistered(manifest.name));
            }
        }

        let id = manifest.id;
        let instance = Arc::new(PluginInstance::new(manifest.clone()));

        // Validate permissions
        self.check_permission_safety(&manifest.permissions)?;

        // Register permissions
        self.permissions.insert(manifest.name.clone(), manifest.permissions.clone());

        self.plugins.insert(id, instance);
        info!("Plugin registered: {} (id={})", manifest.name, id);
        Ok(id)
    }

    pub fn load_plugin(&self, id: &Uuid) -> Result<(), PluginError> {
        let plugin = self.plugins.get(id)
            .ok_or(PluginError::PluginNotFound(*id))?;
        plugin.load()
    }

    pub fn unload_plugin(&self, id: &Uuid) -> Result<(), PluginError> {
        let plugin = self.plugins.get(id)
            .ok_or(PluginError::PluginNotFound(*id))?;
        plugin.unload()
    }

    pub fn remove_plugin(&self, id: &Uuid) -> Result<(), PluginError> {
        let (_, plugin) = self.plugins.remove(id)
            .ok_or(PluginError::PluginNotFound(*id))?;
        let name = plugin.state.read().manifest.name.clone();
        self.permissions.remove(&name);
        info!("Plugin removed: {} (id={})", name, id);
        Ok(())
    }

    pub fn check_plugin_permission(&self, plugin_name: &str, required: &PluginPermission) -> bool {
        self.permissions.get(plugin_name)
            .map(|perms| perms.contains(required))
            .unwrap_or(false)
    }

    pub fn list_plugins(&self) -> Vec<PluginState> {
        self.plugins.iter().map(|e| e.value().state.read().clone()).collect()
    }

    pub fn get_plugin(&self, id: &Uuid) -> Option<Arc<PluginInstance>> {
        self.plugins.get(id).map(|e| e.value().clone())
    }

    fn check_permission_safety(&self, permissions: &[PluginPermission]) -> Result<(), PluginError> {
        // Dangerous permissions require special handling
        for perm in permissions {
            match perm {
                PluginPermission::FilesystemWrite
                | PluginPermission::SecretRead
                | PluginPermission::NetworkAccess
                | PluginPermission::ConfigWrite => {
                    warn!("Plugin requesting high-risk permission: {:?}", perm);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manifest_serialization() {
        let manifest = PluginManifest {
            id: Uuid::new_v4(),
            name: "Docker Dashboard".into(),
            version: "1.0.0".into(),
            author: "InfraCore".into(),
            description: "Enhanced Docker container dashboard".into(),
            entry_point: "plugin.wasm".into(),
            permissions: vec![PluginPermission::DockerRead, PluginPermission::DockerWrite],
            hooks: vec![PluginHook {
                event: "container:started".into(),
                handler: "onContainerStarted".into(),
                priority: 100,
            }],
            ui_panels: vec![UiPanelDefinition {
                id: "docker-detail".into(),
                title: "Docker Detail".into(),
                slot: UiSlot::ContainerDetail,
                component: "DockerDetailPanel".into(),
                icon: "ContainerIcon".into(),
            }],
            dependencies: vec![],
            min_app_version: "1.0.0".into(),
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(json.contains("Docker Dashboard"));

        let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Docker Dashboard");
        assert!(deserialized.permissions.contains(&PluginPermission::DockerRead));
    }

    #[test]
    fn test_plugin_registration() {
        let manager = PluginManager::new();

        let manifest = PluginManifest {
            id: Uuid::new_v4(),
            name: "Test Plugin".into(),
            version: "0.1.0".into(),
            author: "Test".into(),
            description: "Test plugin".into(),
            entry_point: "test.wasm".into(),
            permissions: vec![PluginPermission::SshExec],
            hooks: vec![],
            ui_panels: vec![],
            dependencies: vec![],
            min_app_version: "1.0.0".into(),
        };

        let id = manager.register(manifest).unwrap();
        assert_eq!(manager.list_plugins().len(), 1);

        // Duplicate should fail
        let dup = PluginManifest {
            id: Uuid::new_v4(),
            name: "Test Plugin".into(),
            ..Default::default()
        };
        assert!(manager.register(dup).is_err());
    }
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            version: "0.1.0".into(),
            author: String::new(),
            description: String::new(),
            entry_point: String::new(),
            permissions: vec![],
            hooks: vec![],
            ui_panels: vec![],
            dependencies: vec![],
            min_app_version: "1.0.0".into(),
        }
    }
}
