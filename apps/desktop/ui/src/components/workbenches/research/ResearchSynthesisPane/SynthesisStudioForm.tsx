import React from 'react';
import { Code2, GitBranch } from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import type { ExecutionWorkspace } from '@/types/domain';

export interface SynthesisStudioFormProps {
  isComposing: boolean;
  activeProposalStatus?: string;
  formTitle: string;
  formSummary: string;
  selectedWorkspaceId: string;
  targetBranch: string;
  workspaces: ExecutionWorkspace[];
  onTitleChange: (v: string) => void;
  onSummaryChange: (v: string) => void;
  onWorkspaceChange: (v: string) => void;
  onBranchChange: (v: string) => void;
}

export const SynthesisStudioForm: React.FC<SynthesisStudioFormProps> = ({
  isComposing,
  activeProposalStatus,
  formTitle,
  formSummary,
  selectedWorkspaceId,
  targetBranch,
  workspaces,
  onTitleChange,
  onSummaryChange,
  onWorkspaceChange,
  onBranchChange,
}) => {
  const isReadOnly = !isComposing && activeProposalStatus === 'handoff_completed';

  return (
    <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-4">
      <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
        <Code2 size={13} />
        <span>Synthesis Objective & Target Worktree</span>
      </div>

      <div className="space-y-3">
        <div>
          <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
            Proposal Title
          </label>
          <input
            type="text"
            disabled={isReadOnly}
            value={formTitle}
            onChange={(e) => onTitleChange(e.target.value)}
            placeholder="e.g. Implement Genomic Pipeline Transformer from Verified L2 Claims"
            className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
          />
        </div>

        <div>
          <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
            Summary & Scope
          </label>
          <textarea
            rows={3}
            disabled={isReadOnly}
            value={formSummary}
            onChange={(e) => onSummaryChange(e.target.value)}
            placeholder="Provide architecture rationale, invariants to enforce, and handoff goals..."
            className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-2 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60 resize-y"
          />
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Execution Workspace / Worktree
            </label>
            <NativeSelect
              disabled={isReadOnly}
              value={selectedWorkspaceId}
              onChange={(e) => onWorkspaceChange(e.target.value)}
              className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
            >
              <option value="">Choose a Git workspace</option>
              {workspaces
                .filter((ws) => ws.kind.type === 'git' && ws.status === 'ready')
                .map((ws) => (
                  <option key={ws.id} value={ws.id}>
                    {ws.name} ({ws.kind.type}) - {ws.path}
                  </option>
                ))}
            </NativeSelect>
          </div>

          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Target Git Branch
            </label>
            <div className="relative">
              <GitBranch size={13} className="absolute left-2.5 top-2.5 text-[var(--color-fg-muted)]" />
              <input
                type="text"
                disabled={isReadOnly}
                value={targetBranch}
                onChange={(e) => onBranchChange(e.target.value)}
                placeholder="synthesis/handoff-v1"
                className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg pl-8 pr-3 py-1.5 text-xs font-mono text-[var(--color-editor-fg)] focus:outline-none focus:border-[var(--workbench-accent)] disabled:opacity-60"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
