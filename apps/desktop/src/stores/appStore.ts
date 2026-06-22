import { create } from 'zustand';

// ─── Types ─────────────────────────────────────────────────────────────────────

export interface Server {
  id: string;
  label: string;
  host: string;
  port: number;
  username: string;
  authMethod: 'key' | 'password' | 'agent';
  status: 'online' | 'offline' | 'unknown' | 'connecting';
  group?: string;
  tags: string[];
  metrics?: ServerMetrics;
}

export interface ServerMetrics {
  cpu: number;
  memory: number;
  disk: number;
  load1m: number;
  load5m: number;
  load15m: number;
  uptime: string;
}

export interface SshSession {
  id: string;
  serverId: string;
  label: string;
  connected: boolean;
  host: string;
  port: number;
}

export interface DockerContainer {
  id: string;
  name: string;
  image: string;
  status: string;
  state: string;
  ports: string;
}

export interface Tunnel {
  id: string;
  type: 'socks' | 'local' | 'remote';
  bindAddr: string;
  bindPort: number;
  target?: string;
  active: boolean;
  sessionId: string;
}

export interface AiProvider {
  id: string;
  name: string;
  type: 'openai' | 'custom';
  baseUrl: string;
  defaultModel: string;
  models: string[];
  enabled: boolean;
}

export interface AuditEvent {
  id: string;
  timestamp: string;
  eventType: string;
  action: string;
  resource: string;
  result: string;
  userId?: string;
}

export interface VaultEntry {
  id: string;
  label: string;
  kind: string;
  createdAt: string;
  tags: string[];
}

export interface Conversation {
  id: string;
  messages: ChatMessage[];
  serverId?: string;
  contextType: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  toolCalls?: ToolCall[];
}

export interface ToolCall {
  id: string;
  tool: string;
  arguments: any;
  result?: any;
}

// ─── Store ─────────────────────────────────────────────────────────────────────

interface AppState {
  // Servers
  servers: Server[];
  activeServerId: string | null;
  addServer: (server: Server) => void;
  removeServer: (id: string) => void;
  updateServerStatus: (id: string, status: Server['status']) => void;
  setActiveServer: (id: string | null) => void;

  // SSH Sessions
  sessions: SshSession[];
  addSession: (session: SshSession) => void;
  removeSession: (id: string) => void;

  // Docker
  containers: DockerContainer[];
  setContainers: (containers: DockerContainer[]) => void;

  // Tunnels
  tunnels: Tunnel[];
  addTunnel: (tunnel: Tunnel) => void;
  removeTunnel: (id: string) => void;
  updateTunnelStatus: (id: string, active: boolean) => void;

  // AI Providers
  providers: AiProvider[];
  activeProviderId: string | null;
  addProvider: (provider: AiProvider) => void;
  removeProvider: (id: string) => void;
  setActiveProvider: (id: string | null) => void;

  // Vault
  vaultEntries: VaultEntry[];
  vaultUnlocked: boolean;
  setVaultUnlocked: (unlocked: boolean) => void;
  addVaultEntry: (entry: VaultEntry) => void;
  removeVaultEntry: (id: string) => void;

  // Audit
  auditEvents: AuditEvent[];
  addAuditEvent: (event: AuditEvent) => void;

  // Conversations
  conversations: Conversation[];
  activeConversationId: string | null;
  addConversation: (conv: Conversation) => void;
  addMessage: (convId: string, message: ChatMessage) => void;
  setActiveConversation: (id: string | null) => void;

  // UI State
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  theme: 'dark' | 'light';
  setTheme: (theme: 'dark' | 'light') => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Servers
  servers: [],
  activeServerId: null,
  addServer: (server) => set((s) => ({ servers: [...s.servers, server] })),
  removeServer: (id) => set((s) => ({ servers: s.servers.filter((x) => x.id !== id) })),
  updateServerStatus: (id, status) =>
    set((s) => ({
      servers: s.servers.map((x) => (x.id === id ? { ...x, status } : x)),
    })),
  setActiveServer: (id) => set({ activeServerId: id }),

  // SSH Sessions
  sessions: [],
  addSession: (session) => set((s) => ({ sessions: [...s.sessions, session] })),
  removeSession: (id) => set((s) => ({ sessions: s.sessions.filter((x) => x.id !== id) })),

  // Docker
  containers: [],
  setContainers: (containers) => set({ containers }),

  // Tunnels
  tunnels: [],
  addTunnel: (tunnel) => set((s) => ({ tunnels: [...s.tunnels, tunnel] })),
  removeTunnel: (id) => set((s) => ({ tunnels: s.tunnels.filter((x) => x.id !== id) })),
  updateTunnelStatus: (id, active) =>
    set((s) => ({
      tunnels: s.tunnels.map((x) => (x.id === id ? { ...x, active } : x)),
    })),

  // AI
  providers: [],
  activeProviderId: null,
  addProvider: (provider) => set((s) => ({ providers: [...s.providers, provider] })),
  removeProvider: (id) => set((s) => ({ providers: s.providers.filter((x) => x.id !== id) })),
  setActiveProvider: (id) => set({ activeProviderId: id }),

  // Vault
  vaultEntries: [],
  vaultUnlocked: false,
  setVaultUnlocked: (unlocked) => set({ vaultUnlocked: unlocked }),
  addVaultEntry: (entry) => set((s) => ({ vaultEntries: [...s.vaultEntries, entry] })),
  removeVaultEntry: (id) => set((s) => ({ vaultEntries: s.vaultEntries.filter((x) => x.id !== id) })),

  // Audit
  auditEvents: [],
  addAuditEvent: (event) =>
    set((s) => ({ auditEvents: [event, ...s.auditEvents].slice(0, 1000) })),

  // Conversations
  conversations: [],
  activeConversationId: null,
  addConversation: (conv) => set((s) => ({ conversations: [...s.conversations, conv] })),
  addMessage: (convId, message) =>
    set((s) => ({
      conversations: s.conversations.map((c) =>
        c.id === convId ? { ...c, messages: [...c.messages, message] } : c
      ),
    })),
  setActiveConversation: (id) => set({ activeConversationId: id }),

  // UI
  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  theme: 'dark',
  setTheme: (theme) => set({ theme }),
}));
