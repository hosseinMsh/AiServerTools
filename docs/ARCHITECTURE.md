# InfraCore Architecture

Enterprise SSH & Infrastructure Control Platform

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    USER INTERFACE LAYER                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Dashboard│ │ Terminal │ │  Docker  │ │  AI Assistant  │  │
│  │ Servers  │ │ (xterm)  │ │  Manager │ │  Chat UI       │  │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └───────┬────────┘  │
│       │            │             │               │           │
│  ┌────┴────────────┴─────────────┴───────────────┴────────┐  │
│  │              Tauri IPC Bridge (invoke)                  │  │
│  └─────────────────────────┬───────────────────────────────┘  │
└────────────────────────────┼──────────────────────────────────┘
                             │
┌────────────────────────────┼──────────────────────────────────┐
│                    RUNTIME LAYER (Rust)                       │
│  ┌────────────────────────┴───────────────────────────────┐   │
│  │              AI Orchestrator Engine                     │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │ Intent   │ │ Provider │ │ Tool     │ │ Context  │  │   │
│  │  │ Classify │ │ Router   │ │ Registry │ │ Memory   │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  └────────────────────────────────────────────────────────┘   │
│                             │                                  │
│  ┌──────────────────────────┴──────────────────────────────┐  │
│  │                  TOOL EXECUTION ENGINE                   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │  │
│  │  │   SSH    │ │  Docker  │ │  SFTP    │ │   Tunnel   │  │  │
│  │  │  Engine  │ │  Engine  │ │  Engine  │ │   Engine   │  │  │
│  │  │ (russh)  │ │ (REST)   │ │ (SFTP)   │ │ (SOCKS5)   │  │  │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └─────┬──────┘  │  │
│  └───────┼────────────┼────────────┼───────────────┼───────┘  │
│          │            │            │               │          │
│  ┌───────┴────────────┴────────────┴───────────────┴───────┐  │
│  │              SECURITY & INFRASTRUCTURE                   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │  │
│  │  │Encryption│ │  Vault   │ │  Policy  │ │   Audit    │  │  │
│  │  │ AES-256  │ │ Keychain │ │  Engine  │ │   Logger   │  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
                             │
┌────────────────────────────┼──────────────────────────────────┐
│                    REMOTE LAYER                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │  Linux   │ │  macOS   │ │ Windows  │ │  Docker Daemon   │  │
│  │ Servers  │ │ Servers  │ │ Servers  │ │  (remote API)    │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────────┘  │
└────────────────────────────────────────────────────────────────┘
```

## Module Architecture

### 1. SSH Engine (`packages/core/ssh-engine`)
- **Protocol**: OpenSSH-compatible via `russh`
- **Auth Methods**: Password, RSA/ED25519 keys, SSH Agent
- **Features**: ProxyJump (-J), keepalive, compression, multiplexing
- **Key Types**: Connection config, session handle, shell handle, command result

### 2. Docker Engine (`packages/core/docker-engine`)
- **Protocol**: Docker Engine API v1.45 via REST/HTTP
- **Features**: Container CRUD, logs streaming, stats, image management
- **Connection**: Direct TCP or SSH-tunneled socket
- **Key Types**: ContainerSummary, ContainerStats, ImageSummary

### 3. SFTP Engine (`packages/core/sftp-engine`)
- **Protocol**: SFTP v3 over SSH channel
- **Features**: Directory listing, file read/write, transfer progress, safe overwrite
- **Key Types**: FileInfo, TransferProgress, TransferRequest

### 4. Tunnel Engine (`packages/core/tunnel-engine`)
- **Protocol**: SOCKS5, TCP forwarding
- **Features**: Dynamic proxy (-D), local (-L), remote (-R) forwarding
- **Key Types**: TunnelConfig, TunnelStatus, TunnelInstance

### 5. AI Engine (`packages/ai-engine`)
- **Multi-Provider**: OpenAI, OpenAI-compatible, Custom APIs
- **Features**: Intent classification, tool calling, provider routing, dynamic model fetching
- **Key Types**: AiProviderConfig, ChatRequest/Response, ToolDefinition, ModelInfo

### 6. Security Module (`packages/core/security`)
- **Encryption**: AES-256-GCM for data at rest, Argon2 for key derivation
- **Vault**: Encrypted credential storage with label-based retrieval
- **Policy**: Role-based action evaluation (Allow/Deny/RequireMFA)
- **Key Types**: VaultEntry, Policy, PolicyDecision

### 7. Audit Module (`packages/core/audit`)
- **Events**: All SSH, Docker, SFTP, tunnel, config actions logged
- **Recording**: Full terminal session recording with replay support
- **Key Types**: AuditEvent, SessionRecording, RecordingChunk

### 8. Context Engine (`packages/core/context-engine`)
- **State**: Server state tracking with metrics caching
- **History**: Command history per server
- **Groups**: Server grouping by environment
- **Key Types**: ServerState, ServerMetrics, CommandRecord, ServerGroup

### 9. Plugin System (`packages/plugin-system`)
- **Isolation**: WASM-based sandboxed plugin runtime
- **Permissions**: Granular permission model (SSH, Docker, FS, etc.)
- **Hooks**: Event-driven plugin hooks
- **Key Types**: PluginManifest, PluginState, PluginPermission

## Data Flow

```
User Input
    │
    ▼
