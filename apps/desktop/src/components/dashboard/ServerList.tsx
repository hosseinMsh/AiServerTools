import { useState } from 'react';
import { useAppStore, Server } from '../../stores/appStore';
import { Plus, Monitor, Wifi, WifiOff, MoreHorizontal } from 'lucide-react';

const mockServers: Server[] = [
  { id: '1', label: 'Prod Web 01', host: '10.0.0.1', port: 22, username: 'deploy', authMethod: 'key', status: 'online', group: 'Production', tags: ['web', 'prod'], metrics: { cpu: 23, memory: 45, disk: 62, load1m: 0.5, load5m: 0.3, load15m: 0.2, uptime: '14d' } },
  { id: '2', label: 'Prod Web 02', host: '10.0.0.2', port: 22, username: 'deploy', authMethod: 'key', status: 'online', group: 'Production', tags: ['web', 'prod'], metrics: { cpu: 31, memory: 52, disk: 58, load1m: 0.8, load5m: 0.6, load15m: 0.4, uptime: '14d' } },
  { id: '3', label: 'Staging DB', host: '10.0.1.10', port: 22, username: 'admin', authMethod: 'password', status: 'online', group: 'Staging', tags: ['db', 'staging'], metrics: { cpu: 12, memory: 34, disk: 41, load1m: 0.2, load5m: 0.1, load15m: 0.1, uptime: '7d' } },
  { id: '4', label: 'Dev API Server', host: '192.168.1.50', port: 22, username: 'dev', authMethod: 'agent', status: 'offline', group: 'Development', tags: ['api', 'dev'], metrics: undefined },
  { id: '5', label: 'Prod Docker Host', host: '10.0.0.10', port: 22, username: 'docker', authMethod: 'key', status: 'online', group: 'Production', tags: ['docker', 'prod'], metrics: { cpu: 67, memory: 78, disk: 85, load1m: 2.1, load5m: 1.8, load15m: 1.5, uptime: '30d' } },
];

const statusConfig = {
  online: { label: 'Online', color: 'var(--success)', Icon: Wifi },
  offline: { label: 'Offline', color: 'var(--danger)', Icon: WifiOff },
  unknown: { label: 'Unknown', color: 'var(--text-muted)', Icon: WifiOff },
  connecting: { label: 'Connecting...', color: 'var(--warning)', Icon: WifiOff },
};

export function ServerList() {
  const [servers] = useState<Server[]>(mockServers);
  const setActiveServer = useAppStore((s) => s.setActiveServer);

  return (
    <div style={{ padding: 24 }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>Servers</h1>
          <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
            {servers.length} servers · {servers.filter((s) => s.status === 'online').length} online
          </p>
        </div>
        <button className="btn btn-primary" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Plus size={16} />
          Add Server
        </button>
      </div>

      {/* Group by environment */}
      {['Production', 'Staging', 'Development'].map((group) => {
        const groupServers = servers.filter((s) => s.group === group);
        if (groupServers.length === 0) return null;
        return (
          <div key={group} style={{ marginBottom: 32 }}>
            <h2 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: 1, marginBottom: 12 }}>
              {group}
            </h2>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: 12 }}>
              {groupServers.map((server) => {
                const status = statusConfig[server.status];
                const StatusIcon = status.Icon;
                return (
                  <div
                    key={server.id}
                    className="card card-hover"
                    onClick={() => setActiveServer(server.id)}
                    style={{ cursor: 'pointer' }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 12 }}>
                      <div>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                          <Monitor size={16} color="var(--accent)" />
                          <span style={{ fontWeight: 600, fontSize: 15 }}>{server.label}</span>
                        </div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: 13, fontFamily: 'monospace' }}>
                          {server.username}@{server.host}:{server.port}
                        </div>
                      </div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <span className={`badge badge-${server.status === 'online' ? 'success' : 'danger'}`}>
                          <StatusIcon size={12} style={{ marginRight: 4 }} />
                          {status.label}
                        </span>
                        <button
                          className="btn-ghost"
                          style={{ padding: 4, border: 'none', background: 'transparent', cursor: 'pointer' }}
                          onClick={(e) => { e.stopPropagation(); }}
                        >
                          <MoreHorizontal size={16} color="var(--text-muted)" />
                        </button>
                      </div>
                    </div>

                    {/* Tags */}
                    <div style={{ display: 'flex', gap: 4, marginBottom: 12, flexWrap: 'wrap' }}>
                      {server.tags.map((tag) => (
                        <span key={tag} className="badge badge-info" style={{ fontSize: 11 }}>
                          {tag}
                        </span>
                      ))}
                    </div>

                    {/* Metrics */}
                    {server.metrics ? (
                      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 8 }}>
                        <MetricBar label="CPU" value={server.metrics.cpu} color="var(--accent)" />
                        <MetricBar label="RAM" value={server.metrics.memory} color="var(--success)" />
                        <MetricBar label="DISK" value={server.metrics.disk} color="var(--warning)" />
                      </div>
                    ) : (
                      <div style={{ color: 'var(--text-muted)', fontSize: 12, fontStyle: 'italic' }}>
                        No metrics available
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function MetricBar({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, marginBottom: 4 }}>
        <span style={{ color: 'var(--text-secondary)' }}>{label}</span>
        <span style={{ color: 'var(--text-primary)' }}>{value}%</span>
      </div>
      <div style={{ height: 4, background: 'var(--bg-tertiary)', borderRadius: 2, overflow: 'hidden' }}>
        <div
          style={{
            width: `${value}%`,
            height: '100%',
            background: color,
            borderRadius: 2,
            transition: 'width 0.3s ease',
          }}
        />
      </div>
    </div>
  );
}
