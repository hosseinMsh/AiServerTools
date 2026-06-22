use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("Bind failed on {addr}: {details}")]
    BindFailed { addr: String, details: String },

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Tunnel not found: {0}")]
    TunnelNotFound(Uuid),

    #[error("IO error: {0}")]
    IoError(String),
}

// ─── Tunnel Configuration ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub id: Uuid,
    pub label: String,
    pub session_id: Uuid,
    pub kind: TunnelKind,
    pub bind_addr: String,
    pub bind_port: u16,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TunnelKind {
    /// SOCKS5 dynamic proxy (-D)
    Dynamic,
    /// Local port forwarding (-L)
    Local,
    /// Remote port forwarding (-R)
    Remote,
}

impl TunnelKind {
    pub fn as_flag(&self) -> &str {
        match self {
            TunnelKind::Dynamic => "-D",
            TunnelKind::Local => "-L",
            TunnelKind::Remote => "-R",
        }
    }
}

// ─── Tunnel Status ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatus {
    pub id: Uuid,
    pub kind: TunnelKind,
    pub bind_addr: String,
    pub bind_port: u16,
    pub target: String,
    pub active: bool,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub connected_since: Option<String>,
    pub error: Option<String>,
}

// ─── Tunnel Instance ───────────────────────────────────────────────────────────

pub struct TunnelInstance {
    pub config: TunnelConfig,
    pub active: Arc<AtomicBool>,
    pub status: Arc<RwLock<TunnelStatus>>,
    shutdown_token: Arc<tokio::sync::watch::Sender<bool>>,
    join_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TunnelInstance {
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn status(&self) -> TunnelStatus {
        self.status.read().clone()
    }

    pub async fn stop(&self) -> Result<(), TunnelError> {
        self.active.store(false, Ordering::Relaxed);
        let _ = self.shutdown_token.send(true);

        let mut handle = self.join_handle.lock().await;
        if let Some(h) = handle.take() {
            let _ = h.await;
        }

        info!("Tunnel {} stopped", self.config.id);
        Ok(())
    }
}

// ─── SOCKS5 Proxy Implementation ──────────────────────────────────────────────

async fn handle_socks5_connection(
    mut inbound: TcpStream,
    _session_id: Uuid,
) -> Result<(), TunnelError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // SOCKS5 handshake
    let mut buf = [0u8; 2];
    inbound.read_exact(&mut buf).await
        .map_err(|e| TunnelError::IoError(e.to_string()))?;

    let n_methods = buf[1] as usize;
    let mut methods = vec![0u8; n_methods];
    inbound.read_exact(&mut methods).await
        .map_err(|e| TunnelError::IoError(e.to_string()))?;

    // Reply: no authentication required (0x00)
    inbound.write_all(&[0x05, 0x00]).await
        .map_err(|e| TunnelError::IoError(e.to_string()))?;

    // Read request
    let mut header = [0u8; 4];
    inbound.read_exact(&mut header).await
        .map_err(|e| TunnelError::IoError(e.to_string()))?;

    let atyp = header[3];
    let target_addr = match atyp {
        0x01 => { // IPv4
            let mut addr = [0u8; 4];
            inbound.read_exact(&mut addr).await
                .map_err(|e| TunnelError::IoError(e.to_string()))?;
            SocketAddr::from((addr, 0)).ip().to_string()
        }
        0x03 => { // Domain name
            let mut len = [0u8; 1];
            inbound.read_exact(&mut len).await
                .map_err(|e| TunnelError::IoError(e.to_string()))?;
            let mut domain = vec![0u8; len[0] as usize];
            inbound.read_exact(&mut domain).await
                .map_err(|e| TunnelError::IoError(e.to_string()))?;
            String::from_utf8_lossy(&domain).to_string()
        }
        0x04 => { // IPv6
            let mut addr = [0u8; 16];
            inbound.read_exact(&mut addr).await
                .map_err(|e| TunnelError::IoError(e.to_string()))?;
            SocketAddr::from((addr, 0)).ip().to_string()
        }
        _ => return Err(TunnelError::ConnectionFailed("Unsupported address type".into())),
    };

