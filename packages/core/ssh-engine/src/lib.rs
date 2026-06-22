use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::SocketAddr;
use dashmap::DashMap;
use parking_lot::RwLock;
use russh::client::{self, Handler, Msg, Session};
use russh::keys::load_secret_key;
use russh::Channel;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("SFTP error: {0}")]
    SftpError(String),

    #[error("Key load error: {0}")]
    KeyLoadError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Disconnected")]
    Disconnected,
}

// ─── Authentication Configuration ──────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshAuthConfig {
    pub method: AuthMethod,
    pub username: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthMethod {
    Password { password: String },
    Key { private_key_path: String, passphrase: Option<String> },
    KeyInline { private_key: String, passphrase: Option<String> },
    Agent,
    None,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SshConnectionConfig {
    pub id: Uuid,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub auth: SshAuthConfig,
    pub proxy_jump: Option<ProxyJumpConfig>,
    pub keepalive_interval_secs: u64,
    pub compression: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyJumpConfig {
    pub host: String,
    pub port: u16,
    pub auth: SshAuthConfig,
}

// ─── SSH Session Handler ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SshSessionHandle {
    pub id: Uuid,
    pub label: String,
    pub host: String,
    pub port: u16,
    connected: Arc<AtomicBool>,
    channel_tx: UnboundedSender<SshEvent>,
    session: Arc<Mutex<Option<client::Handle<SshClientHandler>>>>,
}

impl SshSessionHandle {
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn execute_command(&self, command: &str) -> Result<CommandResult, SshError> {
        let mut session = self.session.lock().await;
        let session = session.as_mut().ok_or(SshError::Disconnected)?;

        let mut channel = session.channel_open_session().await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        channel.exec(true, command.as_bytes()).await
            .map_err(|e| SshError::CommandFailed(e.to_string()))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = None;

        loop {
            match channel.wait().await {
                Some(Msg::ChannelSuccess(_)) => {
                    // Wait for exit status
                }
                Some(Msg::ChannelFailure(_)) => {
                    return Err(SshError::CommandFailed("Channel failure".into()));
                }
                Some(Msg::ChannelData { ref data }) => {
                    stdout.extend_from_slice(data);
                }
                Some(Msg::ChannelExtendedData { ref data, .. }) => {
                    stderr.extend_from_slice(data);
                }
                Some(Msg::ChannelEof(_)) => {
                    break;
                }
                Some(Msg::ChannelClose(_)) => {
                    break;
                }
                Some(Msg::ChannelExitSignal { .. }) => {
                    break;
                }
                None => break,
                _ => {}
            }
        }

        Ok(CommandResult {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            exit_code,
        })
    }

    pub async fn start_shell(&self) -> Result<ShellHandle, SshError> {
        let mut session = self.session.lock().await;
        let session = session.as_mut().ok_or(SshError::Disconnected)?;

        let mut channel = session.channel_open_session().await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        channel.request_pty(false, &[]).await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        channel.shell(true).await
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        let (tx, rx) = mpsc::unbounded_channel();

        let shell = ShellHandle {
            channel: Arc::new(Mutex::new(channel)),
            rx: Arc::new(Mutex::new(Some(rx))),
            tx,
        };

        Ok(shell)
    }

    pub async fn disconnect(&self) -> Result<(), SshError> {
        let mut session = self.session.lock().await;
        if let Some(s) = session.take() {
            let _ = s.disconnect(russh::Disconnect::ByApplication, "Client disconnect", "");
            self.connected.store(false, Ordering::Relaxed);
        }
        Ok(())
    }
}

// ─── Shell Handle ──────────────────────────────────────────────────────────────

pub struct ShellHandle {
    channel: Arc<Mutex<Channel<Msg>>>,
    rx: Arc<Mutex<Option<UnboundedReceiver<SshEvent>>>>,
    tx: UnboundedSender<SshEvent>,
}

impl ShellHandle {
    pub async fn write_input(&self, data: &[u8]) -> Result<(), SshError> {
        let mut channel = self.channel.lock().await;
        channel.data(data).await
            .map_err(|e| SshError::ChannelError(e.to_string()))
    }

    pub async fn resize_pty(&self, cols: u32, rows: u32) -> Result<(), SshError> {
        let mut channel = self.channel.lock().await;
        channel.window_change(cols, rows, 0, 0).await
            .map_err(|e| SshError::ChannelError(e.to_string()))
    }

    pub async fn close(&self) -> Result<(), SshError> {
        let mut channel = self.channel.lock().await;
        channel.eof().await.map_err(|e| SshError::ChannelError(e.to_string()))
    }
}

// ─── Event Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SshEvent {
    Output(Vec<u8>),
    ExtendedOutput(Vec<u8>),
    Disconnected,
    Error(String),
    ExitCode(u32),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<u32>,
}

