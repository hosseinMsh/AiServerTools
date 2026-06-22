import { useState } from 'react';
import { Settings, Plus, Trash2, Globe, Bot, Key, Bell } from 'lucide-react';

interface AiProvider {
  id: string;
  name: string;
  type: string;
  baseUrl: string;
  defaultModel: string;
  enabled: boolean;
}

export function SettingsScreen() {
  const [providers, setProviders] = useState<AiProvider[]>([
    { id: '1', name: 'OpenAI Production', type: 'openai', baseUrl: 'https://api.openai.com/v1', defaultModel: 'gpt-4', enabled: true },
    { id: '2', name: 'Custom LLM', type: 'custom', baseUrl: 'https://llm.internal.company.com/v1', defaultModel: 'llama-3-70b', enabled: false },
  ]);
  const [showAdd, setShowAdd] = useState(false);
  const [newProvider, setNewProvider] = useState({ name: '', baseUrl: '', model: '', apiKey: '' });

  return (
    <div style={{ padding: 24, maxWidth: 800 }}>
      <div style={{ marginBottom: 32 }}>
        <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>Settings</h1>
        <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
          Configure AI providers, security, and preferences
        </p>
      </div>

      {/* AI Providers Section */}
      <div style={{ marginBottom: 32 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Bot size={20} color="var(--accent)" />
            <h2 style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>AI Providers</h2>
          </div>
          <button className="btn btn-primary" style={{ display: 'flex', alignItems: 'center', gap: 8 }} onClick={() => setShowAdd(!showAdd)}>
            <Plus size={16} />
            Add Provider
          </button>
        </div>

        {/* Add provider form */}
        {showAdd && (
          <div className="card" style={{ marginBottom: 16 }}>
            <h3 style={{ margin: '0 0 16px', fontSize: 15, fontWeight: 600 }}>New AI Provider</h3>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4, display: 'block' }}>Provider Name</label>
                <input className="input" placeholder="e.g., Internal LLM" value={newProvider.name} onChange={(e) => setNewProvider({ ...newProvider, name: e.target.value })} />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4, display: 'block' }}>Type</label>
                <select className="input">
                  <option>OpenAI Compatible</option>
                  <option>OpenAI</option>
                  <option>Custom</option>
                </select>
              </div>
              <div style={{ gridColumn: '1 / -1' }}>
                <label style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4, display: 'block' }}>Base URL</label>
                <input className="input" placeholder="https://api.openai.com/v1" value={newProvider.baseUrl} onChange={(e) => setNewProvider({ ...newProvider, baseUrl: e.target.value })} />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4, display: 'block' }}>Default Model</label>
                <input className="input" placeholder="gpt-4" value={newProvider.model} onChange={(e) => setNewProvider({ ...newProvider, model: e.target.value })} />
              </div>
              <div>
                <label style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 4, display: 'block' }}>API Key</label>
                <input className="input" type="password" placeholder="sk-..." value={newProvider.apiKey} onChange={(e) => setNewProvider({ ...newProvider, apiKey: e.target.value })} />
              </div>
            </div>
            <div style={{ marginTop: 16, display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button className="btn btn-ghost" onClick={() => setShowAdd(false)}>Cancel</button>
              <button className="btn btn-primary" onClick={() => {
                setProviders([...providers, { id: String(Date.now()), name: newProvider.name, type: 'custom', baseUrl: newProvider.baseUrl, defaultModel: newProvider.model, enabled: true }]);
                setShowAdd(false);
                setNewProvider({ name: '', baseUrl: '', model: '', apiKey: '' });
              }}>Save</button>
            </div>
          </div>
        )}

        {/* Provider list */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {providers.map((provider) => (
            <div key={provider.id} className="card" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <div style={{ padding: 8, background: provider.enabled ? 'rgba(63, 185, 80, 0.1)' : 'var(--bg-tertiary)', borderRadius: 8 }}>
                  <Globe size={20} color={provider.enabled ? 'var(--success)' : 'var(--text-muted)'} />
                </div>
                <div>
                  <div style={{ fontWeight: 600 }}>{provider.name}</div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', fontFamily: 'monospace' }}>
                    {provider.baseUrl} · {provider.defaultModel}
                  </div>
                </div>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span className={`badge badge-${provider.enabled ? 'success' : 'info'}`}>
                  {provider.enabled ? 'Active' : 'Disabled'}
                </span>
                <button className="btn-ghost" style={{ padding: 6, border: 'none', background: 'transparent', cursor: 'pointer' }}>
                  <Trash2 size={14} color="var(--danger)" />
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Other Settings */}
      <div style={{ marginBottom: 32 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
          <Key size={20} color="var(--warning)" />
          <h2 style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>Security</h2>
        </div>
        <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <div style={{ fontWeight: 500 }}>Auto-lock vault on inactivity</div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Lock security vault after 5 minutes of inactivity</div>
            </div>
            <label style={{ position: 'relative', width: 44, height: 24, cursor: 'pointer' }}>
              <input type="checkbox" defaultChecked style={{ display: 'none' }} />
              <div style={{ width: 44, height: 24, background: 'var(--accent)', borderRadius: 12, position: 'relative', transition: '0.2s' }}>
                <div style={{ width: 20, height: 20, background: '#fff', borderRadius: '50%', position: 'absolute', top: 2, right: 2, transition: '0.2s' }} />
              </div>
            </label>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <div style={{ fontWeight: 500 }}>Audit all SSH commands</div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Log every command executed on remote servers</div>
            </div>
            <input type="checkbox" defaultChecked style={{ accentColor: 'var(--accent)', width: 18, height: 18 }} />
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <div style={{ fontWeight: 500 }}>Require confirmation for destructive commands</div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Warn before rm -rf, docker rm -f, reboot, etc.</div>
            </div>
            <input type="checkbox" defaultChecked style={{ accentColor: 'var(--accent)', width: 18, height: 18 }} />
          </div>
        </div>
      </div>

      {/* Notifications */}
      <div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
          <Bell size={20} color="var(--warning)" />
          <h2 style={{ margin: 0, fontSize: 18, fontWeight: 600 }}>Notifications</h2>
        </div>
        <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <div style={{ fontWeight: 500 }}>Server offline alerts</div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Notify when a server becomes unreachable</div>
            </div>
            <input type="checkbox" defaultChecked style={{ accentColor: 'var(--accent)', width: 18, height: 18 }} />
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <div style={{ fontWeight: 500 }}>Disk space warnings</div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>Alert when disk usage exceeds 85%</div>
            </div>
            <input type="checkbox" defaultChecked style={{ accentColor: 'var(--accent)', width: 18, height: 18 }} />
          </div>
        </div>
      </div>
    </div>
  );
}
