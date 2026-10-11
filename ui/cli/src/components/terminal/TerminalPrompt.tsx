import React, { RefObject } from 'react';
import { OperationalMode } from '../../types';
import { AVAILABLE_MODES, getModeSlug, VibeSessionState } from './types';
import { Plus, ArrowUp } from 'lucide-react';

interface TerminalPromptProps {
  currentMode: OperationalMode | string;
  inputVal: string;
  isProcessing: boolean;
  showModeSelector: boolean;
  vibeSession: VibeSessionState;
  inputRef: RefObject<HTMLInputElement | null>;
  onInputChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  onToggleModeSelector: () => void;
  onSend: () => void;
}

export const TerminalPrompt: React.FC<TerminalPromptProps> = ({
  currentMode,
  inputVal,
  isProcessing,
  showModeSelector,
  vibeSession,
  inputRef,
  onInputChange,
  onKeyDown,
  onToggleModeSelector,
  onSend,
}) => {
  const normalizedMode = String(currentMode).toLowerCase();
  const modeSlug = getModeSlug(currentMode as OperationalMode);
  const cfg = AVAILABLE_MODES.find((m) => m.slug === modeSlug);
  const badgeColor = cfg ? cfg.color : '#60a5fa';

  const getPlaceholder = () => {
    if (showModeSelector) return "Gõ để lọc mode hoặc lệnh... (Esc để đóng)";
    if (vibeSession.step === 'mode') return 'Nhập 1 (Code), 2 (Research), 3 (Assistant)...';
    if (vibeSession.step === 'goal') return 'Nhập mục tiêu nhiệm vụ...';
    if (vibeSession.step === 'permit') return 'Nhập y (đồng ý) hoặc n (từ chối)...';
    return "add a new field or task for Custos...";
  };

  const dynamicWidth = Math.max(540, Math.min(window.innerWidth - 60, inputVal.length * 9.5 + 85));
  const modeLabel = normalizedMode === 'custos' ? 'Build' : modeSlug.charAt(0).toUpperCase() + modeSlug.slice(1);

  return (
    <div
      className="opencode-prompt-container"
      style={{
        width: `${dynamicWidth}px`,
        maxWidth: 'calc(100vw - 48px)',
      }}
    >
      <div
        className="gpt-chat-bar opencode-chat-box"
        style={{
          borderLeftColor: badgeColor,
        }}
      >
        <div className="opencode-input-row">
          <input
            ref={inputRef}
            type="text"
            className="gpt-chat-input"
            value={inputVal}
            onChange={onInputChange}
            onKeyDown={onKeyDown}
            placeholder={getPlaceholder()}
            autoFocus
            disabled={isProcessing}
            spellCheck={false}
          />

          <div className="opencode-action-btns">
            <button
              type="button"
              className="opencode-tool-btn"
              onClick={onToggleModeSelector}
              title="Thêm công cụ hoặc lệnh (/)"
              aria-label="Thêm công cụ"
            >
              <Plus size={15} />
            </button>

            <button
              type="button"
              className={`opencode-send-btn ${inputVal.trim() ? 'active' : ''}`}
              onClick={onSend}
              disabled={isProcessing || !inputVal.trim()}
              title="Gửi (Enter)"
              aria-label="Gửi yêu cầu"
            >
              <ArrowUp size={15} />
            </button>
          </div>
        </div>

        <div className="opencode-status-row">
          <button
            type="button"
            className="opencode-mode-badge"
            onClick={onToggleModeSelector}
            title="Chọn mode (Tab)"
            style={{ color: badgeColor }}
          >
            {modeLabel}
          </button>
          <span className="opencode-model-tag">Claude Opus 4.5</span>
          <span className="opencode-engine-tag">OpenCode Zen</span>
        </div>
      </div>

      <div className="focus-shortcuts" aria-label="Phím tắt và điều hướng">
        <button type="button" onClick={onToggleModeSelector}>
          <kbd>tab</kbd> switch agent
        </button>
        <button type="button" onClick={onToggleModeSelector}>
          <kbd>ctrl+p</kbd> commands
        </button>
      </div>
    </div>
  );
};