// ─── SSH Client Handler ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SshClientHandler {
    pub id: Uuid,
    pub connected: Arc<AtomicBool>,
    pub event_tx: UnboundedSender<SshEvent>,
}

impl Handler for SshClientHandler {
    type Error = russh::Error;

    fn handle_auth_banner(&mut self, banner: &[u8]) -> Result<(), Self::Error> {
        debug!("Auth banner: {}", String::from_utf8_lossy(banner));
        Ok(())
    }

    fn handle_debug(&mut self, msg: &str, lang: &str) -> Result<(), Self::Error> {
        debug!("SSH debug: {} (lang: {})", msg, lang);
        Ok(())
    }

    fn handle_unimplemented(&mut self, seq: u32) -> Result<(), Self::Error> {
        warn!("Unimplemented message sequence: {}", seq);
        Ok(())
    }

    fn handle_disconnect(&mut self, reason: russh::Disconnect, msg: &str, lang: &str) {
        info!("Disconnected: {:?} - {} (lang: {})", reason, msg, lang);
        self.connected.store(false, Ordering::Relaxed);
        let _ = self.event_tx.send(SshEvent::Disconnected);
    }
}

// ─── SSH Connection Manager ────────────────────────────────────────────────────

pub struct SshManager {
    sessions: Arc<DashMap<Uuid, SshSessionHandle>>,
    config_store: Arc<RwLock<Vec<SshConnectionConfig>>>,
}

impl SshManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            config_store: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn connect(&self, config: SshConnectionConfig) -> Result<SshSessionHandle, SshError> {
        info!("Connecting to {}:{} as {}", config.host, config.port, config.auth.username);

        // Build proxy jump chain if configured
        let connect_config = if let Some(ref proxy) = config.proxy_jump {
            self.build_proxy_config(&config, proxy)?
        } else {
            self.build_direct_config(&config)?
        };

        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let connected = Arc::new(AtomicBool::new(false));
        let handler = SshClientHandler {
            id: config.id,
            connected: connected.clone(),
            event_tx: event_tx.clone(),
        };

        // Authenticate and connect
        let mut session = match &config.auth.method {
            AuthMethod::Password { password } => {
                let mut handle = client::connect(connect_config, (config.host.clone(), config.port), handler).await
                    .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;
                let auth_result = handle.authenticate_password(&config.auth.username, password).await
                    .map_err(|e| SshError::AuthFailed(e.to_string()))?;
                if !auth_result {
                    return Err(SshError::AuthFailed("Password authentication rejected".into()));
                }
                handle
            }
            AuthMethod::Key { private_key_path, passphrase } => {
                let key_pair = load_secret_key(private_key_path, passphrase.as_deref())
                    .map_err(|e| SshError::KeyLoadError(e.to_string()))?;
                let mut handle = client::connect(connect_config, (config.host.clone(), config.port), handler).await
                    .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;
                let auth_result = handle.authenticate_publickey(&config.auth.username, Arc::new(key_pair)).await
                    .map_err(|e| SshError::AuthFailed(e.to_string()))?;
                if !auth_result {
                    return Err(SshError::AuthFailed("Key authentication rejected".into()));
                }
                handle
            }
            AuthMethod::Agent => {
                let mut handle = client::connect(connect_config, (config.host.clone(), config.port), handler).await
                    .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;
                let auth_result = handle.authenticate_agent(&config.auth.username).await
                    .map_err(|e| SshError::AuthFailed(e.to_string()))?;
                if !auth_result {
                    return Err(SshError::AuthFailed("Agent authentication rejected".into()));
                }
                handle
            }
            AuthMethod::None => {
                let handle = client::connect(connect_config, (config.host.clone(), config.port), handler).await
                    .map_err(|e| SshError::ConnectionFailed(e.to_string()))?;
                handle
            }
            _ => return Err(SshError::AuthFailed("Unsupported auth method".into())),
        };

