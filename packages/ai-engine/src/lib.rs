use dashmap::DashMap;
use parking_lot::RwLock;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, debug, error};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("API request failed: {0}")]
    ApiRequestFailed(String),

    #[error("API response error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited")]
    RateLimited,

    #[error("Context too long: {current} > {max} tokens")]
    ContextTooLong { current: usize, max: usize },

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),
}

// ─── Provider Configuration ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub id: Uuid,
    pub name: String,
    pub provider_type: ProviderType,
    pub api_base_url: String,
    pub api_key: String, // Encrypted at rest
    pub models_endpoint: String,
    pub default_model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_secs: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    OpenAI,
    OpenAiCompatible,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: u32,
    pub capabilities: Vec<ModelCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelCapability {
    Chat,
    Completion,
    FunctionCalling,
    ToolUse,
    Vision,
    Streaming,
    CodeGeneration,
}

// ─── Chat / Message Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: AiRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AiRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<AiMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: AiMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ─── Tool System ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

// ─── Intention Classification ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserIntent {
    SshConnect,
    DockerExec,
    DockerCompose,
    SftpBrowse,
    TunnelCreate,
    SystemDiagnostics,
    FileEdit,
    ServerMonitor,
    Automation,
    GeneralHelp,
    Unknown,
}

impl UserIntent {
    pub fn description(&self) -> &str {
        match self {
            UserIntent::SshConnect => "SSH connection or remote command execution",
            UserIntent::DockerExec => "Docker container management",
            UserIntent::DockerCompose => "Docker Compose orchestration",
            UserIntent::SftpBrowse => "File browsing or transfer via SFTP",
            UserIntent::TunnelCreate => "SSH tunneling or port forwarding",
            UserIntent::SystemDiagnostics => "System diagnostics and debugging",
            UserIntent::FileEdit => "Remote file editing",
            UserIntent::ServerMonitor => "Server metrics and monitoring",
            UserIntent::Automation => "Automation script generation",
            UserIntent::GeneralHelp => "General assistance",
            UserIntent::Unknown => "Unknown intent",
        }
    }
}

// ─── AI Provider Client ────────────────────────────────────────────────────────

pub struct AiProvider {
    pub config: AiProviderConfig,
    http: HttpClient,
    models_cache: RwLock<Vec<ModelInfo>>,
}

impl AiProvider {
    pub fn new(config: AiProviderConfig) -> Result<Self, AiError> {
        let http = HttpClient::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs as u64))
            .build()
            .map_err(|e| AiError::InvalidConfig(e.to_string()))?;

        Ok(Self {
            config,
            http,
            models_cache: RwLock::new(Vec::new()),
        })
    }

    pub async fn fetch_models(&self) -> Result<Vec<ModelInfo>, AiError> {
        let url = format!("{}{}", self.config.api_base_url.trim_end_matches('/'), self.config.models_endpoint);
        debug!("Fetching models from: {}", url);

        let resp = self.http.get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|e| AiError::ApiRequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AiError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiError::ApiRequestFailed(e.to_string()))?;

        let models = match self.config.provider_type {
            ProviderType::OpenAI => {
                let data = body["data"].as_array().ok_or_else(|| {
                    AiError::ApiRequestFailed("Expected 'data' array in response".into())
                })?;
                data.iter().map(|m| ModelInfo {
                    id: m["id"].as_str().unwrap_or("").to_string(),
                    name: m["id"].as_str().unwrap_or("").to_string(),
                    provider: self.config.name.clone(),
                    context_length: m["context_length"].as_u64().unwrap_or(4096) as u32,
                    capabilities: vec![ModelCapability::Chat, ModelCapability::ToolUse],
                }).collect()
            }
            ProviderType::OpenAiCompatible | ProviderType::Custom => {
                // Try OpenAI format first, then flat array
                if let Some(data) = body["data"].as_array() {
                    data.iter().map(|m| ModelInfo {
                        id: m["id"].as_str().unwrap_or("").to_string(),
                        name: m["id"].as_str().unwrap_or("").to_string(),
                        provider: self.config.name.clone(),
                        context_length: m["context_length"].as_u64().unwrap_or(4096) as u32,
                        capabilities: vec![ModelCapability::Chat, ModelCapability::ToolUse],
                    }).collect()
                } else if let Some(data) = body.as_array() {
                    data.iter().map(|m| ModelInfo {
                        id: m["id"].as_str().or_else(|| m["name"].as_str()).unwrap_or("").to_string(),
                        name: m["id"].as_str().or_else(|| m["name"].as_str()).unwrap_or("").to_string(),
                        provider: self.config.name.clone(),
                        context_length: m.get("context_length").or_else(|| m.get("max_tokens"))
                            .and_then(|v| v.as_u64()).unwrap_or(4096) as u32,
                        capabilities: vec![ModelCapability::Chat, ModelCapability::ToolUse],
                    }).collect()
                } else {
                    vec![ModelInfo {
                        id: self.config.default_model.clone(),
                        name: self.config.default_model.clone(),
                        provider: self.config.name.clone(),
                        context_length: self.config.max_tokens,
                        capabilities: vec![ModelCapability::Chat],
                    }]
                }
            }
        };

        *self.models_cache.write() = models.clone();
        info!("Fetched {} models from {}", models.len(), self.config.name);
        Ok(models)
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let url = format!("{}/chat/completions", self.config.api_base_url.trim_end_matches('/'));
        debug!("Sending chat request to: {} (model: {})", url, request.model);

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "stream": request.stream,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(ref tools) = request.tools {
            body["tools"] = serde_json::to_value(tools).unwrap_or_default();
        }
        if let Some(ref tool_choice) = request.tool_choice {
            body["tool_choice"] = tool_choice.clone();
        }

        let resp = self.http.post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::ApiRequestFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            if status == 429 {
                return Err(AiError::RateLimited);
            }
            return Err(AiError::ApiError { status, message: text });
        }

        resp.json().await.map_err(|e| AiError::ApiRequestFailed(e.to_string()))
    }

    pub fn get_models(&self) -> Vec<ModelInfo> {
        self.models_cache.read().clone()
    }

    pub fn is_available(&self) -> bool {
        self.config.enabled
    }
}

