// ─── InfraCore Desktop - Tauri Backend ─────────────────────────────────────────
// Enterprise SSH & Infrastructure Control Platform

use std::sync::Arc;
use tauri::Manager;
use tracing::info;
use uuid::Uuid;

// ─── Application State ────────────────────────────────────────────────────────

pub struct AppState {
    pub ssh: ssh_engine::SshManager,
    pub docker: docker_engine::DockerManager,
    pub sftp: sftp_engine::SftpManager,
    pub tunnel: tunnel_engine::TunnelManager,
    pub security: security::SecurityManager,
    pub audit: audit::AuditLogger,
    pub context: context_engine::ContextEngine,
    pub ai: ai_engine::AiEngine,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ssh: ssh_engine::SshManager::new(),
            docker: docker_engine::DockerManager::new(),
            sftp: sftp_engine::SftpManager::new(),
            tunnel: tunnel_engine::TunnelManager::new(),
            security: security::SecurityManager::new(),
            audit: audit::AuditLogger::new(10_000),
            context: context_engine::ContextEngine::new(),
            ai: ai_engine::AiEngine::new(),
        }
    }
}

// ─── Tauri Commands ────────────────────────────────────────────────────────────

// ── SSH Commands ──────────────────────────────────────────────────────────

#[tauri::command]
async fn ssh_connect(
    state: tauri::State<'_, AppState>,
    config: ssh_engine::SshConnectionConfig,
) -> Result<ssh_engine::SshSessionHandle, String> {
    info!("SSH connect: {}@{}:{}", config.auth.username, config.host, config.port);
    state.ssh.connect(config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_exec(
    state: tauri::State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<ssh_engine::CommandResult, String> {
    let id = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let session = state.ssh.get_session(&id).ok_or("Session not found")?;
    let result = session.execute_command(&command).await.map_err(|e| e.to_string())?;

    // Audit log
    state.audit.log(audit::AuditEvent {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: audit::AuditEventType::SshCommand,
        user_id: None,
        session_id: Some(id),
        server_id: None,
        action: "ssh_exec".into(),
        resource: command.clone(),
        details: serde_json::json!({"command": command, "exit_code": result.exit_code}),
        result: if result.exit_code == Some(0) { audit::AuditResult::Success } else { audit::AuditResult::Failure("Non-zero exit".into()) },
        ip_address: None,
        user_agent: None,
    });

    Ok(result)
}

#[tauri::command]
async fn ssh_disconnect(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    state.ssh.disconnect(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ssh_list_sessions(state: tauri::State<'_, AppState>) -> Vec<ssh_engine::SshSessionHandle> {
    state.ssh.list_sessions()
}

// ── Docker Commands ───────────────────────────────────────────────────────

#[tauri::command]
async fn docker_list_containers(
    config: docker_engine::DockerConnectionConfig,
    all: bool,
) -> Result<Vec<docker_engine::ContainerSummary>, String> {
    let client = docker_engine::DockerClient::new(config).map_err(|e| e.to_string())?;
    client.list_containers(all).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn docker_container_action(
    config: docker_engine::DockerConnectionConfig,
    container_id: String,
    action: String,
) -> Result<(), String> {
    let client = docker_engine::DockerClient::new(config).map_err(|e| e.to_string())?;
    match action.as_str() {
        "start" => client.start_container(&container_id).await.map_err(|e| e.to_string()),
        "stop" => client.stop_container(&container_id, None).await.map_err(|e| e.to_string()),
        "restart" => client.restart_container(&container_id, None).await.map_err(|e| e.to_string()),
        _ => Err(format!("Unknown action: {}", action)),
    }
}

#[tauri::command]
async fn docker_container_logs(
    config: docker_engine::DockerConnectionConfig,
    container_id: String,
    tail: Option<u32>,
) -> Result<String, String> {
    let client = docker_engine::DockerClient::new(config).map_err(|e| e.to_string())?;
    let opts = docker_engine::ContainerLogOptions {
        tail,
        follow: false,
        stdout: true,
        stderr: true,
        timestamps: false,
        since: None,
        until: None,
    };
    client.container_logs(&container_id, opts).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn docker_container_stats(
    config: docker_engine::DockerConnectionConfig,
    container_id: String,
) -> Result<docker_engine::ContainerStats, String> {
    let client = docker_engine::DockerClient::new(config).map_err(|e| e.to_string())?;
    client.container_stats(&container_id).await.map_err(|e| e.to_string())
}

// ── Tunnel Commands ───────────────────────────────────────────────────────

#[tauri::command]
async fn tunnel_start_socks(
    state: tauri::State<'_, AppState>,
    session_id: String,
    bind_port: u16,
    bind_addr: Option<String>,
) -> Result<tunnel_engine::TunnelStatus, String> {
    let config = tunnel_engine::TunnelConfig {
        id: Uuid::new_v4(),
        label: format!("SOCKS5 :{}", bind_port),
        session_id: Uuid::parse_str(&session_id).map_err(|e| e.to_string())?,
        kind: tunnel_engine::TunnelKind::Dynamic,
        bind_addr: bind_addr.unwrap_or_else(|| "127.0.0.1".into()),
        bind_port,
        target_host: None,
        target_port: None,
        enabled: true,
    };
    let instance = state.tunnel.start_tunnel(config).await.map_err(|e| e.to_string())?;
    Ok(instance.status())
}

#[tauri::command]
async fn tunnel_start_local(
    state: tauri::State<'_, AppState>,
    session_id: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<tunnel_engine::TunnelStatus, String> {
    let config = tunnel_engine::TunnelConfig {
        id: Uuid::new_v4(),
        label: format!(":{} -> {}:{}", bind_port, target_host, target_port),
        session_id: Uuid::parse_str(&session_id).map_err(|e| e.to_string())?,
        kind: tunnel_engine::TunnelKind::Local,
        bind_addr: "127.0.0.1".into(),
        bind_port,
        target_host: Some(target_host),
        target_port: Some(target_port),
        enabled: true,
    };
    let instance = state.tunnel.start_tunnel(config).await.map_err(|e| e.to_string())?;
    Ok(instance.status())
}

#[tauri::command]
async fn tunnel_stop(
    state: tauri::State<'_, AppState>,
    tunnel_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&tunnel_id).map_err(|e| e.to_string())?;
    state.tunnel.stop_tunnel(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn tunnel_list(state: tauri::State<'_, AppState>) -> Vec<tunnel_engine::TunnelStatus> {
    state.tunnel.list_tunnels()
}

// ── Security Vault Commands ───────────────────────────────────────────────

#[tauri::command]
async fn vault_unlock(
    state: tauri::State<'_, AppState>,
    master_password: String,
) -> Result<(), String> {
    state.security.unlock(&master_password).map_err(|e| e.to_string())
}

#[tauri::command]
async fn vault_store(
    state: tauri::State<'_, AppState>,
    label: String,
    value: String,
    kind: String,
) -> Result<String, String> {
    let cred_kind = match kind.as_str() {
        "ssh_key" => security::CredentialKind::SshKey,
        "password" => security::CredentialKind::Password,
        "api_key" => security::CredentialKind::ApiKey,
        "token" => security::CredentialKind::Token,
        _ => security::CredentialKind::Other(kind),
    };
    let entry = state.security.vault.store(&label, cred_kind, &value, vec![])
        .map_err(|e| e.to_string())?;
    Ok(entry.id.to_string())
}

#[tauri::command]
async fn vault_retrieve(
    state: tauri::State<'_, AppState>,
    entry_id: String,
) -> Result<String, String> {
    let id = Uuid::parse_str(&entry_id).map_err(|e| e.to_string())?;
    state.security.vault.retrieve(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn vault_list(state: tauri::State<'_, AppState>) -> Vec<security::VaultEntry> {
    state.security.vault.list()
}

// ── AI Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn ai_add_provider(
    state: tauri::State<'_, AppState>,
    config: ai_engine::AiProviderConfig,
) -> Result<String, String> {
    let id = state.ai.add_provider(config).map_err(|e| e.to_string())?;
    Ok(id.to_string())
}

#[tauri::command]
async fn ai_chat(
    state: tauri::State<'_, AppState>,
    request: ai_engine::ChatRequest,
) -> Result<ai_engine::ChatResponse, String> {
    state.ai.chat(request).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ai_fetch_models(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<Vec<ai_engine::ModelInfo>, String> {
    let id = Uuid::parse_str(&provider_id).map_err(|e| e.to_string())?;
    state.ai.fetch_models(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ai_classify_intent(
    state: tauri::State<'_, AppState>,
    input: String,
) -> ai_engine::UserIntent {
    state.ai.classify_intent(&input)
}

#[tauri::command]
async fn ai_get_tools(state: tauri::State<'_, AppState>) -> Vec<ai_engine::ToolDefinition> {
    state.ai.get_tools()
}

// ── Audit Commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn audit_get_recent(
    state: tauri::State<'_, AppState>,
    count: usize,
) -> Vec<audit::AuditEvent> {
    state.audit.get_recent_events(count)
}

// ── Server / Context Commands ─────────────────────────────────────────────

#[tauri::command]
async fn context_register_server(
    state: tauri::State<'_, AppState>,
    server: context_engine::ServerState,
) {
    state.context.register_server(server);
}

#[tauri::command]
async fn context_list_servers(state: tauri::State<'_, AppState>) -> Vec<context_engine::ServerState> {
    state.context.list_servers()
}

#[tauri::command]
async fn context_get_server(
    state: tauri::State<'_, AppState>,
    server_id: String,
) -> Option<context_engine::ServerState> {
    let id = Uuid::parse_str(&server_id).ok()?;
    state.context.get_server(&id)
}

// ─── Main Entry Point ─────────────────────────────────────────────────────────

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .init();

    info!("Starting InfraCore Desktop v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // SSH
            ssh_connect,
            ssh_exec,
            ssh_disconnect,
            ssh_list_sessions,
            // Docker
            docker_list_containers,
            docker_container_action,
            docker_container_logs,
            docker_container_stats,
            // Tunnel
            tunnel_start_socks,
            tunnel_start_local,
            tunnel_stop,
            tunnel_list,
            // Security
            vault_unlock,
            vault_store,
            vault_retrieve,
            vault_list,
            // AI
            ai_add_provider,
            ai_chat,
            ai_fetch_models,
            ai_classify_intent,
            ai_get_tools,
            // Audit
            audit_get_recent,
            // Context
            context_register_server,
            context_list_servers,
            context_get_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running InfraCore Desktop");
}