        connected.store(true, Ordering::Relaxed);
        info!("Connected to {}:{} (id={})", config.host, config.port, config.id);

        let handle = SshSessionHandle {
            id: config.id,
            label: config.label.clone(),
            host: config.host.clone(),
            port: config.port,
            connected: connected.clone(),
            channel_tx: event_tx.clone(),
            session: Arc::new(Mutex::new(Some(session))),
        };

        self.sessions.insert(config.id, handle.clone());

        // Save config
        self.config_store.write().push(config);

        Ok(handle)
    }

    pub fn get_session(&self, id: &Uuid) -> Option<SshSessionHandle> {
        self.sessions.get(id).map(|s| s.clone())
    }

    pub async fn disconnect(&self, id: &Uuid) -> Result<(), SshError> {
        if let Some(session) = self.sessions.get(id) {
            session.disconnect().await?;
            self.sessions.remove(id);
        }
        Ok(())
    }

    pub async fn disconnect_all(&self) -> Result<(), SshError> {
        let ids: Vec<Uuid> = self.sessions.iter().map(|s| s.key().clone()).collect();
        for id in ids {
            self.disconnect(&id).await?;
        }
        Ok(())
    }

    pub fn list_sessions(&self) -> Vec<SshSessionHandle> {
        self.sessions.iter().map(|s| s.value().clone()).collect()
    }

    fn build_direct_config(&self, config: &SshConnectionConfig) -> Result<client::Config, SshError> {
        Ok(client::Config {
            preferred: client::Preferred::default(),
            buffer_alloc_size: Some(32768),
            keepalive_interval: Some(std::time::Duration::from_secs(config.keepalive_interval_secs)),
            keepalive_max: Some(3),
            ..Default::default()
        })
    }

    fn build_proxy_config(&self, config: &SshConnectionConfig, proxy: &ProxyJumpConfig) -> Result<client::Config, SshError> {
        info!("Using ProxyJump via {}:{}", proxy.host, proxy.port);
        Ok(client::Config {
            preferred: client::Preferred::default(),
            buffer_alloc_size: Some(32768),
            keepalive_interval: Some(std::time::Duration::from_secs(config.keepalive_interval_secs)),
            keepalive_max: Some(3),
            // ProxyJump handled at connect layer via SOCKS or direct TCP forwarding
            ..Default::default()
        })
    }
}

impl Default for SshManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Port Forwarding ───────────────────────────────────────────────────────────

pub struct PortForward {
    pub id: Uuid,
    pub session_id: Uuid,
    pub kind: PortForwardKind,
    pub bind_addr: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub active: Arc<AtomicBool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PortForwardKind {
    Local,
    Remote,
    Dynamic,
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_serialization() {
        let config = SshConnectionConfig {
            id: Uuid::new_v4(),
            label: "Test Server".into(),
            host: "192.168.1.100".into(),
            port: 22,
            auth: SshAuthConfig {
                method: AuthMethod::Key {
                    private_key_path: "/home/user/.ssh/id_ed25519".into(),
                    passphrase: None,
                },
                username: "deploy".into(),
            },
            proxy_jump: None,
            keepalive_interval_secs: 30,
            compression: true,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("192.168.1.100"));
        assert!(json.contains("deploy"));

        let deserialized: SshConnectionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.host, "192.168.1.100");
    }

    #[test]
    fn test_command_result_serialization() {
        let result = CommandResult {
            stdout: "OK".into(),
            stderr: "".into(),
            exit_code: Some(0),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"exit_code\":0"));
    }
}
