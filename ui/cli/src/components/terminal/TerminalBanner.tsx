import React from 'react';
import { Sparkles, Terminal } from 'lucide-react';

interface TerminalBannerProps {
  onOpenModeSelector?: () => void;
  onRunCommand?: (command: string) => void;
}

export const TerminalBanner: React.FC<TerminalBannerProps> = ({ onOpenModeSelector, onRunCommand }) => {
  return (
    <div className="terminal-hero-banner">
      <div className="banner-content-column">
        <div className="banner-title-line">
          <span className="banner-mark">C</span>
          <span className="banner-brand-name">custos</span>
          <span className="banner-version">v0.1.1</span>
          <span className="banner-subtitle">Không gian làm việc có kiểm soát</span>
        </div>

        <div className="banner-tagline">
          Bạn muốn bắt đầu việc gì? Chọn một gợi ý hoặc nhập yêu cầu bằng ngôn ngữ tự nhiên.
        </div>

        <div className="banner-quick-actions" aria-label="Gợi ý bắt đầu nhanh">
          <button type="button" onClick={() => onRunCommand?.('create Viết mục tiêu của bạn ở đây')}>Tạo nhiệm vụ</button>
          <button type="button" onClick={() => onRunCommand?.('list')}>Xem nhiệm vụ</button>
          <button type="button" onClick={() => onRunCommand?.('status')}>Kiểm tra hệ thống</button>
          <button
            type="button"
            className="banner-mode-badge-btn"
            onClick={onOpenModeSelector}
            title="Mở bảng lệnh (/)"
          >
            <Terminal size={12} />
            <span>/</span>
            <span className="btn-text">Tất cả lệnh</span>
          </button>
        </div>

        <div className="banner-keyboard-hint">
          <Sparkles size={13} className="hint-icon" />
          Mẹo: nhấn <kbd>/</kbd> để mở lệnh nhanh, <kbd>↑</kbd> để gọi lại lệnh trước.
        </div>
      </div>
    </div>
  );
};
