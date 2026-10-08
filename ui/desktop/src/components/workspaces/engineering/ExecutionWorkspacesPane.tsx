import React, { useEffect, useState } from 'react';
import { Folder, GitBranch, HardDrive, RefreshCw, Server } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { ExecutionWorkspace } from '@/types/domain';

const kindLabel = (workspace: ExecutionWorkspace) => {
  if (workspace.kind.type === 'git') return 'Git worktree';
  if (workspace.kind.type === 'remote_ssh') return 'Remote SSH';
  return 'Folder';
};

const KindIcon = ({ workspace }: { workspace: ExecutionWorkspace }) => {
  if (workspace.kind.type === 'git') return <GitBranch className="h-4 w-4" />;
  if (workspace.kind.type === 'remote_ssh') return <Server className="h-4 w-4" />;
  return <Folder className="h-4 w-4" />;
};

export const ExecutionWorkspacesPane: React.FC = () => {
  const [records, setRecords] = useState<ExecutionWorkspace[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = () => {
    setLoading(true);
    setError(null);
    daemonClient.listWorkspaces()
      .then(setRecords)
      .catch((reason) => setError(String(reason)))
      .finally(() => setLoading(false));
  };

  useEffect(reload, []);

  return (
    <div className="h-full overflow-y-auto bg-[var(--color-canvas)] p-6 text-[var(--color-editor-fg)]">
      <div className="mx-auto max-w-5xl">
        <header className="flex items-start justify-between gap-4">
          <div>
            <div className="workbench-kicker">execution plane</div>
            <h1 className="mt-1 text-lg font-semibold tracking-[-0.02em]">Workspaces</h1>
            <p className="mt-1 max-w-2xl text-xs leading-5 text-[var(--color-fg-muted)]">
              Daemon-owned folder and Git workspace records. Terminal, browser, agent and SSH
              capabilities are evaluated separately; a ready workspace does not imply they exist.
            </p>
          </div>
          <button type="button" onClick={reload} disabled={loading} className="rounded-md border border-[var(--color-border-default)] p-2 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] disabled:opacity-50" aria-label="Refresh workspaces">
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? 'animate-spin' : ''}`} />
          </button>
        </header>

        {error && <div className="mt-5 rounded-lg border border-rose-500/20 p-4 text-xs text-rose-400">Workspace API unavailable: {error}</div>}

        <div className="mt-6 grid gap-3 lg:grid-cols-2">
          {records.map((workspace) => (
            <article key={workspace.id} className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
              <div className="flex items-start justify-between gap-4">
                <div className="flex min-w-0 gap-3">
                  <div className="mt-0.5 text-[var(--color-fg-muted)]"><KindIcon workspace={workspace} /></div>
                  <div className="min-w-0">
                    <h2 className="truncate text-xs font-semibold">{workspace.name}</h2>
                    <p className="mt-1 truncate font-mono text-[10px] text-[var(--color-fg-subtle)]">{workspace.path}</p>
                  </div>
                </div>
                <span className="text-[9px] font-semibold uppercase tracking-[0.08em] text-[var(--color-fg-muted)]">{workspace.status}</span>
              </div>
              <div className="mt-4 flex flex-wrap gap-x-4 gap-y-2 border-t border-[var(--color-border-muted)] pt-3 text-[10px] text-[var(--color-fg-subtle)]">
                <span>{kindLabel(workspace)}</span>
                <span className="font-mono">{workspace.id}</span>
                {workspace.lineage.base_commit && <span>base {workspace.lineage.base_commit.slice(0, 10)}</span>}
                {workspace.lineage.head_commit && <span>head {workspace.lineage.head_commit.slice(0, 10)}</span>}
              </div>
              {workspace.status_reason && <p className="mt-3 text-[10px] text-rose-400">{workspace.status_reason}</p>}
            </article>
          ))}
        </div>

        {!loading && !error && records.length === 0 && (
          <div className="mt-12 text-center">
            <HardDrive className="mx-auto h-5 w-5 text-[var(--color-fg-subtle)]" />
            <p className="mt-3 text-xs text-[var(--color-fg-muted)]">No execution workspaces have been registered.</p>
          </div>
        )}
      </div>
    </div>
  );
};