// ─── Tool Registry (AI Tools for LLM to call) ─────────────────────────────────

pub struct AiToolRegistry {
    tools: RwLock<Vec<ToolDefinition>>,
}

impl AiToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: RwLock::new(Vec::new()) };
        registry.register_default_tools();
        registry
    }

    pub fn register(&self, tool: ToolDefinition) {
        self.tools.write().push(tool);
    }

    pub fn get_tools(&self) -> Vec<ToolDefinition> {
        self.tools.read().clone()
    }

    fn register_default_tools(&self) {
        let default_tools = vec![
            ToolDefinition {
                r#type: "function".into(),
                function: ToolFunction {
                    name: "ssh_exec".into(),
                    description: "Execute a command on a remote server via SSH".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "server_id": {"type": "string", "description": "Target server ID"},
                            "command": {"type": "string", "description": "Command to execute"},
                            "timeout_secs": {"type": "number", "description": "Command timeout"}
                        },
                        "required": ["server_id", "command"]
                    }),
                },
            },
            ToolDefinition {
                r#type: "function".into(),
                function: ToolFunction {
                    name: "docker_list_containers".into(),
                    description: "List Docker containers on a server".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "server_id": {"type": "string", "description": "Target server ID"},
                            "all": {"type": "boolean", "description": "Include stopped containers"}
                        },
                        "required": ["server_id"]
                    }),
                },
            },
            ToolDefinition {
                r#type: "function".into(),
                function: ToolFunction {
                    name: "docker_container_action".into(),
                    description: "Start, stop, or restart a Docker container".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "server_id": {"type": "string"},
                            "container_id": {"type": "string"},
                            "action": {"type": "string", "enum": ["start", "stop", "restart"]}
                        },
                        "required": ["server_id", "container_id", "action"]
                    }),
                },
            },
            ToolDefinition {
                r#type: "function".into(),
                function: ToolFunction {
                    name: "get_server_metrics".into(),
                    description: "Get CPU, RAM, disk, and network metrics from a server".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "server_id": {"type": "string"}
                        },
                        "required": ["server_id"]
                    }),
                },
            },
            ToolDefinition {
                r#type: "function".into(),
                function: ToolFunction {
                    name: "sftp_list_directory".into(),
                    description: "List files in a remote directory via SFTP".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "server_id": {"type": "string"},
                            "path": {"type": "string", "description": "Remote directory path"}
                        },
                        "required": ["server_id", "path"]
                    }),
                },
            },
            ToolDefinition {
                r#type: "function".into(),
                function: ToolFunction {
                    name: "view_logs".into(),
                    description: "View logs from a server or Docker container".into(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "server_id": {"type": "string"},
                            "source": {"type": "string", "enum": ["system", "docker", "file"]},
                            "identifier": {"type": "string", "description": "Container ID, file path, or log name"},
                            "lines": {"type": "number", "description": "Number of lines to show"}
                        },
                        "required": ["server_id", "source"]
                    }),
                },
            },
        ];

        for tool in default_tools {
            self.tools.write().push(tool);
        }
    }
}

