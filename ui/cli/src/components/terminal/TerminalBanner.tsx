import React from 'react';

const CUSTOS_ASCII_LINES = [
  ' ██████╗██╗   ██╗███████╗████████╗ ██████╗ ███████╗',
  '██╔════╝██║   ██║██╔════╝╚══██╔══╝██╔═══██╗██╔════╝',
  '██║     ██║   ██║███████╗   ██║   ██║   ██║███████╗',
  '██║     ██║   ██║╚════██║   ██║   ██║   ██║╚════██║',
  '╚██████╗╚██████╔╝███████║   ██║   ╚██████╔╝███████║',
  ' ╚═════╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝ ╚══════╝',
];

interface TerminalBannerProps {
  onOpenModeSelector?: () => void;
  onRunCommand?: (command: string) => void;
  showNotice?: boolean;
}

export const TerminalBanner: React.FC<TerminalBannerProps> = ({ showNotice = true }) => {
  return (
    <div className="terminal-hero-banner">
      <div className="banner-content-column">
        <pre className="banner-ascii-art" aria-label="Custos Logo">
          {CUSTOS_ASCII_LINES.map((line, idx) => (
            <span key={idx} className={`ascii-line ascii-line-${idx}`}>
              {line}
              {'\n'}
            </span>
          ))}
        </pre>

        <div className="banner-subtitle-line">
          <span className="banner-version">v0.1.1</span>
          <span className="banner-subtitle">agent workspace có kiểm soát</span>
        </div>

        {showNotice && (
          <div className="banner-initial-notice">
            <div className="notice-line">
              <span className="notice-icon">✔</span> Khởi tạo Worktree Sandbox cô lập (.custos/sandboxes/worktree-live)
            </div>
            <div className="notice-line">
              <span className="notice-icon">✔</span> Phân tích AST & tổng hợp bản vá: +5 additions, -2 deletions
            </div>
            <div className="notice-line">
              <span className="notice-icon">✔</span> Nhiệm vụ hoàn tất an toàn dưới cơ chế Epoch Lock Guard!
            </div>
            <div className="notice-meta">
              Assistant · Claude Opus 4.5 · 1.2s · 42.1 tok/s
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
