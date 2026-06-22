import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Layout } from './components/common/Layout';
import { ServerList } from './components/dashboard/ServerList';
import { TerminalScreen } from './screens/TerminalScreen';
import { DockerScreen } from './screens/DockerScreen';
import { SftpScreen } from './screens/SftpScreen';
import { TunnelScreen } from './screens/TunnelScreen';
import { AiChatScreen } from './screens/AiChatScreen';
import { VaultScreen } from './screens/VaultScreen';
import { AuditLogScreen } from './screens/AuditLogScreen';
import { SettingsScreen } from './screens/SettingsScreen';
import { useAppStore } from './stores/appStore';

export function App() {
  const theme = useAppStore((s) => s.theme);

  return (
    <div className={theme === 'dark' ? 'dark' : ''}>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<Navigate to="/dashboard" replace />} />
            <Route path="/dashboard" element={<ServerList />} />
            <Route path="/terminal" element={<TerminalScreen />} />
            <Route path="/docker" element={<DockerScreen />} />
            <Route path="/sftp" element={<SftpScreen />} />
            <Route path="/tunnels" element={<TunnelScreen />} />
            <Route path="/ai" element={<AiChatScreen />} />
            <Route path="/vault" element={<VaultScreen />} />
            <Route path="/audit" element={<AuditLogScreen />} />
            <Route path="/settings" element={<SettingsScreen />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </div>
  );
}
