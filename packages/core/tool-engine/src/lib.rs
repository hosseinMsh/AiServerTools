use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout")]
    Timeout,
}

// ─── Unified Tool Trait ────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> ToolCategory;

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, ToolError>;
    fn parameters_schema(&self) -> serde_json::Value;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCategory {
    Ssh,
    Docker,
    Sftp,
    Tunnel,
    System,
    Automation,
    Ai,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

impl ToolResult {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            stdout: None,
            stderr: None,
            exit_code: Some(0),
            duration_ms: 0,
            warnings: vec![],
        }
    }

    pub fn fail(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            stdout: None,
            stderr: Some(error.into()),
            exit_code: Some(1),
            duration_ms: 0,
            warnings: vec![],
        }
    }
}

// ─── Tool Execution Request ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub id: Uuid,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub session_id: Option<Uuid>,
    pub server_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResponse {
    pub id: Uuid,
    pub tool: String,
    pub result: ToolResult,
    pub executed_at: String,
}

// ─── Tool Registry ─────────────────────────────────────────────────────────────

pub struct ToolRegistry {
    tools: parking_lot::RwLock<std::collections::HashMap<String, Box<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn register(&self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().insert(name, tool);
    }

    pub fn execute(
        &self,
        request: ToolExecutionRequest,
    ) -> Result<ToolExecutionResponse, ToolError> {
        let tools = self.tools.read();
        let tool = tools
            .get(&request.tool)
            .ok_or_else(|| ToolError::ToolNotFound(request.tool.clone()))?;

        let start = std::time::Instant::now();
        let result = tool.execute(request.arguments)?;
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolExecutionResponse {
            id: request.id,
            tool: request.tool,
            result,
            executed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tools.read().keys().cloned().collect()
    }

    pub fn get_tool_metadata(&self, name: &str) -> Option<(String, String, ToolCategory)> {
        let tools = self.tools.read();
        tools
            .get(name)
            .map(|t| (t.name().into(), t.description().into(), t.category()))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
