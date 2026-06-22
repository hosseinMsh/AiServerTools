import { useEffect, useRef, useState } from 'react';
import { useAppStore } from '../stores/appStore';
import { Terminal, Plus, X } from 'lucide-react';

interface TerminalTab {
  id: string;
  label: string;
  serverId: string;
}

export function TerminalScreen() {
  const [tabs, setTabs] = useState<TerminalTab[]>([
    { id: '1', label: 'Prod Web 01', serverId: '1' },
    { id: '2', label: 'Staging DB', serverId: '3' },
  ]);
  const [activeTab, setActiveTab] = useState('1');
  const terminalRef = useRef<HTMLDivElement>(null);
  const servers = useAppStore((s) => s.servers);

  useEffect(() => {
    // In production: initialize xterm.js and connect to SSH session
    if (terminalRef.current && activeTab) {
      // xterm.js would be initialized here
      console.log('Initializing terminal for tab:', activeTab);
    }
  }, [activeTab]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      {/* Terminal tabs */}
      <div
        style={{
          display: 'flex',
          background: 'var(--bg-secondary)',
          borderBottom: '1px solid var(--border)',
          overflow: 'auto',
          flexShrink: 0,
        }}
      >
        {tabs.map((tab) => (
          <div
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '8px 16px',
              cursor: 'pointer',
              borderRight: '1px solid var(--border)',
              background: activeTab === tab.id ? 'var(--bg-primary)' : 'transparent',
              borderBottom: activeTab === tab.id ? '2px solid var(--accent)' : '2px solid transparent',
              fontSize: 13,
              color: activeTab === tab.id ? 'var(--text-primary)' : 'var(--text-secondary)',
              whiteSpace: 'nowrap',
            }}
          >
            <Terminal size={14} />
            <span>{tab.label}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                setTabs((prev) => prev.filter((t) => t.id !== tab.id));
              }}
              style={{
                border: 'none',
                background: 'transparent',
                color: 'var(--text-muted)',
                cursor: 'pointer',
                padding: 2,
                borderRadius: 4,
              }}
            >
              <X size={12} />
            </button>
          </div>
        ))}
        <button
          onClick={() => {
            const newTab = {
              id: String(Date.now()),
              label: `New Session`,
              serverId: '',
            };
            setTabs((prev) => [...prev, newTab]);
            setActiveTab(newTab.id);
          }}
          style={{
            display: 'flex',
            alignItems: 'center',
            padding: '8px 16px',
            border: 'none',
            background: 'transparent',
            color: 'var(--text-muted)',
            cursor: 'pointer',
          }}
        >
          <Plus size={16} />
        </button>
      </div>

      {/* Terminal content */}
      <div
        ref={terminalRef}
        style={{
          flex: 1,
          background: 'var(--terminal-bg)',
          color: 'var(--terminal-fg)',
          fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
          fontSize: 14,
          lineHeight: 1.5,
          padding: 16,
          overflow: 'auto',
        }}
      >
        <div style={{ color: 'var(--text-muted)', fontStyle: 'italic' }}>
          {activeTab
            ? `Connected to ${tabs.find((t) => t.id === activeTab)?.label}`
            : 'Select or create a terminal session'}
        </div>
        <div style={{ marginTop: 16, color: 'var(--success)' }}>
          $ <span style={{ color: 'var(--terminal-fg)' }}>_</span>
        </div>
      </div>

      {/* Status bar */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          padding: '4px 16px',
          background: 'var(--bg-tertiary)',
          borderTop: '1px solid var(--border)',
          fontSize: 12,
          color: 'var(--text-muted)',
          flexShrink: 0,
        }}
      >
        <span>SSH: Connected</span>
        <span>infra-core@1.0.0</span>
      </div>
    </div>
  );
}
