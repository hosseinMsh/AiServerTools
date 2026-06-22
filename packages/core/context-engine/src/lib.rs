use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, debug, warn};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Server not found: {0}")]
    ServerNotFound(Uuid),

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("State not found: {0}")]
    StateNotFound(String),

    #[error("Cache expired: {0}")]
    CacheExpired(String),
}

// ─── Server State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub id: Uuid,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub status: ServerStatus,
    pub last_seen: Option<String>,
    pub metrics: Option<ServerMetrics>,
    pub os_info: Option<OsInfo>,
    pub tags: Vec<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Online,
    Offline,
    Unreachable,
    Maintenance,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub cpu_percent: f64,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_percent: f64,
    pub disk_total_gb: u64,
    pub disk_used_gb: u64,
    pub disk_percent: f64,
    pub load_1m: f64,
    pub load_5m: f64,
    pub load_15m: f64,
    pub uptime_seconds: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub collected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub platform: String,
}

// ─── Command History ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    pub id: Uuid,
    pub session_id: Uuid,
    pub server_id: Uuid,
    pub command: String,
    pub output_preview: String,
    pub exit_code: Option<i32>,
    pub executed_at: String,
    pub duration_ms: u64,
    pub status: CommandStatus,
    pub ai_suggested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandStatus {
    Success,
    Failed,
    Running,
    Cancelled,
    Timeout,
}

// ─── Conversation / AI Context ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConversation {
    pub id: Uuid,
    pub server_id: Option<Uuid>,
    pub messages: Vec<AiMessage>,
    pub started_at: String,
    pub updated_at: String,
    pub context_type: ConversationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: AiRole,
    pub content: String,
    pub timestamp: String,
    pub tool_calls: Vec<AiToolCall>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConversationType {
    Chat,
    Diagnostics,
    Deployment,
    Debug,
    Automation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolCall {
    pub id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
}

// ─── Server Groups ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerGroup {
    pub id: Uuid,
    pub name: String,
    pub environment: Environment,
    pub servers: Vec<Uuid>,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Staging,
    Production,
    DisasterRecovery,
    Custom(String),
}

// ─── Context Engine ────────────────────────────────────────────────────────────

pub struct ContextEngine {
    servers: Arc<DashMap<Uuid, ServerState>>,
    commands: Arc<DashMap<Uuid, CommandRecord>>,
    conversations: Arc<DashMap<Uuid, AiConversation>>,
    groups: Arc<RwLock<Vec<ServerGroup>>>,
    metrics_cache: Arc<DashMap<Uuid, ServerMetrics>>,
}

