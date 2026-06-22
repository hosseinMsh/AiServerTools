import { useState } from 'react';
import { GitBranch, Plus, StopCircle, Play, Globe } from 'lucide-react';

interface TunnelItem {
  id: string;
  type: 'SOCKS5' | 'Local' | 'Remote';
  bindAddr: string;
  bindPort: number;
  target: string;
  status: 'active' | 'stopped' | 'error';
  session: string;
  bytesIn: number;
  bytesOut: number;
}

const mockTunnels: TunnelItem[] = [
  { id: '1', type: 'SOCKS5', bindAddr: '127.0.0.1', bindPort: 1080, target: 'Dynamic', status: 'active', session: 'Prod Web 01', bytesIn: 1452000, bytesOut: 892000 },
  { id: '2', type: 'Local', bindAddr: '127.0.0.1', bindPort: 3306, target: 'staging-db:3306', status: 'active', session: 'Staging DB', bytesIn: 45200, bytesOut: 12800 },
  { id: '3', type: 'Remote', bindAddr: '0.0.0.0', bindPort: 8080, target: 'localhost:3000', status: 'stopped', session: 'Dev API', bytesIn: 0, bytesOut: 0 },
];

export function TunnelScreen() {
  const [tunnels] = useState(mockTunnels);
  const [showCreate, setShowCreate] = useState(false);

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>SSH Tunnels</h1>
          <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
            {tunnels.filter((t) => t.status === 'active').length} active tunnels
          </p>
        </div>
        <button className="btn btn-primary" style={{ display: 'flex', alignItems: 'center', gap: 8 }} onClick={() => setShowCreate(!showCreate)}>
          <Plus size={16} />
          New Tunnel
        </button>
      </div>

      {/* Create tunnel form */}
      {showCreate && (
        <div className="card" style={{ marginBottom: 24 }}>
          <h3 style={{ margin: '0 0 16px', fontSize: 16, fontWeight: 600 }}>Create Tunnel</h3>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
            <select className="input">
              <option>SOCKS5 Proxy (-D)</option>
              <option>Local Forward (-L)</option>
              <option>Remote Forward (-R)</option>
            </select>
            <input className="input" placeholder="Bind address (e.g., 127.0.0.1)" defaultValue="127.0.0.1" />
            <input className="input" type="number" placeholder="Bind port" defaultValue={1080} />
          </div>
          <div style={{ marginTop: 12 }}>
            <input className="input" placeholder="Target (e.g., localhost:3000) — optional for SOCKS5" />
          </div>
          <div style={{ marginTop: 12, display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
            <button className="btn btn-ghost" onClick={() => setShowCreate(false)}>Cancel</button>
            <button className="btn btn-primary">Start Tunnel</button>
          </div>
        </div>
      )}

      {/* Tunnel list */}
      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 14 }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border)', background: 'var(--bg-tertiary)' }}>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Status</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Type</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Bind</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Target</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Session</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Traffic</th>
              <th style={{ padding: '10px 16px', textAlign: 'right', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {tunnels.map((tunnel) => (
              <tr key={tunnel.id} style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '12px 16px' }}>
                  <span className={`badge badge-${tunnel.status === 'active' ? 'success' : tunnel.status === 'error' ? 'danger' : 'info'}`}>
                    {tunnel.status}
                  </span>
                </td>
                <td style={{ padding: '12px 16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <GitBranch size={14} color={tunnel.type === 'SOCKS5' ? 'var(--accent)' : 'var(--text-secondary)'} />
                    <span>{tunnel.type}</span>
                  </div>
                </td>
                <td style={{ padding: '12px 16px', fontFamily: 'monospace', fontSize: 13 }}>
                  {tunnel.bindAddr}:{tunnel.bindPort}
                </td>
                <td style={{ padding: '12px 16px', fontFamily: 'monospace', fontSize: 13, color: 'var(--text-secondary)' }}>
                  {tunnel.target}
                </td>
                <td style={{ padding: '12px 16px', color: 'var(--text-secondary)' }}>{tunnel.session}</td>
                <td style={{ padding: '12px 16px', fontSize: 13, color: 'var(--text-secondary)' }}>
                  ↓ {(tunnel.bytesIn / 1024 / 1024).toFixed(1)}MB ↑ {(tunnel.bytesOut / 1024 / 1024).toFixed(1)}MB
                </td>
                <td style={{ padding: '12px 16px', textAlign: 'right' }}>
                  <div style={{ display: 'flex', gap: 4, justifyContent: 'flex-end' }}>
                    {tunnel.status === 'stopped' ? (
                      <button className="btn-ghost" style={{ padding: '6px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--success)' }}>
                        <Play size={14} />
                      </button>
                    ) : (
                      <button className="btn-ghost" style={{ padding: '6px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--danger)' }}>
                        <StopCircle size={14} />
                      </button>
                    )}
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