impl Default for AiToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Intent Classifier ─────────────────────────────────────────────────────────

pub struct IntentClassifier;

impl IntentClassifier {
    pub fn classify(input: &str) -> UserIntent {
        let lower = input.to_lowercase();

        if lower.contains("ssh") && (lower.contains("connect") || lower.contains("login") || lower.contains("access")) {
            return UserIntent::SshConnect;
        }
        if lower.contains("docker") {
            if lower.contains("compose") || lower.contains("up") || lower.contains("down") {
                return UserIntent::DockerCompose;
            }
            if lower.contains("container") || lower.contains("ps") || lower.contains("logs") {
                return UserIntent::DockerExec;
            }
            return UserIntent::DockerExec;
        }
        if lower.contains("file") || lower.contains("upload") || lower.contains("download") || lower.contains("sftp") {
            return UserIntent::SftpBrowse;
        }
        if lower.contains("tunnel") || lower.contains("proxy") || lower.contains("forward") || lower.contains("socks") {
            return UserIntent::TunnelCreate;
        }
        if lower.contains("diagnos") || lower.contains("debug") || lower.contains("error") || lower.contains("fail") {
            return UserIntent::SystemDiagnostics;
        }
        if lower.contains("edit") || lower.contains("change") || lower.contains("modify") {
            return UserIntent::FileEdit;
        }
        if lower.contains("cpu") || lower.contains("memory") || lower.contains("disk") || lower.contains("monitor") || lower.contains("metric") {
            return UserIntent::ServerMonitor;
        }
        if lower.contains("script") || lower.contains("automate") || lower.contains("deploy") {
            return UserIntent::Automation;
        }

        UserIntent::Unknown
    }
}

// ─── AI Engine (Orchestrator) ──────────────────────────────────────────────────

pub struct AiEngine {
    providers: Arc<DashMap<Uuid, Arc<AiProvider>>>,
    active_provider_id: RwLock<Option<Uuid>>,
    tool_registry: Arc<AiToolRegistry>,
}

