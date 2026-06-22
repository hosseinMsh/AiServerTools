import { useState } from 'react';
import { Container, Play, Square, RotateCcw, Trash2, Search } from 'lucide-react';

interface ContainerItem {
  id: string;
  name: string;
  image: string;
  status: string;
  state: 'running' | 'stopped' | 'exited' | 'paused';
  ports: string;
  created: string;
}

const mockContainers: ContainerItem[] = [
  { id: 'a1b2c3', name: 'web-app', image: 'nginx:1.25', status: 'Up 2 hours', state: 'running', ports: '0.0.0.0:80->80/tcp', created: '2024-01-15' },
  { id: 'd4e5f6', name: 'api-backend', image: 'node:20-alpine', status: 'Up 2 hours', state: 'running', ports: '0.0.0.0:3000->3000/tcp', created: '2024-01-15' },
  { id: 'g7h8i9', name: 'redis-cache', image: 'redis:7-alpine', status: 'Exited (0) 1 hour ago', state: 'exited', ports: '6379/tcp', created: '2024-01-14' },
  { id: 'j0k1l2', name: 'postgres-db', image: 'postgres:16', status: 'Up 3 days', state: 'running', ports: '0.0.0.0:5432->5432/tcp', created: '2024-01-12' },
  { id: 'm3n4o5', name: 'grafana', image: 'grafana/grafana:latest', status: 'Up 3 days', state: 'running', ports: '0.0.0.0:3001->3000/tcp', created: '2024-01-12' },
];

const stateColors: Record<string, string> = {
  running: 'var(--success)',
  stopped: 'var(--text-muted)',
  exited: 'var(--danger)',
  paused: 'var(--warning)',
};

export function DockerScreen() {
  const [containers] = useState(mockContainers);
  const [filter, setFilter] = useState('');

  const filtered = containers.filter(
    (c) =>
      c.name.toLowerCase().includes(filter.toLowerCase()) ||
      c.image.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>Docker</h1>
          <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
            {containers.length} containers · {containers.filter((c) => c.state === 'running').length} running
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div style={{ position: 'relative' }}>
            <Search size={14} style={{ position: 'absolute', left: 10, top: '50%', transform: 'translateY(-50%)', color: 'var(--text-muted)' }} />
            <input
              className="input"
              style={{ paddingLeft: 32, width: 240 }}
              placeholder="Search containers..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </div>
          <button className="btn btn-primary" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Container size={16} />
            Connect
          </button>
        </div>
      </div>

      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 14 }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border)', background: 'var(--bg-tertiary)' }}>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Status</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Name</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Image</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Ports</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Status</th>
              <th style={{ padding: '10px 16px', textAlign: 'right', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((container) => (
              <tr key={container.id} style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '12px 16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <div style={{ width: 8, height: 8, borderRadius: '50%', background: stateColors[container.state] }} />
                    <span style={{ fontSize: 12, color: 'var(--text-secondary)', textTransform: 'capitalize' }}>{container.state}</span>
                  </div>
                </td>
                <td style={{ padding: '12px 16px', fontWeight: 500 }}>{container.name}</td>
                <td style={{ padding: '12px 16px', color: 'var(--text-secondary)', fontFamily: 'monospace', fontSize: 13 }}>{container.image}</td>
                <td style={{ padding: '12px 16px', color: 'var(--text-secondary)', fontFamily: 'monospace', fontSize: 13 }}>{container.ports}</td>
                <td style={{ padding: '12px 16px', color: 'var(--text-secondary)', fontSize: 13 }}>{container.status}</td>
                <td style={{ padding: '12px 16px', textAlign: 'right' }}>
                  <div style={{ display: 'flex', gap: 4, justifyContent: 'flex-end' }}>
                    {container.state !== 'running' && (
                      <button className="btn-ghost" style={{ padding: '6px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--success)' }} title="Start">
                        <Play size={14} />
                      </button>
                    )}
                    {container.state === 'running' && (
                      <button className="btn-ghost" style={{ padding: '6px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--warning)' }} title="Stop">
                        <Square size={14} />
                      </button>
                    )}
                    <button className="btn-ghost" style={{ padding: '6px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--accent)' }} title="Restart">
                      <RotateCcw size={14} />
                    </button>
                    <button className="btn-ghost" style={{ padding: '6px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--danger)' }} title="Remove">
                      <Trash2 size={14} />
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
