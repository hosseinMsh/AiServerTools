use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SftpError {
    #[error("Session not connected")]
    NotConnected,

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Transfer failed: {0}")]
    TransferFailed(String),

    #[error("Path error: {0}")]
    PathError(String),

    #[error("SSH error: {0}")]
    SshError(String),
}

// ─── File Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub permissions: String,
    pub modified: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub owner: String,
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub elapsed_secs: f64,
    pub status: TransferStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub id: Uuid,
    pub kind: TransferKind,
    pub local_path: PathBuf,
    pub remote_path: String,
    pub overwrite: bool,
    pub preserve_attrs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferKind {
    Upload,
    Download,
}

// ─── Remote File System ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SftpSession {
    pub session_id: Uuid,
    pub remote_path: String,
}

impl SftpSession {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            session_id,
            remote_path: "/".to_string(),
        }
    }

    pub async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>, SftpError> {
        // In production, this uses SSH exec to run `ls -la` or uses SFTP subsystem
        // For Tauri IPC, the frontend sends SSH commands via the SSH engine
        Ok(Vec::new())
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>, SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn write_file(&self, path: &str, data: &[u8]) -> Result<(), SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn create_directory(&self, path: &str) -> Result<(), SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn remove_directory(&self, path: &str, recursive: bool) -> Result<(), SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<(), SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn chmod(&self, path: &str, mode: u32) -> Result<(), SftpError> {
        Err(SftpError::NotConnected)
    }

    pub async fn stat(&self, path: &str) -> Result<FileInfo, SftpError> {
        Err(SftpError::FileNotFound(path.to_string()))
    }

    pub fn change_directory(&mut self, path: &str) -> Result<(), SftpError> {
        let new_path = if path.starts_with('/') {
            path.to_string()
        } else {
            let current = self.remote_path.trim_end_matches('/');
            format!("{}/{}", current, path)
        };
        self.remote_path = new_path;
        Ok(())
    }
}

// ─── File Manager (High-Level) ─────────────────────────────────────────────────

pub struct SftpManager {
    sessions: parking_lot::RwLock<std::collections::HashMap<Uuid, SftpSession>>,
    transfers: parking_lot::RwLock<std::collections::HashMap<Uuid, TransferProgress>>,
}

impl SftpManager {
    pub fn new() -> Self {
        Self {
            sessions: parking_lot::RwLock::new(std::collections::HashMap::new()),
            transfers: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn create_session(&self, session_id: Uuid) -> SftpSession {
        let session = SftpSession::new(session_id);
        self.sessions.write().insert(session_id, session.clone());
        session
    }

    pub fn get_session(&self, session_id: &Uuid) -> Option<SftpSession> {
        self.sessions.read().get(session_id).cloned()
    }

    pub fn remove_session(&self, session_id: &Uuid) {
        self.sessions.write().remove(session_id);
    }
}

impl Default for SftpManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Safe File Operations Strategy ─────────────────────────────────────────────

pub struct SafeFileOps;

impl SafeFileOps {
    /// Creates a backup before overwriting
    pub fn backup_path(original: &Path) -> PathBuf {
        let mut backup = original.to_path_buf();
        let ext = backup
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let stem = backup
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        backup.set_file_name(format!("{}_{}{}", stem, timestamp, ext));
        backup
    }

    /// Warns if file exists before overwrite
    pub fn check_overwrite(path: &Path) -> bool {
        path.exists()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info_serialization() {
        let info = FileInfo {
            name: "config.yml".into(),
            path: "/etc/config.yml".into(),
            size: 1024,
            permissions: "-rw-r--r--".into(),
            modified: "2024-01-01T00:00:00Z".into(),
            is_dir: false,
            is_symlink: false,
            owner: "root".into(),
            group: "root".into(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("config.yml"));

        let deserialized: FileInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.owner, "root");
    }

    #[test]
    fn test_backup_path() {
        let original = Path::new("/etc/nginx/nginx.conf");
        let backup = SafeFileOps::backup_path(original);
        assert!(backup.to_string_lossy().contains("nginx"));
        assert!(backup.to_string_lossy().contains(".conf"));
        assert_ne!(backup, original);
    }
}