impl ContextEngine {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(DashMap::new()),
            commands: Arc::new(DashMap::new()),
            conversations: Arc::new(DashMap::new()),
            groups: Arc::new(RwLock::new(Vec::new())),
            metrics_cache: Arc::new(DashMap::new()),
        }
    }

    // ─── Server State ──────────────────────────────────────────────────────

    pub fn register_server(&self, config: ServerState) {
        self.servers.insert(config.id, config);
        debug!("Server registered: {}", config.id);
    }

    pub fn update_server_status(&self, id: &Uuid, status: ServerStatus) {
        if let Some(mut server) = self.servers.get_mut(id) {
            server.status = status;
            server.last_seen = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    pub fn update_server_metrics(&self, id: &Uuid, metrics: ServerMetrics) {
        if let Some(mut server) = self.servers.get_mut(id) {
            server.metrics = Some(metrics.clone());
        }
        self.metrics_cache.insert(*id, metrics);
    }

    pub fn get_server(&self, id: &Uuid) -> Option<ServerState> {
        self.servers.get(id).map(|s| s.clone())
    }

    pub fn list_servers(&self) -> Vec<ServerState> {
        self.servers.iter().map(|s| s.clone()).collect()
    }

    pub fn list_servers_by_group(&self, group_name: &str) -> Vec<ServerState> {
        self.servers.iter()
            .filter(|s| s.group.as_deref() == Some(group_name))
            .map(|s| s.clone())
            .collect()
    }

    // ─── Command History ───────────────────────────────────────────────────

    pub fn record_command(&self, record: CommandRecord) {
        self.commands.insert(record.id, record);
    }

    pub fn get_command_history(&self, server_id: &Uuid, limit: usize) -> Vec<CommandRecord> {
        let mut history: Vec<CommandRecord> = self.commands.iter()
            .filter(|c| c.server_id == *server_id)
            .map(|c| c.clone())
            .collect();
        history.sort_by(|a, b| b.executed_at.cmp(&a.executed_at));
        history.truncate(limit);
        history
    }

    // ─── AI Conversations ──────────────────────────────────────────────────

    pub fn create_conversation(&self, server_id: Option<Uuid>, context_type: ConversationType) -> AiConversation {
        let now = chrono::Utc::now().to_rfc3339();
        let conversation = AiConversation {
            id: Uuid::new_v4(),
            server_id,
            messages: Vec::new(),
            started_at: now.clone(),
            updated_at: now,
            context_type,
        };
        self.conversations.insert(conversation.id, conversation.clone());
        conversation
    }

    pub fn add_message(&self, conversation_id: &Uuid, message: AiMessage) -> Option<AiConversation> {
        if let Some(mut conv) = self.conversations.get_mut(conversation_id) {
            conv.messages.push(message);
            conv.updated_at = chrono::Utc::now().to_rfc3339();
            return Some(conv.clone());
        }
        None
    }

    pub fn get_conversation(&self, id: &Uuid) -> Option<AiConversation> {
        self.conversations.get(id).map(|c| c.clone())
    }

    pub fn get_recent_conversations(&self, limit: usize) -> Vec<AiConversation> {
        let mut convs: Vec<AiConversation> = self.conversations.iter()
            .map(|c| c.clone())
            .collect();
        convs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        convs.truncate(limit);
        convs
    }

    // ─── Server Groups ─────────────────────────────────────────────────────

    pub fn create_group(&self, group: ServerGroup) {
        self.groups.write().push(group);
    }

    pub fn list_groups(&self) -> Vec<ServerGroup> {
        self.groups.read().clone()
    }

    pub fn get_group(&self, id: &Uuid) -> Option<ServerGroup> {
        self.groups.read().iter().find(|g| g.id == *id).cloned()
    }
}

impl Default for ContextEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_state_lifecycle() {
        let ctx = ContextEngine::new();

        let server = ServerState {
            id: Uuid::new_v4(),
            label: "Prod Web 01".into(),
            host: "10.0.0.1".into(),
            port: 22,
            status: ServerStatus::Unknown,
            last_seen: None,
            metrics: None,
            os_info: None,
            tags: vec!["prod".into(), "web".into()],
            group: Some("Production".into()),
        };

        ctx.register_server(server);
        assert_eq!(ctx.list_servers().len(), 1);

        ctx.update_server_status(&ctx.list_servers()[0].id, ServerStatus::Online);
        assert_eq!(ctx.list_servers()[0].status, ServerStatus::Online);
    }

    #[test]
    fn test_conversation_flow() {
        let ctx = ContextEngine::new();

        let conv = ctx.create_conversation(None, ConversationType::Chat);
        assert_eq!(conv.messages.len(), 0);

        ctx.add_message(&conv.id, AiMessage {
            role: AiRole::User,
            content: "Show me running containers".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_calls: vec![],
            metadata: None,
        });

        ctx.add_message(&conv.id, AiMessage {
            role: AiRole::Assistant,
            content: "Running `docker ps` on production server...".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool_calls: vec![AiToolCall {
                id: "call_1".into(),
                tool: "docker_list_containers".into(),
                arguments: serde_json::json!({"all": false}),
                result: None,
            }],
            metadata: None,
        });

        let updated = ctx.get_conversation(&conv.id).unwrap();
        assert_eq!(updated.messages.len(), 2);
    }

    #[test]
    fn test_server_groups() {
        let ctx = ContextEngine::new();

        ctx.create_group(ServerGroup {
            id: Uuid::new_v4(),
            name: "Production Web".into(),
            environment: Environment::Production,
            servers: vec![],
            description: "Production web servers".into(),
            tags: vec!["prod".into(), "web".into()],
        });

        assert_eq!(ctx.list_groups().len(), 1);
    }
}
