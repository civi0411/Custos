import React, { useEffect, useState } from 'react';
import {
  Play,
  RotateCcw,
  Square,
  Plus,
  Trash2,
  ChevronUp,
  ChevronDown,
  Terminal,
  Clock,
  ShieldCheck,
  AlertCircle,
  Save,
  RefreshCw,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type {
  NotebookCell,
  NotebookKernelState,
  NotebookCellType,
  CellExecutionStatus,
} from '@/types/domain';

interface NotebookWorkspacePaneProps {
  sessionId?: string;
  onAskAgent?: (draftText: string) => void;
  onShowToast?: (msg: string) => void;
}

export const NotebookWorkspacePane: React.FC<NotebookWorkspacePaneProps> = ({
  sessionId = 'default-notebook-session',
  onShowToast,
}) => {
  const [cells, setCells] = useState<NotebookCell[]>([]);
  const [kernelState, setKernelState] = useState<NotebookKernelState | null>(null);
  const [runningCellId, setRunningCellId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeSessionId = sessionId || 'default-notebook-session';

  const loadNotebook = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [fetchedCells, status] = await Promise.all([
        daemonClient.listNotebookCells(activeSessionId),
        daemonClient.getNotebookKernelStatus(activeSessionId),
      ]);

      setKernelState(status);

      if (fetchedCells && fetchedCells.length > 0) {
        setCells(fetchedCells);
      } else {
        const defaultCell: NotebookCell = {
          id: `cell_${Date.now()}_1`,
          session_id: activeSessionId,
          cell_type: 'code',
          source: `# Custos Authorized Python Kernel\n# State and variables persist across cells within the current Epoch.\nimport sys\nprint(f"Kernel Python Version: {sys.version}")\n\nx = 21 * 2\nprint(f"Computed value: x = {x}")`,
          cell_index: 0,
          execution_count: null,
          status: 'idle',
          stdout: null,
          stderr: null,
          output_image: null,
          wall_ms: null,
          epoch: status?.epoch || 1,
          updated_at: Math.floor(Date.now() / 1000),
        };
        setCells([defaultCell]);
        await daemonClient.saveNotebookCells(activeSessionId, [defaultCell]);
      }
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadNotebook();
  }, [activeSessionId]);

  const saveCells = async (updatedCells: NotebookCell[]) => {
    setIsSaving(true);
    try {
      await daemonClient.saveNotebookCells(activeSessionId, updatedCells);
      onShowToast?.('Notebook saved');
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const handleCellSourceChange = (cellId: string, newSource: string) => {
    setCells((prev) =>
      prev.map((c) => (c.id === cellId ? { ...c, source: newSource } : c))
    );
  };

  const handleRunCell = async (cell: NotebookCell) => {
    if (runningCellId) return;
    setRunningCellId(cell.id);
    setError(null);

    setCells((prev) =>
      prev.map((c) =>
        c.id === cell.id ? { ...c, status: 'running' as CellExecutionStatus } : c
      )
    );

    try {
      const result = await daemonClient.executeNotebookCell({
        session_id: activeSessionId,
        cell_id: cell.id,
        code: cell.source,
      });

      setCells((prev) =>
        prev.map((c) => {
          if (c.id === cell.id) {
            return {
              ...c,
              status: result.status,
              execution_count: result.execution_count,
              stdout: result.stdout,
              stderr: result.stderr,
              output_image: result.output_image,
              wall_ms: result.wall_ms,
              epoch: result.epoch,
            };
          }
          return c;
        })
      );

      const status = await daemonClient.getNotebookKernelStatus(activeSessionId);
      setKernelState(status);
    } catch (err: any) {
      setError(err?.message || String(err));
      setCells((prev) =>
        prev.map((c) =>
          c.id === cell.id ? { ...c, status: 'error' as CellExecutionStatus, stderr: err?.message || String(err) } : c
        )
      );
    } finally {
      setRunningCellId(null);
    }
  };

  const handleInterrupt = async () => {
    try {
      await daemonClient.interruptNotebookKernel(activeSessionId);
      onShowToast?.('SIGINT signal sent to active kernel process');
      const status = await daemonClient.getNotebookKernelStatus(activeSessionId);
      setKernelState(status);
    } catch (err: any) {
      setError(err?.message || String(err));
    }
  };

  const handleResetKernel = async () => {
    try {
      const status = await daemonClient.resetNotebookKernel(activeSessionId);
      setKernelState(status);
      onShowToast?.(`Kernel reset. New Epoch #${status.epoch} initialized.`);
    } catch (err: any) {
      setError(err?.message || String(err));
    }
  };

  const addCell = (type: NotebookCellType = 'code', afterIndex?: number) => {
    const newCell: NotebookCell = {
      id: `cell_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
      session_id: activeSessionId,
      cell_type: type,
      source: type === 'code' ? '# Code cell\n' : '### Markdown Note\n',
      cell_index: afterIndex !== undefined ? afterIndex + 1 : cells.length,
      execution_count: null,
      status: 'idle',
      stdout: null,
      stderr: null,
      output_image: null,
      wall_ms: null,
      epoch: kernelState?.epoch || 1,
      updated_at: Math.floor(Date.now() / 1000),
    };

    let updated: NotebookCell[];
    if (afterIndex !== undefined) {
      updated = [...cells.slice(0, afterIndex + 1), newCell, ...cells.slice(afterIndex + 1)].map(
        (c, idx) => ({ ...c, cell_index: idx })
      );
    } else {
      updated = [...cells, newCell].map((c, idx) => ({ ...c, cell_index: idx }));
    }

    setCells(updated);
    saveCells(updated);
  };

  const removeCell = (cellId: string) => {
    if (cells.length <= 1) {
      onShowToast?.('Notebook must retain at least one cell');
      return;
    }
    const updated = cells.filter((c) => c.id !== cellId).map((c, idx) => ({ ...c, cell_index: idx }));
    setCells(updated);
    saveCells(updated);
  };

  const moveCell = (index: number, direction: 'up' | 'down') => {
    if (direction === 'up' && index === 0) return;
    if (direction === 'down' && index === cells.length - 1) return;
    const targetIndex = direction === 'up' ? index - 1 : index + 1;
    const reordered = [...cells];
    const [moved] = reordered.splice(index, 1);
    reordered.splice(targetIndex, 0, moved);
    const updated = reordered.map((c, idx) => ({ ...c, cell_index: idx }));
    setCells(updated);
    saveCells(updated);
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-fg-default)]">
      {/* Top Header / Kernel Status Bar */}
      <div className="flex flex-wrap items-center justify-between border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] px-4 py-2.5">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Terminal className="h-4 w-4 text-[var(--color-fg-muted)]" />
            <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-default)]">
              Python Kernel
            </span>
          </div>

          {/* Kernel Status Badge */}
          <div className="flex items-center gap-1.5 rounded-full border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-0.5 text-xs">
            <span
              className={`h-2 w-2 rounded-full ${
                runningCellId
                  ? 'animate-pulse bg-amber-400'
                  : kernelState?.status === 'idle'
                  ? 'bg-emerald-400'
                  : 'bg-zinc-400'
              }`}
            />
            <span className="font-mono text-[11px] text-[var(--color-fg-muted)]">
              {runningCellId ? 'Busy' : kernelState?.status || 'Idle'}
            </span>
          </div>

          {/* Epoch Pill */}
          <div
            className="flex items-center gap-1 rounded border border-blue-500/20 bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-400"
            title="Epoch isolates execution state across kernel resets"
          >
            <ShieldCheck className="h-3 w-3" />
            <span>Epoch #{kernelState?.epoch ?? 1}</span>
          </div>

          {/* Python Version */}
          {kernelState?.python_version && (
            <span className="hidden font-mono text-[11px] text-[var(--color-fg-subtle)] sm:inline">
              {kernelState.python_version}
            </span>
          )}
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleInterrupt}
            disabled={!runningCellId}
            className="inline-flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-muted)] transition-colors hover:bg-[var(--color-surface-3)] hover:text-amber-400 disabled:opacity-40"
            title="Send SIGINT interrupt to kernel"
          >
            <Square className="h-3 w-3" />
            <span>Interrupt</span>
          </button>

          <button
            onClick={handleResetKernel}
            className="inline-flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-muted)] transition-colors hover:bg-[var(--color-surface-3)] hover:text-red-400"
            title="Reset kernel environment and advance to new Epoch"
          >
            <RotateCcw className="h-3 w-3" />
            <span>Reset (New Epoch)</span>
          </button>

          <button
            onClick={() => saveCells(cells)}
            disabled={isSaving}
            className="inline-flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-muted)] transition-colors hover:bg-[var(--color-surface-3)] hover:text-[var(--color-fg-default)]"
            title="Persist notebook cells"
          >
            <Save className="h-3 w-3" />
            <span>{isSaving ? 'Saving...' : 'Save'}</span>
          </button>

          <button
            onClick={() => addCell('code')}
            className="inline-flex items-center gap-1 rounded bg-blue-600 px-2.5 py-1 text-xs font-medium text-white transition-colors hover:bg-blue-500"
          >
            <Plus className="h-3 w-3" />
            <span>Cell</span>
          </button>
        </div>
      </div>

      {/* Global Error Banner */}
      {error && (
        <div className="flex items-center gap-2 border-b border-red-500/20 bg-red-500/10 px-4 py-2 text-xs text-red-400">
          <AlertCircle className="h-4 w-4 shrink-0" />
          <span className="flex-1 font-mono">{error}</span>
          <button
            onClick={() => setError(null)}
            className="text-[11px] underline opacity-80 hover:opacity-100"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Notebook Scrollable Cells Area */}
      <div className="flex-1 space-y-4 overflow-y-auto p-4 sm:p-6">
        {isLoading && cells.length === 0 ? (
          <div className="flex h-40 items-center justify-center text-xs text-[var(--color-fg-muted)]">
            <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
            Connecting to authorized kernel...
          </div>
        ) : (
          cells.map((cell, index) => (
            <div
              key={cell.id}
              className="group relative rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] transition-shadow hover:border-[var(--color-border-default)]"
            >
              {/* Cell Header Toolbar */}
              <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]/60 px-3 py-1.5 text-xs">
                <div className="flex items-center gap-2">
                  {/* Execution Counter Indicator */}
                  <span className="font-mono text-xs font-semibold text-[var(--color-fg-muted)]">
                    {cell.status === 'running'
                      ? 'In [*]:'
                      : cell.execution_count != null
                      ? `In [${cell.execution_count}]:`
                      : 'In [ ]:'}
                  </span>

                  {cell.wall_ms != null && (
                    <span className="inline-flex items-center gap-1 font-mono text-[10px] text-[var(--color-fg-subtle)]">
                      <Clock className="h-2.5 w-2.5" />
                      {cell.wall_ms}ms
                    </span>
                  )}

                  {cell.epoch > 0 && (
                    <span className="rounded bg-[var(--color-surface-3)] px-1.5 py-0.2 text-[10px] font-mono text-[var(--color-fg-subtle)]">
                      e{cell.epoch}
                    </span>
                  )}
                </div>

                {/* Cell Controls */}
                <div className="flex items-center gap-1 opacity-70 group-hover:opacity-100">
                  <button
                    onClick={() => handleRunCell(cell)}
                    disabled={runningCellId !== null}
                    className="inline-flex items-center gap-1 rounded bg-emerald-600/20 px-2 py-0.5 text-xs font-medium text-emerald-400 hover:bg-emerald-600/30 disabled:opacity-40"
                    title="Execute cell"
                  >
                    <Play className="h-3 w-3 fill-emerald-400" />
                    <span>Run</span>
                  </button>

                  <button
                    onClick={() => moveCell(index, 'up')}
                    disabled={index === 0}
                    className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)] disabled:opacity-30"
                    title="Move up"
                  >
                    <ChevronUp className="h-3.5 w-3.5" />
                  </button>

                  <button
                    onClick={() => moveCell(index, 'down')}
                    disabled={index === cells.length - 1}
                    className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)] disabled:opacity-30"
                    title="Move down"
                  >
                    <ChevronDown className="h-3.5 w-3.5" />
                  </button>

                  <button
                    onClick={() => removeCell(cell.id)}
                    className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)] hover:text-red-400"
                    title="Delete cell"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              {/* Cell Code Editor Input */}
              <div className="p-3">
                <textarea
                  value={cell.source}
                  onChange={(e) => handleCellSourceChange(cell.id, e.target.value)}
                  onKeyDown={(e) => {
                    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
                      e.preventDefault();
                      handleRunCell(cell);
                    }
                  }}
                  rows={Math.max(2, cell.source.split('\n').length)}
                  className="w-full resize-y rounded bg-black/30 p-2.5 font-mono text-xs leading-relaxed text-[var(--color-fg-default)] placeholder:text-[var(--color-fg-subtle)] focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500/40"
                  placeholder="# Enter Python code here. Press Ctrl+Enter or Cmd+Enter to run."
                  spellCheck={false}
                />
              </div>

              {/* Cell Outputs */}
              {(cell.stdout || cell.stderr || cell.output_image || cell.status === 'running') && (
                <div className="border-t border-[var(--color-border-subtle)] bg-black/40 p-3">
                  {cell.status === 'running' && (
                    <div className="flex items-center gap-2 text-xs text-amber-400">
                      <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                      <span>Executing in epoch #{cell.epoch}...</span>
                    </div>
                  )}

                  {/* Stdout */}
                  {cell.stdout && (
                    <div className="mb-2">
                      <pre className="overflow-x-auto whitespace-pre-wrap font-mono text-xs text-zinc-300">
                        {cell.stdout}
                      </pre>
                    </div>
                  )}

                  {/* Stderr */}
                  {cell.stderr && (
                    <div className="mb-2 rounded border border-red-500/20 bg-red-950/20 p-2">
                      <pre className="overflow-x-auto whitespace-pre-wrap font-mono text-xs text-red-400">
                        {cell.stderr}
                      </pre>
                    </div>
                  )}

                  {/* Rendered Matplotlib / PIL Plot Image */}
                  {cell.output_image && (
                    <div className="mt-2 overflow-hidden rounded border border-[var(--color-border-subtle)] bg-white/5 p-2">
                      <img
                        src={cell.output_image}
                        alt="Kernel Output Plot"
                        className="max-h-96 rounded object-contain"
                      />
                    </div>
                  )}
                </div>
              )}
            </div>
          ))
        )}

        {/* Bottom Append Bar */}
        <div className="flex items-center justify-center pt-2">
          <button
            onClick={() => addCell('code')}
            className="inline-flex items-center gap-1.5 rounded-full border border-dashed border-[var(--color-border-default)] px-4 py-1.5 text-xs text-[var(--color-fg-muted)] transition-colors hover:border-blue-500 hover:text-blue-400"
          >
            <Plus className="h-3.5 w-3.5" />
            <span>Add Code Cell</span>
          </button>
        </div>
      </div>
    </div>
  );
};
