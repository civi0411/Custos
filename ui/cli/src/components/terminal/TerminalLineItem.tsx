import React from 'react';
import { TerminalLine, OperationalMode } from '../../types';
import { SAMPLE_DIFF } from '../../data/constants';
import { DiffViewer } from '../DiffViewer';
import { TerminalBanner } from './TerminalBanner';
import { TerminalModeCard } from './TerminalModeCard';
import { AVAILABLE_MODES, getModeSlug } from './types';

interface TerminalLineItemProps {
  line: TerminalLine;
  currentMode: OperationalMode;
  isPermitPending: boolean;
  hasChatted?: boolean;
  onOpenModeSelector: () => void;
  onRunCommand: (command: string) => void;
  onPermitDecision: (approved: boolean) => void;
}

export const TerminalLineItem: React.FC<TerminalLineItemProps> = ({
  line,
  currentMode,
  isPermitPending,
  hasChatted = false,
  onOpenModeSelector,
  onRunCommand,
  onPermitDecision,
}) => {
  if (line.type === 'banner') {
    return (
      <TerminalBanner
        key={line.id}
        showNotice={!hasChatted}
        onOpenModeSelector={onOpenModeSelector}
        onRunCommand={onRunCommand}
      />
    );
  }

  if (line.type === 'card') {
    const mode = (line.metadata?.mode || currentMode) as OperationalMode;
    return <TerminalModeCard key={line.id} mode={mode} />;
  }

  if (line.type === 'diff') {
    return (
      <div key={line.id} className="terminal-diff-block">
        <DiffViewer
          filePath={line.metadata?.filePath || SAMPLE_DIFF.filePath}
          oldCode={SAMPLE_DIFF.oldCode}
          newCode={SAMPLE_DIFF.newCode}
          compact
        />
      </div>
    );
  }

  if (line.type === 'input') {
    const modeSlug = getModeSlug(currentMode);
    const cfg = AVAILABLE_MODES.find((m) => m.slug === modeSlug);
    const badgeColor = cfg ? cfg.color : '#38bdf8';
    const cleanPrompt = line.content.replace(/^[a-zA-Z-]+\s*❯\s*/, '');
    return (
      <div key={line.id} className="opencode-user-card" style={{ borderLeftColor: badgeColor }}>
        <div className="opencode-user-text">{cleanPrompt}</div>
      </div>
    );
  }

  return (
    <div key={line.id} className={`opencode-assistant-msg line-${line.type}`}>
      <div className="assistant-msg-content">{line.content}</div>

      {line.metadata?.showControls && isPermitPending && (
        <div className="inline-permit-controls">
          <button className="inline-approve-btn" onClick={() => onPermitDecision(true)}>
            [ Y ] Approve Permit
          </button>
          <button className="inline-reject-btn" onClick={() => onPermitDecision(false)}>
            [ N ] Reject Permit
          </button>
        </div>
      )}
    </div>
  );
};
