-- InfraCore Database Schema
-- SQLite 3.x - Encrypted at rest via SQLCipher

-- ─── Server Configuration ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS servers (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    host            TEXT NOT NULL,
    port            INTEGER NOT NULL DEFAULT 22,
    username        TEXT NOT NULL,
    auth_method     TEXT NOT NULL CHECK(auth_method IN ('key', 'password', 'agent')),
    auth_ref        TEXT,  -- reference to vault entry for key/password
    proxy_jump_id   TEXT,
    keepalive       INTEGER NOT NULL DEFAULT 30,
    compression     INTEGER NOT NULL DEFAULT 1,
    group_name      TEXT,
    tags            TEXT,  -- JSON array
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (proxy_jump_id) REFERENCES servers(id) ON DELETE SET NULL
);

CREATE INDEX idx_servers_group ON servers(group_name);

-- ─── Docker Connections ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS docker_connections (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    server_id       TEXT NOT NULL,
    api_port        INTEGER NOT NULL DEFAULT 2375,
    tls_enabled     INTEGER NOT NULL DEFAULT 0,
    ca_cert_path    TEXT,
    cert_path       TEXT,
    key_path        TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
);

-- ─── SSH Tunnels ───────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS tunnels (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    session_id      TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK(kind IN ('dynamic', 'local', 'remote')),
    bind_addr       TEXT NOT NULL DEFAULT '127.0.0.1',
    bind_port       INTEGER NOT NULL,
    target_host     TEXT,
    target_port     INTEGER,
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES servers(id) ON DELETE CASCADE
);

-- ─── Security Vault ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS vault_entries (
    id              TEXT PRIMARY KEY,
    label           TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK(kind IN ('ssh_key', 'password', 'api_key', 'token', 'certificate', 'other')),
    encrypted_data  TEXT NOT NULL,  -- AES-256-GCM encrypted
    tags            TEXT,  -- JSON array
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_vault_label ON vault_entries(label);

-- ─── Audit Log ─────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS audit_events (
    id              TEXT PRIMARY KEY,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now')),
    event_type      TEXT NOT NULL,
    user_id         TEXT,
    session_id      TEXT,
    server_id       TEXT,
    action          TEXT NOT NULL,
    resource        TEXT NOT NULL,
    details         TEXT,  -- JSON
    result          TEXT NOT NULL,
    ip_address      TEXT,
    user_agent      TEXT
);

CREATE INDEX idx_audit_timestamp ON audit_events(timestamp);
CREATE INDEX idx_audit_event_type ON audit_events(event_type);
CREATE INDEX idx_audit_user ON audit_events(user_id);
CREATE INDEX idx_audit_server ON audit_events(server_id);

-- ─── Session Recordings ────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS session_recordings (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    server_label    TEXT NOT NULL,
    user_name       TEXT NOT NULL,
    started_at      TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at        TEXT,
    total_chunks    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS recording_chunks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    recording_id    TEXT NOT NULL,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now')),
    kind            TEXT NOT NULL CHECK(kind IN ('stdin', 'stdout', 'stderr', 'resize', 'event')),
    data            TEXT NOT NULL,
    sequence        INTEGER NOT NULL,
    FOREIGN KEY (recording_id) REFERENCES session_recordings(id) ON DELETE CASCADE
);

CREATE INDEX idx_chunks_recording ON recording_chunks(recording_id, sequence);

-- ─── AI Providers ──────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS ai_providers (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    provider_type   TEXT NOT NULL CHECK(provider_type IN ('openai', 'openai_compatible', 'custom')),
    api_base_url    TEXT NOT NULL,
    api_key_ref     TEXT,  -- reference to vault entry
    models_endpoint TEXT NOT NULL DEFAULT '/models',
    default_model   TEXT NOT NULL,
    max_tokens      INTEGER NOT NULL DEFAULT 4096,
    temperature     REAL NOT NULL DEFAULT 0.3,
    timeout_secs    INTEGER NOT NULL DEFAULT 60,
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ─── Server Metrics History ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS metrics_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id       TEXT NOT NULL,
    cpu_percent     REAL,
    memory_used     INTEGER,
    memory_total    INTEGER,
    disk_used       INTEGER,
    disk_total      INTEGER,
    load_1m         REAL,
    load_5m         REAL,
    load_15m        REAL,
    network_rx      INTEGER,
    network_tx      INTEGER,
    collected_at    TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
);

CREATE INDEX idx_metrics_server_time ON metrics_history(server_id, collected_at);

-- ─── Users & Permissions ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    email           TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    role            TEXT NOT NULL CHECK(role IN ('admin', 'operator', 'viewer')),
    password_hash   TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_login      TEXT
);

CREATE TABLE IF NOT EXISTS user_sessions (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL,
    token           TEXT NOT NULL UNIQUE,
    expires_at      TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ─── Server Groups ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS server_groups (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    environment     TEXT NOT NULL CHECK(environment IN ('development', 'staging', 'production', 'dr', 'custom')),
    description     TEXT,
    tags            TEXT  -- JSON array
);

-- ─── Command Snippets ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS snippets (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT,
    command         TEXT NOT NULL,
    category        TEXT NOT NULL DEFAULT 'general',
    is_dangerous    INTEGER NOT NULL DEFAULT 0,
    created_by      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ─── Plugin Registry ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS plugins (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    version         TEXT NOT NULL,
    author          TEXT,
    description     TEXT,
    entry_point     TEXT NOT NULL,
    permissions     TEXT NOT NULL,  -- JSON array
    enabled         INTEGER NOT NULL DEFAULT 0,
    installed_at    TEXT NOT NULL DEFAULT (datetime('now')),
    config          TEXT  -- JSON
);

-- ─── Schema Version ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS schema_version (
    version         INTEGER PRIMARY KEY,
    applied_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO schema_version (version) VALUES (1);
