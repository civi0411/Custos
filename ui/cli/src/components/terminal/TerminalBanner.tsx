import React from 'react';
import { Sparkles, Terminal } from 'lucide-react';

interface TerminalBannerProps {
  onOpenModeSelector?: () => void;
}

export const TerminalBanner: React.FC<TerminalBannerProps> = ({ onOpenModeSelector }) => {
  return (
    <div className="terminal-hero-banner">
      <div className="banner-content-column">
        <div className="banner-ascii-logo">
          <pre>
{`   ██████╗ ██╗   ██╗ ███████╗ ████████╗  ██████╗  ███████╗
  ██╔════╝ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔═══██╗ ██╔════╝
  ██║      ██║   ██║ ███████╗    ██║    ██║   ██║ ███████╗
  ██║      ██║   ██║ ╚════██║    ██║    ██║   ██║ ╚════██║
  ╚██████╗ ╚██████╔╝ ███████║    ██║    ╚██████╔╝ ███████║
   ╚═════╝  ╚═════╝  ╚══════╝    ╚═╝     ╚═════╝  ╚══════╝`}
          </pre>
        </div>

        <div className="banner-title-line">
          <span className="banner-snow">❄</span>
          <span className="banner-brand-name">Custos</span>
          <span className="banner-version">v0.1.1</span>
          <span className="banner-subtitle">— Guardian of Agentic Work</span>
        </div>

        <div className="banner-tagline">
          Human-governed runtime for specialized agentic workflows
        </div>

        <div className="banner-action-row">
          <span className="banner-chat-hint">
            <Sparkles size={13} className="hint-icon" />
            Chat bình thường hoặc nhấn
          </span>
          <button
            type="button"
            className="banner-mode-badge-btn"
            onClick={onOpenModeSelector}
            title="Nhấn để chọn mode (/)"
          >
            <Terminal size={12} />
            <span>/</span>
            <span className="btn-text">để chọn mode (custos-code, custos-research, custos-assistant)</span>
          </button>
        </div>
      </div>
    </div>
  );
};