┌─────────────┐     ┌──────────────┐     ┌────────────────┐
│  Intent     │────►│  AI Provider │────►│  Tool Registry │
│  Classifier │     │  (LLM)       │     │  (function call)│
└─────────────┘     └──────────────┘     └───────┬────────┘
                                                  │
                    ┌─────────────────────────────┘
                    ▼
          ┌─────────────────┐
          │  Policy Engine  │── Denied? → Return error
          └────────┬────────┘
                   │ Allowed
                   ▼
          ┌─────────────────┐
          │  Tool Executor  │
          │  (SSH/Docker/   │
          │   SFTP/Tunnel)  │
          └────────┬────────┘
                   │
          ┌────────▼────────┐
          │  Audit Logger   │── Log event
          └─────────────────┘
                   │
                   ▼
           Return Result
```

## Security Architecture (Zero Trust)

1. **Encryption at Rest**: All secrets encrypted with AES-256-GCM, key derived via Argon2
2. **Encryption in Transit**: SSH tunnel, TLS for Docker/API
3. **Vault**: Master-password-protected, auto-locks on inactivity
4. **Policy Engine**: Every operation checked against role-based policies
5. **Audit Trail**: All commands and actions logged immutably
6. **Plugin Sandbox**: WASM isolation, no direct system access
7. **No Plaintext Secrets**: Memory is zeroed after use, no logging of secrets

## Key Design Decisions

- **Rust for Core**: Memory safety, zero-cost abstractions, async I/O
- **russh over ssh2**: Pure Rust, async, no C dependencies
- **Tauri over Electron**: Smaller binary, lower memory, native performance
- **SQLite via SQLCipher**: Portable encrypted database
- **WASM Plugins**: Sandboxed, secure, cross-platform plugin runtime
- **Multi-Provider AI**: Vendor independence, custom model support

## Build Targets

| Platform    | Architecture | Binary            |
|-------------|-------------|-------------------|
| macOS       | x86_64, aarch64 | .dmg, .app   |
| Windows     | x86_64       | .msi, .exe        |
| Linux       | x86_64, aarch64 | .deb, .AppImage |
| Docker      | linux/amd64, linux/arm64 | CLI only |

## Performance Targets

- SSH connection: < 500ms
- Docker API response: < 200ms
- AI chat response: < 2s (streaming)
- File transfer: Line speed via SFTP
- Tunnel throughput: Hardware limited, no per-packet overhead
- Memory: < 50MB idle, < 200MB with active sessions
