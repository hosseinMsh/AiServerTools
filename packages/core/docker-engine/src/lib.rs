use std::collections::HashMap;
use parking_lot::RwLock;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// ─── Error Types ───────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Compose error: {0}")]
    ComposeError(String),

    #[error("SSH tunnel required")]
    SshTunnelRequired,

    #[error("Serialization error: {0}")]
    SerdeError(String),
}

// ─── Docker Connection ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConnectionConfig {
    pub id: Uuid,
    pub label: String,
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub ca_cert_path: Option<String>,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ssh_tunnel_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContext {
    pub configs: Vec<DockerConnectionConfig>,
    pub active_id: Option<Uuid>,
}

// ─── Container Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: String,
    pub created: String,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub ip: Option<String>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    pub source: String,
    pub destination: String,
    pub mode: String,
    pub rw: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: f64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
    pub pids: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSummary {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub created: String,
    pub size: u64,
    pub virtual_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerLogOptions {
    pub tail: Option<u32>,
    pub follow: bool,
    pub stdout: bool,
    pub stderr: bool,
    pub timestamps: bool,
    pub since: Option<String>,
    pub until: Option<String>,
}

// ─── Docker Client ─────────────────────────────────────────────────────────────

pub struct DockerClient {
    config: DockerConnectionConfig,
    http: HttpClient,
    api_base: String,
}

impl DockerClient {
    pub fn new(config: DockerConnectionConfig) -> Result<Self, DockerError> {
        let api_base = if config.port == 0 {
            // Unix socket mode
            format!("http://localhost/v1.45")
        } else {
            format!("http://{}:{}/v1.45", config.host, config.port)
        };

        let http = HttpClient::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        Ok(Self { config, http, api_base })
    }

    // ─── Container Operations ──────────────────────────────────────────────

    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DockerError> {
        let url = format!("{}/containers/json?all={}", self.api_base, all);
        let resp = self.http.get(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let raw: Vec<serde_json::Value> = resp.json().await
            .map_err(|e| DockerError::SerdeError(e.to_string()))?;

        let containers = raw.into_iter().map(|c| ContainerSummary {
            id: c["Id"].as_str().unwrap_or("").to_string(),
            name: c["Names"].as_array()
                .and_then(|n| n.first())
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .trim_start_matches('/')
                .to_string(),
            image: c["Image"].as_str().unwrap_or("").to_string(),
            status: c["Status"].as_str().unwrap_or("").to_string(),
            state: c["State"].as_str().unwrap_or("").to_string(),
            created: c["Created"].as_i64().map(|t| chrono::DateTime::from_timestamp(t, 0)
                .map(|dt| dt.to_rfc3339()).unwrap_or_default()).unwrap_or_default(),
            ports: c["Ports"].as_array().map(|ports| {
                ports.iter().map(|p| PortMapping {
                    private_port: p["PrivatePort"].as_u64().unwrap_or(0) as u16,
                    public_port: p["PublicPort"].as_u64().map(|v| v as u16),
                    ip: p["IP"].as_str().map(|s| s.to_string()),
                    protocol: p["Type"].as_str().unwrap_or("tcp").to_string(),
                }).collect()
            }).unwrap_or_default(),
            mounts: c["Mounts"].as_array().map(|mounts| {
                mounts.iter().map(|m| MountPoint {
                    source: m["Source"].as_str().unwrap_or("").to_string(),
                    destination: m["Destination"].as_str().unwrap_or("").to_string(),
                    mode: m["Mode"].as_str().unwrap_or("").to_string(),
                    rw: m["RW"].as_bool().unwrap_or(true),
                }).collect()
            }).unwrap_or_default(),
        }).collect();

        Ok(containers)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<(), DockerError> {
        let url = format!("{}/containers/{}/start", self.api_base, container_id);
        let resp = self.http.post(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() && resp.status().as_u16() != 304 {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        info!("Container {} started", container_id);
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str, timeout: Option<u32>) -> Result<(), DockerError> {
        let url = format!("{}/containers/{}/stop?t={}", self.api_base, container_id,
            timeout.unwrap_or(10));
        let resp = self.http.post(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() && resp.status().as_u16() != 304 {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        info!("Container {} stopped", container_id);
        Ok(())
    }

    pub async fn restart_container(&self, container_id: &str, timeout: Option<u32>) -> Result<(), DockerError> {
        let url = format!("{}/containers/{}/restart?t={}", self.api_base, container_id,
            timeout.unwrap_or(10));
        let resp = self.http.post(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        info!("Container {} restarted", container_id);
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str, force: bool) -> Result<(), DockerError> {
        let url = format!("{}/containers/{}?force={}", self.api_base, container_id, force);
        let resp = self.http.delete(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        info!("Container {} removed", container_id);
        Ok(())
    }

    pub async fn container_logs(&self, container_id: &str, opts: ContainerLogOptions) -> Result<String, DockerError> {
        let mut url = format!("{}/containers/{}/logs?stdout={}&stderr={}&timestamps={}",
            self.api_base, container_id, opts.stdout, opts.stderr, opts.timestamps);

        if let Some(tail) = opts.tail {
            url.push_str(&format!("&tail={}", tail));
        }
        if let Some(since) = &opts.since {
            url.push_str(&format!("&since={}", since));
        }
        if let Some(until) = &opts.until {
            url.push_str(&format!("&until={}", until));
        }
        if opts.follow {
            url.push_str("&follow=true");
        }

        let resp = self.http.get(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        resp.text().await.map_err(|e| DockerError::SerdeError(e.to_string()))
    }

    pub async fn container_stats(&self, container_id: &str) -> Result<ContainerStats, DockerError> {
        let url = format!("{}/containers/{}/stats?stream=false", self.api_base, container_id);
        let resp = self.http.get(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let raw: serde_json::Value = resp.json().await
            .map_err(|e| DockerError::SerdeError(e.to_string()))?;

        let cpu_delta = raw["cpu_stats"]["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
        let system_delta = raw["cpu_stats"]["system_cpu_usage"].as_u64().unwrap_or(0);
        let pre_cpu = raw["precpu_stats"]["cpu_usage"]["total_usage"].as_u64().unwrap_or(0);
        let pre_system = raw["precpu_stats"]["system_cpu_usage"].as_u64().unwrap_or(0);

        let cpu_delta = cpu_delta.saturating_sub(pre_cpu);
        let system_delta = system_delta.saturating_sub(pre_system);
        let num_cpus = raw["cpu_stats"]["online_cpus"].as_u64().unwrap_or(1);
        let cpu_percent = if system_delta > 0 {
            (cpu_delta as f64 / system_delta as f64) * num_cpus as f64 * 100.0
        } else {
            0.0
        };

        let mem_usage = raw["memory_stats"]["usage"].as_u64().unwrap_or(0);
        let mem_limit = raw["memory_stats"]["limit"].as_u64().unwrap_or(1);
        let mem_percent = if mem_limit > 0 {
            (mem_usage as f64 / mem_limit as f64) * 100.0
        } else {
            0.0
        };

        let networks = raw["networks"].as_object();
        let (net_rx, net_tx) = networks.map(|n| {
            let total_rx: u64 = n.values().map(|v| v["rx_bytes"].as_u64().unwrap_or(0)).sum();
            let total_tx: u64 = n.values().map(|v| v["tx_bytes"].as_u64().unwrap_or(0)).sum();
            (total_rx, total_tx)
        }).unwrap_or((0, 0));

        Ok(ContainerStats {
            cpu_percent,
            memory_usage: mem_usage,
            memory_limit: mem_limit,
            memory_percent: mem_percent,
            network_rx: net_rx,
            network_tx: net_tx,
            block_read: raw["blkio_stats"]["io_service_bytes_recursive"]
                .as_array().and_then(|a| a.iter()
                    .find(|v| v["op"].as_str() == Some("read"))
                    .and_then(|v| v["value"].as_u64())).unwrap_or(0),
            block_write: raw["blkio_stats"]["io_service_bytes_recursive"]
                .as_array().and_then(|a| a.iter()
                    .find(|v| v["op"].as_str() == Some("write"))
                    .and_then(|v| v["value"].as_u64())).unwrap_or(0),
            pids: raw["pids_stats"]["current"].as_u64().unwrap_or(0) as u32,
        })
    }

    // ─── Image Operations ───────────────────────────────────────────────────

    pub async fn list_images(&self) -> Result<Vec<ImageSummary>, DockerError> {
        let url = format!("{}/images/json", self.api_base);
        let resp = self.http.get(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let raw: Vec<serde_json::Value> = resp.json().await
            .map_err(|e| DockerError::SerdeError(e.to_string()))?;

        let images = raw.into_iter().map(|i| ImageSummary {
            id: i["Id"].as_str().unwrap_or("").to_string(),
            repo_tags: i["RepoTags"].as_array()
                .map(|t| t.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            created: i["Created"].as_i64().map(|t| chrono::DateTime::from_timestamp(t, 0)
                .map(|dt| dt.to_rfc3339()).unwrap_or_default()).unwrap_or_default(),
            size: i["Size"].as_u64().unwrap_or(0),
            virtual_size: i["VirtualSize"].as_u64().unwrap_or(0),
        }).collect();

        Ok(images)
    }

    // ─── System Info ────────────────────────────────────────────────────────

    pub async fn system_info(&self) -> Result<serde_json::Value, DockerError> {
        let url = format!("{}/info", self.api_base);
        let resp = self.http.get(&url).send().await
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DockerError::ApiError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        resp.json().await.map_err(|e| DockerError::SerdeError(e.to_string()))
    }
}

// ─── Docker Manager ────────────────────────────────────────────────────────────

pub struct DockerManager {
    clients: parking_lot::RwLock<HashMap<Uuid, DockerClient>>,
}

impl DockerManager {
    pub fn new() -> Self {
        Self { clients: parking_lot::RwLock::new(HashMap::new()) }
    }

    pub fn add_connection(&self, config: DockerConnectionConfig) -> Result<Uuid, DockerError> {
        let client = DockerClient::new(config.clone())?;
        self.clients.write().insert(config.id, client);
        info!("Docker connection added: {} (id={})", config.label, config.id);
        Ok(config.id)
    }

    pub fn get_client(&self, id: &Uuid) -> Option<tokio::sync::MutexGuard<DockerClient>> {
        // Simplified - in production would return a reference
        None
    }

    pub fn remove_connection(&self, id: &Uuid) {
        self.clients.write().remove(id);
        info!("Docker connection removed: {}", id);
    }
}

impl Default for DockerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_config_serialization() {
        let config = DockerConnectionConfig {
            id: Uuid::new_v4(),
            label: "Production Docker".into(),
            host: "192.168.1.100".into(),
            port: 2375,
            tls_enabled: false,
            ca_cert_path: None,
            cert_path: None,
            key_path: None,
            ssh_tunnel_id: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: DockerConnectionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.host, "192.168.1.100");
        assert_eq!(deserialized.port, 2375);
    }

    #[test]
    fn test_container_summary_serialization() {
        let container = ContainerSummary {
            id: "abc123".into(),
            name: "web-app".into(),
            image: "nginx:latest".into(),
            status: "Up 2 hours".into(),
            state: "running".into(),
            created: "2024-01-01T00:00:00Z".into(),
            ports: vec![PortMapping {
                private_port: 80,
                public_port: Some(8080),
                ip: Some("0.0.0.0".into()),
                protocol: "tcp".into(),
            }],
            mounts: vec![MountPoint {
                source: "/data".into(),
                destination: "/var/data".into(),
                mode: "rw".into(),
                rw: true,
            }],
        };

        let json = serde_json::to_string(&container).unwrap();
        assert!(json.contains("abc123"));
        assert!(json.contains("nginx"));

        let deserialized: ContainerSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.state, "running");
    }
}
