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
  inputRef: RefObject<HTMLTextAreaElement | null>;
  onInputChange: (e: React.ChangeEvent<HTMLTextAreaElement>) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
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
  const promptStyle = {
    borderLeftColor: badgeColor,
    '--mode-accent': badgeColor,
  } as React.CSSProperties & { '--mode-accent': string };

  const getPlaceholder = () => {
    if (showModeSelector) return "Gõ để lọc mode hoặc lệnh... (Esc để đóng)";
    if (vibeSession.step === 'mode') return 'Nhập 1 (Code), 2 (Research), 3 (Assistant)...';
    if (vibeSession.step === 'goal') return 'Nhập mục tiêu nhiệm vụ...';
    if (vibeSession.step === 'permit') return 'Nhập y (đồng ý) hoặc n (từ chối)...';
    return "add a new field or task for Custos...";
  };

  const modeLabel = normalizedMode === 'custos' ? 'Custos' : cfg?.mode || currentMode;

  return (
    <div className="opencode-prompt-container">
      <div
        className="gpt-chat-bar opencode-chat-box"
        style={promptStyle}
      >
        <div className="opencode-input-row">
          <textarea
            ref={inputRef}
            className="gpt-chat-input"
            value={inputVal}
            onChange={onInputChange}
            onKeyDown={onKeyDown}
            placeholder={getPlaceholder()}
            autoFocus
            disabled={isProcessing}
            spellCheck={false}
            rows={1}
            aria-label="Nhập tin nhắn"
            style={{ caretColor: badgeColor }}
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
