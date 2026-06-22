import { useState } from 'react';
import { Shield, Key, Lock, Unlock, Plus, Eye, EyeOff, Trash2 } from 'lucide-react';

interface VaultItem {
  id: string;
  label: string;
  kind: string;
  createdAt: string;
  tags: string[];
}

const mockEntries: VaultItem[] = [
  { id: '1', label: 'Production DB Password', kind: 'password', createdAt: '2024-01-10', tags: ['prod', 'db'] },
  { id: '2', label: 'GitHub API Token', kind: 'token', createdAt: '2024-01-12', tags: ['ci', 'github'] },
  { id: '3', label: 'AWS Access Key', kind: 'api_key', createdAt: '2024-01-14', tags: ['aws', 'prod'] },
  { id: '4', label: 'Prod Web SSH Key', kind: 'ssh_key', createdAt: '2024-01-15', tags: ['ssh', 'prod'] },
];

export function VaultScreen() {
  const [locked, setLocked] = useState(true);
  const [password, setPassword] = useState('');
  const [entries] = useState(mockEntries);
  const [visibleId, setVisibleId] = useState<string | null>(null);

  if (locked) {
    return (
      <div style={{ padding: 24, display: 'flex', justifyContent: 'center', alignItems: 'center', height: '80vh' }}>
        <div className="card" style={{ width: 400, textAlign: 'center' }}>
          <div style={{ padding: 24, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 16 }}>
            <div style={{ padding: 16, background: 'rgba(88, 166, 255, 0.1)', borderRadius: '50%' }}>
              <Lock size={32} color="var(--accent)" />
            </div>
            <h2 style={{ margin: 0, fontSize: 20, fontWeight: 600 }}>Security Vault Locked</h2>
            <p style={{ color: 'var(--text-secondary)', fontSize: 14, margin: 0 }}>
              Enter your master password to unlock the encrypted vault
            </p>
            <input
              type="password"
              className="input"
              placeholder="Master password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              style={{ textAlign: 'center' }}
              onKeyDown={(e) => e.key === 'Enter' && setLocked(false)}
            />
            <button className="btn btn-primary" onClick={() => setLocked(false)} style={{ width: '100%' }}>
              <Unlock size={16} style={{ marginRight: 8 }} />
              Unlock Vault
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>Security Vault</h1>
          <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
            {entries.length} entries · Encrypted at rest
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button className="btn btn-ghost" onClick={() => setLocked(true)} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Lock size={16} />
            Lock
          </button>
          <button className="btn btn-primary" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Plus size={16} />
            Add Entry
          </button>
        </div>
      </div>

      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 14 }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border)', background: 'var(--bg-tertiary)' }}>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Label</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Type</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Created</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Tags</th>
              <th style={{ padding: '10px 16px', textAlign: 'right', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <tr key={entry.id} style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '12px 16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <Key size={14} color="var(--warning)" />
                    <span style={{ fontWeight: 500 }}>{entry.label}</span>
                  </div>
                </td>
                <td style={{ padding: '12px 16px', color: 'var(--text-secondary)' }}>
                  <span className="badge badge-info">{entry.kind}</span>
                </td>
                <td style={{ padding: '12px 16px', color: 'var(--text-secondary)', fontSize: 13 }}>{entry.createdAt}</td>
                <td style={{ padding: '12px 16px' }}>
                  <div style={{ display: 'flex', gap: 4 }}>
                    {entry.tags.map((tag) => (
                      <span key={tag} className="badge badge-info" style={{ fontSize: 11 }}>{tag}</span>
                    ))}
                  </div>
                </td>
                <td style={{ padding: '12px 16px', textAlign: 'right' }}>
                  <div style={{ display: 'flex', gap: 4, justifyContent: 'flex-end' }}>
                    <button
                      className="btn-ghost"
                      style={{ padding: 6, border: 'none', background: 'transparent', cursor: 'pointer' }}
                      onClick={() => setVisibleId(visibleId === entry.id ? null : entry.id)}
                    >
                      {visibleId === entry.id ? <EyeOff size={14} color="var(--text-muted)" /> : <Eye size={14} color="var(--text-muted)" />}
                    </button>
                    <button className="btn-ghost" style={{ padding: 6, border: 'none', background: 'transparent', cursor: 'pointer' }}>
                      <Trash2 size={14} color="var(--danger)" />
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