    let mut port_buf = [0u8; 2];
    inbound.read_exact(&mut port_buf).await
        .map_err(|e| TunnelError::IoError(e.to_string()))?;
    let target_port = u16::from_be_bytes(port_buf);

    debug!("SOCKS5 request: {}:{}", target_addr, target_port);

    // Connect to target
    match TcpStream::connect(format!("{}:{}", target_addr, target_port)).await {
        Ok(mut outbound) => {
            // Reply: success
            inbound.write_all(&[0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).await
                .map_err(|e| TunnelError::IoError(e.to_string()))?;

            // Bidirectional copy
            let (mut ri, mut wi) = inbound.split();
            let (mut ro, mut wo) = outbound.split();

            tokio::select! {
                r1 = tokio::io::copy(&mut ri, &mut wo) => {
                    if let Err(e) = r1 {
                        debug!("SOCKS5 upstream copy error: {}", e);
                    }
                }
                r2 = tokio::io::copy(&mut ro, &mut wi) => {
                    if let Err(e) = r2 {
                        debug!("SOCKS5 downstream copy error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            warn!("SOCKS5 connection to {}:{} failed: {}", target_addr, target_port, e);
            // Reply: connection refused
            inbound.write_all(&[0x05, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).await
                .map_err(|e| TunnelError::IoError(e.to_string()))?;
        }
    }

    Ok(())
}

// ─── Tunnel Manager ────────────────────────────────────────────────────────────

pub struct TunnelManager {
    tunnels: Arc<DashMap<Uuid, Arc<TunnelInstance>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self { tunnels: Arc::new(DashMap::new()) }
    }

    pub async fn start_tunnel(&self, config: TunnelConfig) -> Result<Arc<TunnelInstance>, TunnelError> {
        info!("Starting {} tunnel {} on {}:{}",
            config.kind.as_flag(), config.label, config.bind_addr, config.bind_port);

        let active = Arc::new(AtomicBool::new(true));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

        let instance = Arc::new(TunnelInstance {
            status: Arc::new(RwLock::new(TunnelStatus {
                id: config.id,
                kind: config.kind.clone(),
                bind_addr: config.bind_addr.clone(),
                bind_port: config.bind_port,
                target: config.target_host.as_deref().unwrap_or("*").to_string(),
                active: true,
                bytes_in: 0,
                bytes_out: 0,
                connected_since: Some(chrono::Utc::now().to_rfc3339()),
                error: None,
            })),
            active: active.clone(),
            config: config.clone(),
            shutdown_token: shutdown_tx,
            join_handle: Arc::new(Mutex::new(None)),
        });

        let instance_clone = instance.clone();
        let active_clone = active.clone();
        let session_id = config.session_id;

        match config.kind {
            TunnelKind::Dynamic => {
                let bind_addr = config.bind_addr.clone();
                let bind_port = config.bind_port;
                let instance_ref = instance.clone();

                let handle = tokio::spawn(async move {
                    let listener = match TcpListener::bind(format!("{}:{}", bind_addr, bind_port)).await {
                        Ok(l) => l,
                        Err(e) => {
                            error!("Failed to bind SOCKS5 proxy on {}:{}: {}", bind_addr, bind_port, e);
                            active_clone.store(false, Ordering::Relaxed);
                            let mut status = instance_ref.status.write();
                            status.active = false;
                            status.error = Some(format!("Bind failed: {}", e));
                            return;
                        }
                    };

                    info!("SOCKS5 proxy listening on {}:{}", bind_addr, bind_port);

                    loop {
                        tokio::select! {
                            _ = shutdown_rx.changed() => {
                                info!("SOCKS5 proxy shutting down on {}:{}", bind_addr, bind_port);
                                break;
                            }
                            result = listener.accept() => {
                                match result {
                                    Ok((inbound, addr)) => {
                                        debug!("SOCKS5 connection from {}", addr);
                                        let session_id = session_id;
                                        tokio::spawn(async move {
                                            if let Err(e) = handle_socks5_connection(inbound, session_id).await {
                                                debug!("SOCKS5 handler error: {}", e);
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        warn!("SOCKS5 accept error: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    active_clone.store(false, Ordering::Relaxed);
                });

                *instance.join_handle.lock().await = Some(handle);
            }
            TunnelKind::Local => {
                let bind_addr = config.bind_addr.clone();
                let bind_port = config.bind_port;
                let target_host = config.target_host.clone().unwrap_or_default();
                let target_port = config.target_port.unwrap_or(0);
                let instance_ref = instance.clone();

                let handle = tokio::spawn(async move {
                    let listener = match TcpListener::bind(format!("{}:{}", bind_addr, bind_port)).await {
                        Ok(l) => l,
                        Err(e) => {
                            error!("Failed to bind local forward on {}:{}: {}", bind_addr, bind_port, e);
                            active_clone.store(false, Ordering::Relaxed);
                            let mut status = instance_ref.status.write();
                            status.active = false;
                            status.error = Some(format!("Bind failed: {}", e));
                            return;
                        }
                    };

                    info!("Local forward {}:{} -> {}:{}", bind_addr, bind_port, target_host, target_port);

                    loop {
                        tokio::select! {
                            _ = shutdown_rx.changed() => break,
                            result = listener.accept() => {
                                if let Ok((inbound, addr)) = result {
                                    debug!("Local forward connection from {}", addr);
                                    let target = format!("{}:{}", target_host, target_port);
                                    tokio::spawn(async move {
                                        match TcpStream::connect(&target).await {
                                            Ok(outbound) => {
                                                let (mut ri, mut wi) = inbound.split();
                                                let (mut ro, mut wo) = outbound.split();
                                                let _ = tokio::select! {
                                                    r = tokio::io::copy(&mut ri, &mut wo) => r,
                                                    r = tokio::io::copy(&mut ro, &mut wi) => r,
                                                };
                                            }
                                            Err(e) => {
                                                warn!("Local forward connect failed: {}", e);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }

                    active_clone.store(false, Ordering::Relaxed);
                });

                *instance.join_handle.lock().await = Some(handle);
            }
            TunnelKind::Remote => {
                warn!("Remote forwarding requires SSH session - use SSH -R command instead");
                active.store(false, Ordering::Relaxed);
                let mut status = instance.status.write();
                status.active = false;
                status.error = Some("Remote forwarding not implemented in tunnel manager".into());
            }
        }

        self.tunnels.insert(config.id, instance.clone());
        info!("Tunnel {} created ({} on {}:{})", config.label, config.kind.as_flag(),
            config.bind_addr, config.bind_port);

        Ok(instance)
    }

    pub async fn stop_tunnel(&self, id: &Uuid) -> Result<(), TunnelError> {
        if let Some((_, instance)) = self.tunnels.remove(id) {
            instance.stop().await?;
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<(), TunnelError> {
        let ids: Vec<Uuid> = self.tunnels.iter().map(|e| *e.key()).collect();
        for id in ids {
            self.stop_tunnel(&id).await?;
        }
        Ok(())
    }

    pub fn get_tunnel(&self, id: &Uuid) -> Option<Arc<TunnelInstance>> {
        self.tunnels.get(id).map(|e| e.value().clone())
    }

    pub fn list_tunnels(&self) -> Vec<TunnelStatus> {
        self.tunnels.iter().map(|e| e.value().status()).collect()
    }
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_config_serialization() {
        let config = TunnelConfig {
            id: Uuid::new_v4(),
            label: "Dev SOCKS Proxy".into(),
            session_id: Uuid::new_v4(),
            kind: TunnelKind::Dynamic,
            bind_addr: "127.0.0.1".into(),
            bind_port: 1080,
            target_host: None,
            target_port: None,
            enabled: true,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("1080"));
        assert!(json.contains("Dynamic"));

        let deserialized: TunnelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, TunnelKind::Dynamic);
    }

    #[test]
    fn test_tunnel_kind_flags() {
        assert_eq!(TunnelKind::Dynamic.as_flag(), "-D");
        assert_eq!(TunnelKind::Local.as_flag(), "-L");
        assert_eq!(TunnelKind::Remote.as_flag(), "-R");
    }
}
