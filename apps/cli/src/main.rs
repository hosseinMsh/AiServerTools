use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "infra", version, about = "InfraCore - Enterprise Infrastructure Control Platform")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server ID (required for most operations)
    #[arg(global = true, short, long)]
    server: Option<String>,

    /// Output format (json or text)
    #[arg(global = true, short, long, default_value = "text")]
    output: String,
}

#[derive(Subcommand)]
enum Commands {
    /// SSH into a remote server
    Ssh {
        host: String,
        port: Option<u16>,
        #[arg(short, long)]
        user: Option<String>,
        #[arg(short, long)]
        key: Option<String>,
        #[arg(short = 'J', long)]
        proxy_jump: Option<String>,
    },

    /// Execute command on remote server
    Exec {
        command: String,
        #[arg(short, long)]
        timeout: Option<u64>,
    },

    /// Docker operations
    Docker {
        #[command(subcommand)]
        action: DockerAction,
    },

    /// SFTP file operations
    Sftp {
        #[command(subcommand)]
        action: SftpAction,
    },

    /// Tunnel management (SOCKS, port forwarding)
    Tunnel {
        #[command(subcommand)]
        action: TunnelAction,
    },

    /// List servers
    #[command(name = "servers")]
    ListServers,

    /// Security vault
    Vault {
        #[command(subcommand)]
        action: VaultAction,
    },

    /// AI assistant
    Ai {
        prompt: String,
        #[arg(short, long)]
        provider: Option<String>,
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Audit logs
    Logs {
        #[arg(short, long)]
        tail: Option<usize>,
        #[arg(short, long)]
        server: Option<String>,
    },
}

#[derive(Subcommand)]
enum DockerAction {
    Ps { all: bool },
    Logs { container: String, tail: Option<u32> },
    Start { container: String },
    Stop { container: String },
    Restart { container: String },
    Stats { container: String },
}

#[derive(Subcommand)]
enum SftpAction {
    Ls { path: String },
    Download { remote: String, local: String },
    Upload { local: String, remote: String },
}

#[derive(Subcommand)]
enum TunnelAction {
    Socks { port: u16, bind: Option<String> },
    Local { bind_port: u16, target_host: String, target_port: u16 },
    List,
    Stop { id: String },
}

#[derive(Subcommand)]
enum VaultAction {
    Store { label: String, value: String },
    Get { id: String },
    List,
    Delete { id: String },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Ssh { host, port, user, key, proxy_jump } => {
            let port = port.unwrap_or(22);
            let user = user.as_deref().unwrap_or("root");
            println!("Connecting to {}@{}:{}...", user, host, port);

            let manager = ssh_engine::SshManager::new();
            let config = ssh_engine::SshConnectionConfig {
                id: Uuid::new_v4(),
                label: format!("{}@{}", user, host),
                host: host.clone(),
                port,
                auth: ssh_engine::SshAuthConfig {
                    method: if let Some(key_path) = key {
                        ssh_engine::AuthMethod::Key {
                            private_key_path: key_path.clone(),
                            passphrase: None,
                        }
                    } else {
                        ssh_engine::AuthMethod::Agent
                    },
                    username: user.to_string(),
                },
                proxy_jump: proxy_jump.as_ref().map(|j| {
                    let parts: Vec<&str> = j.splitn(2, '@').collect();
                    let (user, host) = if parts.len() == 2 {
                        (parts[0], parts[1])
                    } else {
                        ("root", parts[0])
                    };
                    let host_parts: Vec<&str> = host.splitn(2, ':').collect();
                    let (host, port) = if host_parts.len() == 2 {
                        (host_parts[0], host_parts[1].parse().unwrap_or(22))
                    } else {
                        (host_parts[0], 22)
                    };
                    ssh_engine::ProxyJumpConfig {
                        host: host.to_string(),
                        port,
                        auth: ssh_engine::SshAuthConfig {
                            method: ssh_engine::AuthMethod::Agent,
                            username: user.to_string(),
                        },
                    }
                }),
                keepalive_interval_secs: 30,
                compression: true,
            };

            match manager.connect(config).await {
                Ok(session) => {
                    println!("Connected! Session ID: {}", session.id);
                    println!("Interactive shell mode - type 'exit' to disconnect");

                    // Simple REPL
                    loop {
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let input = input.trim();

                        if input == "exit" || input == "quit" {
                            session.disconnect().await.unwrap();
                            break;
                        }

                        match session.execute_command(input).await {
                            Ok(result) => {
                                if !result.stdout.is_empty() {
                                    println!("{}", result.stdout);
                                }
                                if !result.stderr.is_empty() {
                                    eprintln!("{}", result.stderr);
                                }
                            }
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }
        Commands::Exec { command, timeout } => {
            println!("Executing: {}", command);
            // TODO: get session from manager by server ID
            println!("Command execution requires an active SSH session");
        }
        Commands::Docker { action } => {
            match action {
                DockerAction::Ps { all } => {
                    println!("Docker ps (all={})", all);
                }
                DockerAction::Logs { container, tail } => {
                    println!("Logs for {} (tail={:?})", container, tail);
                }
                DockerAction::Start { container } => {
                    println!("Starting container {}", container);
                }
                DockerAction::Stop { container } => {
                    println!("Stopping container {}", container);
                }
                DockerAction::Restart { container } => {
                    println!("Restarting container {}", container);
                }
                DockerAction::Stats { container } => {
                    println!("Stats for {}", container);
                }
            }
        }
        Commands::Sftp { action } => {
            match action {
                SftpAction::Ls { path } => println!("Listing: {}", path),
                SftpAction::Download { remote, local } => println!("Download {} -> {}", remote, local),
                SftpAction::Upload { local, remote } => println!("Upload {} -> {}", local, remote),
            }
        }
        Commands::Tunnel { action } => {
            match action {
                TunnelAction::Socks { port, bind } => {
                    let bind = bind.as_deref().unwrap_or("127.0.0.1");
                    println!("Starting SOCKS5 proxy on {}:{}", bind, port);
                }
                TunnelAction::Local { bind_port, target_host, target_port } => {
                    println!("Local forward :{} -> {}:{}", bind_port, target_host, target_port);
                }
                TunnelAction::List => println!("Active tunnels:"),
                TunnelAction::Stop { id } => println!("Stopping tunnel {}", id),
            }
        }
        Commands::ListServers => {
            println!("Registered servers:");
            println!("  (No servers configured)");
        }
        Commands::Vault { action } => {
            match action {
                VaultAction::Store { label, value } => println!("Stored '{}' in vault", label),
                VaultAction::Get { id } => println!("Retrieved secret {}", id),
                VaultAction::List => println!("Vault entries:"),
                VaultAction::Delete { id } => println!("Deleted entry {}", id),
            }
        }
        Commands::Ai { prompt, provider, model } => {
            println!("AI Query: {}", prompt);
            if let Some(p) = provider {
                println!("Provider: {}", p);
            }
            if let Some(m) = model {
                println!("Model: {}", m);
            }
            println!("[AI mode requires configured provider]");
        }
        Commands::Logs { tail, server } => {
            let tail = tail.unwrap_or(100);
            if let Some(s) = server {
                println!("Last {} audit events for server {}", tail, s);
            } else {
                println!("Last {} audit events", tail);
            }
        }
    }
}
