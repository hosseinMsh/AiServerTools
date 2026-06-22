use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{info, warn, error};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Replay error: {0}")]
    ReplayError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
}

// ─── Audit Event Types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: String,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub session_id: Option<Uuid>,
    pub server_id: Option<Uuid>,
    pub action: String,
    pub resource: String,
    pub details: serde_json::Value,
    pub result: AuditResult,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditEventType {
    SshConnect,
    SshDisconnect,
    SshCommand,
    SshShellInput,
    DockerExec,
    DockerStart,
    DockerStop,
    DockerRestart,
    DockerRemove,
    SftpUpload,
    SftpDownload,
    SftpDelete,
    TunnelStart,
    TunnelStop,
    ConfigCreate,
    ConfigUpdate,
    ConfigDelete,
    SecretAccess,
    Login,
    Logout,
    AdminAction,
    PluginAction,
    AiSuggestion,
    PolicyViolation,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditResult {
    Success,
    Failure(String),
    Denied(String),
    Pending,
}

// ─── Session Recording ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecording {
    pub id: Uuid,
    pub session_id: Uuid,
    pub server_label: String,
    pub user: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub chunks: Vec<RecordingChunk>,
    pub total_chunks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingChunk {
    pub timestamp: String,
    pub kind: ChunkKind,
    pub data: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkKind {
    Stdin,
    Stdout,
    Stderr,
    Resize,
    Event,
}

// ─── Audit Logger ──────────────────────────────────────────────────────────────

pub struct AuditLogger {
    events: RwLock<VecDeque<AuditEvent>>,
    recordings: RwLock<std::collections::HashMap<Uuid, SessionRecording>>,
    max_events: usize,
}

impl AuditLogger {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_events)),
            recordings: RwLock::new(std::collections::HashMap::new()),
            max_events,
        }
    }

    pub fn log(&self, event: AuditEvent) {
        let mut events = self.events.write();
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event.clone());

        // Also emit via tracing
        info!(
            audit_event = %event.event_type.as_ref(),
            action = %event.action,
            resource = %event.resource,
            result = ?event.result,
        );
    }

    pub fn start_recording(&self, session_id: Uuid, server_label: String, user: String) -> SessionRecording {
        let recording = SessionRecording {
            id: Uuid::new_v4(),
            session_id,
            server_label,
            user,
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            chunks: Vec::new(),
            total_chunks: 0,
        };

        self.recordings.write().insert(session_id, recording.clone());
        info!("Session recording started: {}", session_id);
        recording
    }

    pub fn record_chunk(&self, session_id: &Uuid, kind: ChunkKind, data: String) -> Result<(), AuditError> {
        let mut recordings = self.recordings.write();
        let recording = recordings.get_mut(session_id)
            .ok_or(AuditError::SessionNotFound(*session_id))?;

        let chunk = RecordingChunk {
            timestamp: chrono::Utc::now().to_rfc3339(),
            kind,
            data,
            sequence: recording.total_chunks,
        };

        recording.chunks.push(chunk);
        recording.total_chunks += 1;
        Ok(())
    }

    pub fn stop_recording(&self, session_id: &Uuid) -> Result<SessionRecording, AuditError> {
        let mut recordings = self.recordings.write();
        let recording = recordings.get_mut(session_id)
            .ok_or(AuditError::SessionNotFound(*session_id))?;
        recording.ended_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(recording.clone())
    }

    pub fn get_recording(&self, session_id: &Uuid) -> Option<SessionRecording> {
        self.recordings.read().get(session_id).cloned()
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<AuditEvent> {
        let events = self.events.read();
        events.iter().rev().take(count).cloned().collect()
    }

    pub fn query_events(&self, filter: AuditQuery) -> Vec<AuditEvent> {
        let events = self.events.read();
        events.iter().filter(|e| {
            if let Some(event_type) = &filter.event_type {
                if &e.event_type != event_type { return false; }
            }
            if let Some(user) = &filter.user_id {
                if e.user_id.as_ref().map_or(true, |u| u != user) { return false; }
            }
            if let Some(action) = &filter.action {
                if !e.action.contains(action) { return false; }
            }
            if let Some(resource) = &filter.resource {
                if !e.resource.contains(resource) { return false; }
            }
            if let Some(from) = &filter.from {
                if e.timestamp < *from { return false; }
            }
            if let Some(to) = &filter.to {
                if e.timestamp > *to { return false; }
            }
            true
        }).cloned().collect()
    }
}

#[derive(Debug, Default)]
pub struct AuditQuery {
    pub event_type: Option<AuditEventType>,
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub resource: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10_000)
    }
}

// ─── Serialization helpers ─────────────────────────────────────────────────────

impl AsRef<str> for AuditEventType {
    fn as_ref(&self) -> &str {
        match self {
            AuditEventType::SshConnect => "ssh_connect",
            AuditEventType::SshDisconnect => "ssh_disconnect",
            AuditEventType::SshCommand => "ssh_command",
            AuditEventType::SshShellInput => "ssh_shell_input",
            AuditEventType::DockerExec => "docker_exec",
            AuditEventType::DockerStart => "docker_start",
            AuditEventType::DockerStop => "docker_stop",
            AuditEventType::DockerRestart => "docker_restart",
            AuditEventType::DockerRemove => "docker_remove",
            AuditEventType::SftpUpload => "sftp_upload",
            AuditEventType::SftpDownload => "sftp_download",
            AuditEventType::SftpDelete => "sftp_delete",
            AuditEventType::TunnelStart => "tunnel_start",
            AuditEventType::TunnelStop => "tunnel_stop",
            AuditEventType::ConfigCreate => "config_create",
            AuditEventType::ConfigUpdate => "config_update",
            AuditEventType::ConfigDelete => "config_delete",
            AuditEventType::SecretAccess => "secret_access",
            AuditEventType::Login => "login",
            AuditEventType::Logout => "logout",
            AuditEventType::AdminAction => "admin_action",
            AuditEventType::PluginAction => "plugin_action",
            AuditEventType::AiSuggestion => "ai_suggestion",
            AuditEventType::PolicyViolation => "policy_violation",
            AuditEventType::Error => "error",
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logging() {
        let logger = AuditLogger::new(100);

        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: AuditEventType::SshConnect,
            user_id: Some("admin".into()),
            session_id: Some(Uuid::new_v4()),
            server_id: Some(Uuid::new_v4()),
            action: "ssh_connect".into(),
            resource: "server:prod-web-01".into(),
            details: serde_json::json!({"host": "10.0.0.1", "port": 22}),
            result: AuditResult::Success,
            ip_address: Some("192.168.1.1".into()),
            user_agent: Some("InfraCore/1.0".into()),
        };

        logger.log(event);

        let recent = logger.get_recent_events(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].action, "ssh_connect");
    }

    #[test]
    fn test_session_recording() {
        let logger = AuditLogger::new(1000);
        let session_id = Uuid::new_v4();

        let recording = logger.start_recording(session_id, "prod-web-01".into(), "admin".into());
        assert!(recording.ended_at.is_none());

        logger.record_chunk(&session_id, ChunkKind::Stdin, "ls -la\n".into()).unwrap();
        logger.record_chunk(&session_id, ChunkKind::Stdout, "total 24\n".into()).unwrap();

        let stopped = logger.stop_recording(&session_id).unwrap();
        assert_eq!(stopped.total_chunks, 2);
        assert!(stopped.ended_at.is_some());
    }
}
