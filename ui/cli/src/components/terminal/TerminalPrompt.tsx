import React, { RefObject } from 'react';
import { OperationalMode } from '../../types';
import { AVAILABLE_MODES, getModeSlug, VibeSessionState } from './types';
import { Plus, ChevronDown, Mic, ArrowUp } from 'lucide-react';

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
  const isSpecialized = normalizedMode !== 'standard' && normalizedMode !== 'custos';
  const modeSlug = getModeSlug(currentMode as OperationalMode);
  const cfg = AVAILABLE_MODES.find((m) => m.slug === modeSlug);
  const badgeColor = cfg ? cfg.color : '#38bdf8';

  const getPlaceholder = () => {
    if (showModeSelector) return "Gõ để lọc mode hoặc lệnh... (Esc để đóng)";
    if (vibeSession.step === 'mode') return 'Nhập 1 (Code), 2 (Research), 3 (Assistant)...';
    if (vibeSession.step === 'goal') return 'Nhập mục tiêu nhiệm vụ...';
    if (vibeSession.step === 'permit') return 'Nhập y (đồng ý) hoặc n (từ chối)...';
    if (!isSpecialized) {
      return "Hỏi bất cứ điều gì, hoặc gõ '/' để chọn mode...";
    }
    return `[${modeSlug}] Nhập yêu cầu, câu hỏi hoặc gõ '/' để đổi mode...`;
  };

  return (
    <div className="gpt-chat-bar">
      <button
        type="button"
        className="gpt-plus-btn"
        onClick={onToggleModeSelector}
        title="Chọn mode hoạt động hoặc công cụ (/)"
        aria-label="Thêm công cụ"
      >
        <Plus size={18} />
      </button>

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

      <div className="gpt-chat-actions">
        <button
          type="button"
          className="gpt-mode-pill"
          onClick={onToggleModeSelector}
          title="Chọn mode (/)"
          style={{
            color: badgeColor,
            borderColor: `${badgeColor}35`,
            backgroundColor: `${badgeColor}12`,
          }}
        >
          <span>{modeSlug}</span>
          <ChevronDown size={14} />
        </button>

        <button
          type="button"
          className="gpt-mic-btn"
          onClick={onToggleModeSelector}
          title="Lệnh nhanh (/)"
          aria-label="Lệnh nhanh"
        >
          <Mic size={16} />
        </button>

        <button
          type="button"
          className={`gpt-send-btn ${inputVal.trim() ? 'active' : ''}`}
          onClick={onSend}
          disabled={isProcessing || !inputVal.trim()}
          title="Gửi (Enter)"
          aria-label="Gửi yêu cầu"
        >
          <ArrowUp size={16} />
        </button>
      </div>
    </div>
  );
};
