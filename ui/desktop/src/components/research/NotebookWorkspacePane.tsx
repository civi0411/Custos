import React from 'react';
import { FileCode, ShieldCheck, ServerOff } from 'lucide-react';

interface NotebookWorkspacePaneProps {
  onAskAgent?: (draftText: string) => void;
  onShowToast?: (msg: string) => void;
}

export const NotebookWorkspacePane: React.FC<NotebookWorkspacePaneProps> = ({
  onShowToast,
}) => {
  const explainUnavailable = () => {
    onShowToast?.(
      'Notebook execution requires a connected, authorized kernel. Custos will not simulate a run.'
    );
  };

  return (
    <div className="flex h-full w-full flex-col overflow-hidden bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      <header className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4">
        <div className="flex items-center gap-2">
          <FileCode className="h-4 w-4 workbench-accent" />
          <span className="text-xs font-semibold">Notebook & compute</span>
        </div>
        <span className="workbench-kicker">Research</span>
      </header>

      <div className="flex flex-1 items-center justify-center p-8">
        <div className="max-w-md text-center">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)]">
            <ServerOff className="h-5 w-5 text-[var(--color-fg-muted)]" />
          </div>
          <h2 className="text-sm font-semibold">No authorized kernel</h2>
          <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">
            Connect a local, SSH, Slurm, or managed compute profile before running code. A
            kernel launch must record its environment, inputs, outputs, cost, and execution
            receipt; opening this pane never creates a run.
          </p>
          <button
            type="button"
            onClick={explainUnavailable}
            className="mt-5 inline-flex items-center gap-2 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-2 text-xs text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)]"
          >
            <ShieldCheck className="h-3.5 w-3.5 workbench-accent" />
            Configure compute in Settings
          </button>
        </div>
      </div>
    </div>
  );
};
