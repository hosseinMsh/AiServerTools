import { useState } from 'react';
import { ScrollText, Search, Filter } from 'lucide-react';

interface AuditEvent {
  id: string;
  timestamp: string;
  eventType: string;
  action: string;
  resource: string;
  result: string;
  userId: string;
}

const mockEvents: AuditEvent[] = Array.from({ length: 50 }, (_, i) => ({
  id: String(i),
  timestamp: new Date(Date.now() - i * 60000).toISOString(),
  eventType: ['ssh_connect', 'ssh_command', 'docker_exec', 'sftp_upload', 'tunnel_start', 'config_update'][i % 6],
  action: ['SSH Connect', 'Execute Command', 'Docker Exec', 'SFTP Upload', 'Tunnel Start', 'Config Update'][i % 6],
  resource: ['server:prod-web-01', 'server:staging-db', 'docker:web-app', '/etc/nginx.conf', 'tunnel:1080', 'config:providers'][i % 6],
  result: ['success', 'success', 'success', 'success', 'success', 'denied'][i % 6],
  userId: ['admin@company.com', 'devops@company.com', 'deploy-bot'][i % 3],
}));

export function AuditLogScreen() {
  const [events] = useState(mockEvents);
  const [filter, setFilter] = useState('');

  const filtered = filter
    ? events.filter(
        (e) =>
          e.action.toLowerCase().includes(filter.toLowerCase()) ||
          e.resource.toLowerCase().includes(filter.toLowerCase()) ||
          e.userId.toLowerCase().includes(filter.toLowerCase())
      )
    : events;

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>Audit Log</h1>
          <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
            {events.length} events · All actions recorded
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div style={{ position: 'relative' }}>
            <Search size={14} style={{ position: 'absolute', left: 10, top: '50%', transform: 'translateY(-50%)', color: 'var(--text-muted)' }} />
            <input className="input" style={{ paddingLeft: 32, width: 240 }} placeholder="Search events..." value={filter} onChange={(e) => setFilter(e.target.value)} />
          </div>
          <button className="btn btn-ghost" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Filter size={16} />
            Filter
          </button>
        </div>
      </div>

      <div className="card" style={{ padding: 0, overflow: 'auto', maxHeight: 'calc(100vh - 200px)' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
          <thead style={{ position: 'sticky', top: 0, background: 'var(--bg-tertiary)' }}>
            <tr style={{ borderBottom: '1px solid var(--border)' }}>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Timestamp</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>User</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Action</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Resource</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Result</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((event) => (
              <tr key={event.id} style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '10px 16px', color: 'var(--text-secondary)', fontFamily: 'monospace', fontSize: 12, whiteSpace: 'nowrap' }}>
                  {new Date(event.timestamp).toLocaleString()}
                </td>
                <td style={{ padding: '10px 16px', color: 'var(--text-secondary)' }}>
                  {event.userId}
                </td>
                <td style={{ padding: '10px 16px', fontWeight: 500 }}>{event.action}</td>
                <td style={{ padding: '10px 16px', fontFamily: 'monospace', fontSize: 12, color: 'var(--text-secondary)' }}>
                  {event.resource}
                </td>
                <td style={{ padding: '10px 16px' }}>
                  <span className={`badge badge-${event.result === 'success' ? 'success' : 'danger'}`}>
                    {event.result}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
