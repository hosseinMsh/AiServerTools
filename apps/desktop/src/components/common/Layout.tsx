import { Outlet, NavLink, useLocation } from 'react-router-dom';
import { useAppStore } from '../../stores/appStore';
import {
  Terminal, Server, Container, FileCode2, GitBranch, Bot,
  Shield, ScrollText, Settings, ChevronLeft, ChevronRight, Activity
} from 'lucide-react';

const navItems = [
  { path: '/dashboard', label: 'Servers', icon: Server },
  { path: '/terminal', label: 'Terminal', icon: Terminal },
  { path: '/docker', label: 'Docker', icon: Container },
  { path: '/sftp', label: 'Files', icon: FileCode2 },
  { path: '/tunnels', label: 'Tunnels', icon: GitBranch },
  { path: '/ai', label: 'AI Assistant', icon: Bot },
  { path: '/vault', label: 'Vault', icon: Shield },
  { path: '/audit', label: 'Audit Log', icon: ScrollText },
];

export function Layout() {
  const sidebarCollapsed = useAppStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const location = useLocation();

  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      {/* Sidebar */}
      <aside
        style={{
          width: sidebarCollapsed ? 56 : 240,
          background: 'var(--bg-secondary)',
          borderRight: '1px solid var(--border)',
          display: 'flex',
          flexDirection: 'column',
          transition: 'width 0.2s ease',
          flexShrink: 0,
        }}
      >
        {/* Logo / Brand */}
        <div
          style={{
            padding: '16px',
            borderBottom: '1px solid var(--border)',
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            justifyContent: sidebarCollapsed ? 'center' : 'flex-start',
          }}
        >
          <Activity size={24} color="var(--accent)" />
          {!sidebarCollapsed && (
            <span style={{ fontWeight: 700, fontSize: 16, color: 'var(--text-primary)' }}>
              InfraCore
            </span>
          )}
        </div>

        {/* Navigation */}
        <nav style={{ flex: 1, padding: '8px', display: 'flex', flexDirection: 'column', gap: 2 }}>
          {navItems.map((item) => {
            const isActive = location.pathname === item.path;
            return (
              <NavLink
                key={item.path}
                to={item.path}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 12,
                  padding: '10px 12px',
                  borderRadius: 6,
                  textDecoration: 'none',
                  color: isActive ? 'var(--accent)' : 'var(--text-secondary)',
                  background: isActive ? 'rgba(88, 166, 255, 0.1)' : 'transparent',
                  justifyContent: sidebarCollapsed ? 'center' : 'flex-start',
                  fontSize: 14,
                  fontWeight: isActive ? 500 : 400,
                }}
              >
                <item.icon size={20} />
                {!sidebarCollapsed && <span>{item.label}</span>}
              </NavLink>
            );
          })}
        </nav>

        {/* Settings */}
        <div style={{ padding: '8px', borderTop: '1px solid var(--border)' }}>
          <NavLink
            to="/settings"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '10px 12px',
              borderRadius: 6,
              textDecoration: 'none',
              color: location.pathname === '/settings' ? 'var(--accent)' : 'var(--text-secondary)',
              background: location.pathname === '/settings' ? 'rgba(88, 166, 255, 0.1)' : 'transparent',
              justifyContent: sidebarCollapsed ? 'center' : 'flex-start',
              fontSize: 14,
            }}
          >
            <Settings size={20} />
            {!sidebarCollapsed && <span>Settings</span>}
          </NavLink>
        </div>

        {/* Collapse toggle */}
        <button
          onClick={toggleSidebar}
          style={{
            padding: '8px',
            border: 'none',
            background: 'transparent',
            color: 'var(--text-muted)',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderTop: '1px solid var(--border)',
          }}
        >
          {sidebarCollapsed ? <ChevronRight size={16} /> : <ChevronLeft size={16} />}
        </button>
      </aside>

      {/* Main content */}
      <main style={{ flex: 1, overflow: 'auto', background: 'var(--bg-primary)' }}>
        <Outlet />
      </main>
    </div>
  );
}
