import React, { useEffect } from 'react';
import { ExecutionPermit } from '../types';
import { RISK_BADGES } from '../data/constants';
import { DiffViewer } from './DiffViewer';
import { ShieldAlert, Check, X, AlertTriangle } from 'lucide-react';

interface ExecutionPermitModalProps {
  permit: ExecutionPermit;
  onApprove: (permitId: string) => void;
  onReject: (permitId: string) => void;
}

export const ExecutionPermitModal: React.FC<ExecutionPermitModalProps> = ({
  permit,
  onApprove,
  onReject,
}) => {
  const riskBadge = RISK_BADGES[permit.risk] || RISK_BADGES.Medium;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }
      if (e.key === 'y' || e.key === 'Y' || e.key === 'Enter') {
        e.preventDefault();
        onApprove(permit.id);
      } else if (e.key === 'n' || e.key === 'N' || e.key === 'Escape') {
        e.preventDefault();
        onReject(permit.id);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [permit.id, onApprove, onReject]);

  return (
    <div className="permit-modal-overlay">
      <div className="permit-modal-container">
        <div className="permit-modal-header">
          <div className="permit-header-left">
            <div className="permit-icon-pulse">
              <ShieldAlert size={20} className="shield-alert-icon" />
            </div>
            <div>
              <div className="permit-meta-line">
                <span
                  className="risk-badge"
                  style={{
                    backgroundColor: riskBadge.bg,
                    color: riskBadge.color,
                    borderColor: riskBadge.border,
                  }}
                >
                  {riskBadge.text}
                </span>
                <span className="permit-action-type">{permit.actionType}</span>
              </div>
              <h3 className="permit-target-title">Target: {permit.target}</h3>
            </div>
          </div>
        </div>

        <div className="permit-modal-body">
          <div className="permit-notice-box">
            <AlertTriangle size={16} className="warning-icon" />
            <div>
              <strong>Human-in-the-Loop Governance:</strong> The autonomous agent has synthesized
              code changes in the sandbox worktree. Production files will not be touched unless you grant
              this ExecutionPermit.
            </div>
          </div>

          <div className="permit-diff-section">
            <div className="section-label">Proposed Worktree Modification:</div>
            <DiffViewer
              filePath={permit.filePath}
              oldCode={permit.oldCode}
              newCode={permit.newCode}
              compact
            />
          </div>
        </div>

        <div className="permit-modal-footer">
          <div className="keyboard-hints">
            <span>Press <kbd>Y</kbd> or <kbd>Enter</kbd> to Approve</span>
            <span>Press <kbd>N</kbd> or <kbd>Esc</kbd> to Reject</span>
          </div>

          <div className="permit-btn-group">
            <button
              className="permit-reject-btn"
              onClick={() => onReject(permit.id)}
              title="Reject this permit and keep worktree untouched"
            >
              <X size={16} />
              <span>Reject Permit (N)</span>
            </button>

            <button
              className="permit-approve-btn"
              onClick={() => onApprove(permit.id)}
              title="Grant permit to isolate and patch worktree"
            >
              <Check size={16} />
              <span>Approve Permit (Y)</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