impl AiEngine {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(DashMap::new()),
            active_provider_id: RwLock::new(None),
            tool_registry: Arc::new(AiToolRegistry::new()),
        }
    }

    pub fn add_provider(&self, config: AiProviderConfig) -> Result<Uuid, AiError> {
        let id = config.id;
        let provider = Arc::new(AiProvider::new(config)?);
        self.providers.insert(id, provider);
        info!("AI provider added: {}", id);

        if self.active_provider_id.read().is_none() {
            *self.active_provider_id.write() = Some(id);
        }

        Ok(id)
    }

    pub fn remove_provider(&self, id: &Uuid) {
        self.providers.remove(id);
        if *self.active_provider_id.read() == Some(*id) {
            *self.active_provider_id.write() = self.providers.iter().next().map(|e| *e.key());
        }
    }

    pub fn set_active_provider(&self, id: &Uuid) -> Result<(), AiError> {
        if self.providers.contains_key(id) {
            *self.active_provider_id.write() = Some(*id);
            info!("Active AI provider set to: {}", id);
            Ok(())
        } else {
            Err(AiError::ProviderNotFound(id.to_string()))
        }
    }

    pub fn get_active_provider(&self) -> Option<Arc<AiProvider>> {
        let id = self.active_provider_id.read().clone()?;
        self.providers.get(&id).map(|p| p.clone())
    }

    pub fn list_providers(&self) -> Vec<AiProviderConfig> {
        self.providers.iter().map(|e| e.value().config.clone()).collect()
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let provider = self.get_active_provider()
            .ok_or_else(|| AiError::ProviderNotFound("No active provider".into()))?;

        // TODO: Add model fallback, retry logic, context trimming
        provider.chat(request).await
    }

    pub async fn fetch_models(&self, provider_id: &Uuid) -> Result<Vec<ModelInfo>, AiError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| AiError::ProviderNotFound(provider_id.to_string()))?;
        provider.fetch_models().await
    }

    pub fn get_tools(&self) -> Vec<ToolDefinition> {
        self.tool_registry.get_tools()
    }

    pub fn classify_intent(&self, input: &str) -> UserIntent {
        IntentClassifier::classify(input)
    }

    /// Build a system prompt that adapts to DevOps context
    pub fn build_system_prompt(&self, server_context: Option<&str>, user_role: &str) -> String {
        format!(
            "You are InfraCore AI, an Enterprise Infrastructure & DevOps Copilot embedded inside a secure cross-platform SSH/Server Management application.\n\n\
            You are NOT a chatbot. You are a mission-critical operations assistant for managing servers, containers, secure connections, and infrastructure systems.\n\n\
            ## CORE RULES\n\
            1. NEVER assume server state — always suggest running verification commands\n\
            2. NEVER expose secrets or private keys\n\
            3. ALWAYS warn before destructive operations (rm -rf, docker rm -f, reboot, etc.)\n\
            4. Prefer safe diagnostic commands first (uptime, free -m, df -h, docker ps)\n\
            5. When uncertain, ask the user for logs or command output before proceeding\n\
            6. Format commands in code blocks with their explanation\n\
            7. Keep responses concise and execution-oriented\n\n\
            ## USER CONTEXT\n\
            - Role: {}\n\
            - Server context: {}\n\n\
            ## TOOLS AVAILABLE\n\
            You can request the system to execute: SSH commands, Docker operations, SFTP file operations, and tunnel management.\n\
            Always explain what a command will do before executing it.\n\
            For destructive actions, clearly state the risk and get user confirmation.",
            user_role,
            server_context.unwrap_or("No active server")
        )
    }
}

impl Default for AiEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_classifier() {
        assert_eq!(IntentClassifier::classify("SSH into prod-web-01"), UserIntent::SshConnect);
        assert_eq!(IntentClassifier::classify("Show me docker containers"), UserIntent::DockerExec);
        assert_eq!(IntentClassifier::classify("Run docker-compose up -d"), UserIntent::DockerCompose);
        assert_eq!(IntentClassifier::classify("Upload file to /etc/nginx"), UserIntent::SftpBrowse);
        assert_eq!(IntentClassifier::classify("Create SOCKS tunnel on 1080"), UserIntent::TunnelCreate);
        assert_eq!(IntentClassifier::classify("Debug why nginx is failing"), UserIntent::SystemDiagnostics);
        assert_eq!(IntentClassifier::classify("Check CPU and memory usage"), UserIntent::ServerMonitor);
    }

    #[test]
    fn test_provider_config_serialization() {
        let config = AiProviderConfig {
            id: Uuid::new_v4(),
            name: "My OpenAI".into(),
            provider_type: ProviderType::OpenAI,
            api_base_url: "https://api.openai.com/v1".into(),
            api_key: "encrypted_key_here".into(),
            models_endpoint: "/models".into(),
            default_model: "gpt-4".into(),
            max_tokens: 4096,
            temperature: 0.3,
            timeout_secs: 60,
            enabled: true,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("OpenAI"));

        let deserialized: AiProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.default_model, "gpt-4");
    }

    #[test]
    fn test_tool_definition() {
        let tool = ToolDefinition {
            r#type: "function".into(),
            function: ToolFunction {
                name: "ssh_exec".into(),
                description: "Execute command over SSH".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string"}
                    },
                    "required": ["command"]
                }),
            },
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("ssh_exec"));
        let deserialized: ToolDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.function.name, "ssh_exec");
    }

    #[test]
    fn test_tool_registry() {
        let registry = AiToolRegistry::new();
        let tools = registry.get_tools();
        assert!(tools.len() >= 5);
        assert!(tools.iter().any(|t| t.function.name == "ssh_exec"));
        assert!(tools.iter().any(|t| t.function.name == "docker_list_containers"));
    }

    #[test]
    fn test_system_prompt_generation() {
        let engine = AiEngine::new();
        let prompt = engine.build_system_prompt(Some("prod-web-01 (10.0.0.1)"), "admin");
        assert!(prompt.contains("InfraCore AI"));
        assert!(prompt.contains("admin"));
        assert!(prompt.contains("prod-web-01"));
    }
}
