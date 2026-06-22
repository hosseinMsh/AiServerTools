import { useState, useRef, useEffect } from 'react';
import { Bot, Send, Settings2, Sparkles, Terminal, Container, FileCode2, GitBranch } from 'lucide-react';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: string;
  toolCalls?: { tool: string; args: string }[];
}

const intentIcons: Record<string, any> = {
  ssh: Terminal,
  docker: Container,
  sftp: FileCode2,
  tunnel: GitBranch,
};

const welcomeSuggestions = [
  'SSH into prod-web-01 and check disk space',
  'Show running Docker containers on staging',
  'Debug why nginx is failing on prod-web-01',
  'Create a SOCKS5 tunnel on port 1080',
  'Check CPU and memory on all production servers',
];

export function AiChatScreen() {
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: '0',
      role: 'assistant',
      content: "# InfraCore AI Assistant\n\nI'm your DevOps operations copilot. I can help you manage servers, containers, tunnels, and infrastructure.\n\n**Available capabilities:**\n- SSH remote command execution\n- Docker container management\n- SFTP file operations\n- SSH tunneling (SOCKS5, port forwarding)\n- Server monitoring & diagnostics\n- Automation & script generation\n\nHow can I help you today?",
      timestamp: new Date().toISOString(),
    },
  ]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || loading) return;

    const userMsg: ChatMessage = {
      id: String(Date.now()),
      role: 'user',
      content: input,
      timestamp: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, userMsg]);
    setInput('');
    setLoading(true);

    // TODO: Connect to AI engine via Tauri IPC
    setTimeout(() => {
      const assistantMsg: ChatMessage = {
        id: String(Date.now() + 1),
        role: 'assistant',
        content: `I'll help you with: **${input}**\n\nLet me first check the server state...\n\n\`\`\`bash\n# Running diagnostic commands\nuptime\nfree -m\ndf -h\n\`\`\`\n\nHere's what I found — the system is healthy. What specific information do you need?`,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, assistantMsg]);
      setLoading(false);
    }, 1500);
  };

  return (
    <div style={{ display: 'flex', height: '100vh' }}>
      {/* Chat area */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
        {/* Header */}
        <div
          style={{
            padding: '16px 24px',
            borderBottom: '1px solid var(--border)',
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            background: 'var(--bg-secondary)',
          }}
        >
          <div style={{ padding: 8, background: 'rgba(88, 166, 255, 0.1)', borderRadius: 8 }}>
            <Bot size={20} color="var(--accent)" />
          </div>
          <div>
            <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>AI Assistant</h2>
            <p style={{ margin: '2px 0 0', fontSize: 12, color: 'var(--text-secondary)' }}>
              Connected · gpt-4 · Tokens: 1,234
            </p>
          </div>
        </div>

        {/* Messages */}
        <div style={{ flex: 1, overflow: 'auto', padding: 24 }}>
          {messages.map((msg) => (
            <div key={msg.id} style={{ marginBottom: 20 }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: 12,
                  flexDirection: msg.role === 'user' ? 'row-reverse' : 'row',
                }}
              >
                <div
                  style={{
                    width: 32,
                    height: 32,
                    borderRadius: 8,
                    background: msg.role === 'user' ? 'var(--bg-tertiary)' : 'rgba(88, 166, 255, 0.1)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                  }}
                >
                  {msg.role === 'user' ? (
                    <span style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-secondary)' }}>U</span>
                  ) : (
                    <Sparkles size={16} color="var(--accent)" />
                  )}
                </div>
                <div
                  style={{
                    maxWidth: '80%',
                    padding: '12px 16px',
                    borderRadius: 8,
                    background: msg.role === 'user' ? 'var(--accent)' : 'var(--bg-secondary)',
                    color: msg.role === 'user' ? '#fff' : 'var(--text-primary)',
                    border: msg.role === 'user' ? 'none' : '1px solid var(--border)',
                    fontSize: 14,
                    lineHeight: 1.6,
                  }}
                  dangerouslySetInnerHTML={{ __html: msg.content.replace(/\n/g, '<br/>') }}
                />
              </div>
            </div>
          ))}
          {loading && (
            <div style={{ display: 'flex', gap: 8, alignItems: 'center', color: 'var(--text-muted)', fontSize: 14, padding: '12px 0' }}>
              <div style={{ width: 8, height: 8, borderRadius: '50%', background: 'var(--accent)', animation: 'pulse 1s infinite' }} />
              AI is thinking...
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        <div style={{ padding: '16px 24px', borderTop: '1px solid var(--border)', background: 'var(--bg-secondary)' }}>
          <div style={{ display: 'flex', gap: 8 }}>
            <input
              className="input"
              placeholder="Ask me anything about your infrastructure..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSend()}
              style={{ flex: 1 }}
            />
            <button
              className="btn btn-primary"
              onClick={handleSend}
              disabled={loading || !input.trim()}
              style={{ display: 'flex', alignItems: 'center', gap: 8 }}
            >
              <Send size={16} />
              Send
            </button>
          </div>
        </div>
      </div>

      {/* Side panel - Suggestions */}
      <div
        style={{
          width: 280,
          borderLeft: '1px solid var(--border)',
          background: 'var(--bg-secondary)',
          padding: 16,
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
        <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: 1, margin: 0 }}>
          Suggested Actions
        </h3>
        {welcomeSuggestions.map((suggestion, i) => (
          <button
            key={i}
            onClick={() => setInput(suggestion)}
            style={{
              padding: '10px 12px',
              background: 'var(--bg-primary)',
              border: '1px solid var(--border)',
              borderRadius: 8,
              cursor: 'pointer',
              textAlign: 'left',
              fontSize: 13,
              color: 'var(--text-secondary)',
              lineHeight: 1.4,
              transition: 'all 0.15s ease',
            }}
            onMouseEnter={(e) => { e.currentTarget.style.borderColor = 'var(--accent)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
            onMouseLeave={(e) => { e.currentTarget.style.borderColor = 'var(--border)'; e.currentTarget.style.color = 'var(--text-secondary)'; }}
          >
            {suggestion}
          </button>
        ))}
      </div>
    </div>
  );
}
