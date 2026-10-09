import React from 'react';
import { TerminalLine, OperationalMode } from '../../types';
import { SAMPLE_DIFF } from '../../data/constants';
import { DiffViewer } from '../DiffViewer';
import { TerminalBanner } from './TerminalBanner';
import { TerminalModeCard } from './TerminalModeCard';

interface TerminalLineItemProps {
  line: TerminalLine;
  currentMode: OperationalMode;
  isPermitPending: boolean;
  onOpenModeSelector: () => void;
  onRunCommand: (command: string) => void;
  onPermitDecision: (approved: boolean) => void;
}

export const TerminalLineItem: React.FC<TerminalLineItemProps> = ({
  line,
  currentMode,
  isPermitPending,
  onOpenModeSelector,
  onRunCommand,
  onPermitDecision,
}) => {
  if (line.type === 'banner') {
    return <TerminalBanner key={line.id} onOpenModeSelector={onOpenModeSelector} onRunCommand={onRunCommand} />;
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

  return (
    <div key={line.id} className={`terminal-output-line line-${line.type}`}>
      <span className="line-content">{line.content}</span>

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
