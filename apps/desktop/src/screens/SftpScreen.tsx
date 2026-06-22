import { useState } from 'react';
import { Folder, File, ChevronRight, Download, Upload, ArrowLeft } from 'lucide-react';

interface FileItem {
  name: string;
  path: string;
  size: number;
  modified: string;
  isDir: boolean;
  permissions: string;
}

const mockFiles: FileItem[] = [
  { name: '..', path: '/', size: 0, modified: '', isDir: true, permissions: 'drwxr-xr-x' },
  { name: 'etc', path: '/etc', size: 4096, modified: '2024-01-15 10:30', isDir: true, permissions: 'drwxr-xr-x' },
  { name: 'var', path: '/var', size: 4096, modified: '2024-01-15 10:30', isDir: true, permissions: 'drwxr-xr-x' },
  { name: 'usr', path: '/usr', size: 4096, modified: '2024-01-15 10:30', isDir: true, permissions: 'drwxr-xr-x' },
  { name: 'opt', path: '/opt', size: 4096, modified: '2024-01-15 10:30', isDir: true, permissions: 'drwxr-xr-x' },
  { name: 'nginx.conf', path: '/etc/nginx.conf', size: 2847, modified: '2024-01-14 15:22', isDir: false, permissions: '-rw-r--r--' },
  { name: 'docker-compose.yml', path: '/opt/docker-compose.yml', size: 1250, modified: '2024-01-13 09:15', isDir: false, permissions: '-rw-r--r--' },
  { name: 'app.log', path: '/var/log/app.log', size: 15728640, modified: '2024-01-15 11:00', isDir: false, permissions: '-rw-r--r--' },
];

function formatSize(bytes: number): string {
  if (bytes === 0) return '-';
  const units = ['B', 'KB', 'MB', 'GB'];
  let size = bytes;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit++; }
  return `${size.toFixed(1)} ${units[unit]}`;
}

export function SftpScreen() {
  const [currentPath, setCurrentPath] = useState('/');
  const [files] = useState(mockFiles);

  return (
    <div style={{ padding: 24 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 24, fontWeight: 700, margin: 0 }}>File Manager</h1>
          <p style={{ color: 'var(--text-secondary)', margin: '4px 0 0', fontSize: 14 }}>
            SFTP · Prod Web 01
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button className="btn btn-ghost" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Upload size={16} />
            Upload
          </button>
          <button className="btn btn-ghost" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Download size={16} />
            Download
          </button>
        </div>
      </div>

      {/* Breadcrumb */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '8px 16px',
          background: 'var(--bg-secondary)',
          border: '1px solid var(--border)',
          borderRadius: 6,
          marginBottom: 12,
          fontSize: 13,
          fontFamily: 'monospace',
        }}
      >
        <button
          className="btn-ghost"
          style={{ padding: 4, border: 'none', background: 'transparent', cursor: 'pointer' }}
        >
          <ArrowLeft size={14} />
        </button>
        <Folder size={14} color="var(--accent)" />
        <span style={{ color: 'var(--text-primary)' }}>{currentPath}</span>
      </div>

      {/* File list */}
      <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 14 }}>
          <thead>
            <tr style={{ borderBottom: '1px solid var(--border)', background: 'var(--bg-tertiary)' }}>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Name</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Size</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Modified</th>
              <th style={{ padding: '10px 16px', textAlign: 'left', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Permissions</th>
              <th style={{ padding: '10px 16px', textAlign: 'right', fontWeight: 500, color: 'var(--text-secondary)', fontSize: 12 }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {files.map((file) => (
              <tr key={file.path} style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '10px 16px' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: file.isDir ? 'pointer' : 'default' }}>
                    {file.isDir ? (
                      <Folder size={16} color="var(--accent)" />
                    ) : (
                      <File size={16} color="var(--text-muted)" />
                    )}
                    <span style={{ fontWeight: file.isDir ? 500 : 400 }}>{file.name}</span>
                  </div>
                </td>
                <td style={{ padding: '10px 16px', color: 'var(--text-secondary)', fontSize: 13 }}>{formatSize(file.size)}</td>
                <td style={{ padding: '10px 16px', color: 'var(--text-secondary)', fontSize: 13 }}>{file.modified}</td>
                <td style={{ padding: '10px 16px', color: 'var(--text-muted)', fontFamily: 'monospace', fontSize: 12 }}>{file.permissions}</td>
                <td style={{ padding: '10px 16px', textAlign: 'right' }}>
                  {!file.isDir && (
                    <button className="btn-ghost" style={{ padding: '4px 8px', border: 'none', background: 'transparent', cursor: 'pointer', color: 'var(--accent)', fontSize: 12 }}>
                      Edit
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
