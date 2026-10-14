import React, { useEffect, useState, useCallback } from 'react';
import {
  NotebookTabs,
  Play,
  RotateCcw,
  Plus,
  MoreVertical,
  FlaskConical,
  BarChart3,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { NotebookCell, NotebookKernelState } from '@/types/domain';
import { RunsLedgerPane } from './RunsLedgerPane';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface LabPaneProps {
  sessionId?: string;
  onAskAgent?: (draftText: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export const LabPane: React.FC<LabPaneProps> = ({
  sessionId,
  onAskAgent: _onAskAgent,
  onShowToast,
  onOpenTab,
}) => {
  const [activeSubView, setActiveSubView] = useState<'notebook' | 'runs'>('notebook');
  const [cells, setCells] = useState<NotebookCell[]>([]);
  const [selectedCellId, setSelectedCellId] = useState<string | null>(null);
  const [kernelState, setKernelState] = useState<NotebookKernelState | null>(null);
  const [runningCellId, setRunningCellId] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [kernelError, setKernelError] = useState<string | null>(null);
  const [showSecondaryMenu, setShowSecondaryMenu] = useState<boolean>(false);

  const activeSessionId = sessionId || '';

  const loadData = useCallback(async () => {
    if (!activeSessionId) {
      setCells([]);
      setSelectedCellId(null);
      setKernelState(null);
      setLoading(false);
      return;
    }
    try {
      setLoading(true);
      setLoadError(null);
      const fetchedCells = await daemonClient.listNotebookCells(activeSessionId);
      setCells(fetchedCells);
      setSelectedCellId((previous) =>
        fetchedCells.some((cell) => cell.id === previous) ? previous : (fetchedCells[0]?.id || null)
      );
      try {
        setKernelState(await daemonClient.getNotebookKernelStatus(activeSessionId));
        setKernelError(null);
      } catch (err: any) {
        setKernelState(null);
        setKernelError(err?.message || String(err));
      }
    } catch (err: any) {
      setLoadError(err?.message || String(err));
      setCells([]);
      setSelectedCellId(null);
    } finally {
      setLoading(false);
    }
  }, [activeSessionId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Primary Action: Run Selected Cell
  const handleRunCell = async () => {
    if (!activeSessionId || !kernelState) return;
    const targetCell = cells.find((c) => c.id === selectedCellId) || cells[0];
    if (!targetCell || !targetCell.source.trim()) return;

    setRunningCellId(targetCell.id);
    try {
      await daemonClient.saveNotebookCells(activeSessionId, cells);
      const result = await daemonClient.executeNotebookCell({
        session_id: activeSessionId,
        cell_id: targetCell.id,
        code: targetCell.source,
      });

      setCells((prev) =>
        prev.map((c) =>
          c.id === targetCell.id
            ? {
                ...c,
                execution_count: result.execution_count,
                status: result.status,
                stdout: result.stdout,
                stderr: result.stderr,
                wall_ms: result.wall_ms,
              }
            : c
        )
      );
      onShowToast?.(`Cell ${result.status} (${result.wall_ms}ms).`);
    } catch (err: any) {
      onShowToast?.(`Cell execution failed: ${err?.message || err}`);
    } finally {
      setRunningCellId(null);
    }
  };

  const handleAddCell = async () => {
    setShowSecondaryMenu(false);
    if (!activeSessionId) {
      onShowToast?.('Open a conversation before creating notebook cells.');
      return;
    }
    const newCell: NotebookCell = {
      id: crypto.randomUUID(),
      session_id: activeSessionId,
      cell_index: cells.length,
      cell_type: 'code',
      source: '',
      execution_count: null,
      status: 'idle',
      epoch: 1,
      updated_at: Date.now(),
    };
    try {
      const nextCells = [...cells, newCell];
      await daemonClient.saveNotebookCells(activeSessionId, nextCells);
      setCells(nextCells);
      setSelectedCellId(newCell.id);
    } catch (err: any) {
      onShowToast?.(`Failed to save cell: ${err?.message || err}`);
    }
  };

  const handleResetKernel = async () => {
    setShowSecondaryMenu(false);
    try {
      await daemonClient.resetNotebookKernel(activeSessionId);
      onShowToast?.('Kernel reset completed.');
      await loadData();
    } catch (err: any) {
      onShowToast?.(`Reset failed: ${err?.message || err}`);
    }
  };

  const handleSourceChange = (cellId: string, newSource: string) => {
    setCells((prev) =>
      prev.map((c) => (c.id === cellId && c.source !== newSource
        ? {
            ...c,
            source: newSource,
            execution_count: null,
            status: 'idle',
            stdout: null,
            stderr: null,
            output_image: null,
            wall_ms: null,
            updated_at: Date.now(),
          }
        : c))
    );
  };

  const saveCells = async () => {
    if (!activeSessionId || cells.length === 0) return;
    try {
      await daemonClient.saveNotebookCells(activeSessionId, cells);
    } catch (err: any) {
      onShowToast?.(`Failed to save notebook: ${err?.message || err}`);
    }
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Header */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 bg-[var(--color-surface-1)]/50">
        <div className="flex items-center gap-2">
          <FlaskConical className="h-4 w-4 text-[var(--color-fg-muted)] shrink-0" />
          <span className="text-xs font-semibold">Lab</span>

          {/* Sub-view switcher */}
          <div className="ml-3 flex rounded border border-[var(--color-border-subtle)] p-0.5 bg-[var(--color-surface-1)]">
            <button
              type="button"
              onClick={() => setActiveSubView('notebook')}
              className={`flex items-center gap-1.5 px-2 py-0.5 text-[10px] font-medium rounded transition ${
                activeSubView === 'notebook'
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <NotebookTabs className="h-3 w-3" />
              Notebook
            </button>
            <button
              type="button"
              onClick={() => setActiveSubView('runs')}
              className={`flex items-center gap-1.5 px-2 py-0.5 text-[10px] font-medium rounded transition ${
                activeSubView === 'runs'
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <BarChart3 className="h-3 w-3" />
              Run Ledger
            </button>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Single Primary Action: Run Cell */}
          <button
            type="button"
            disabled={activeSubView !== 'notebook' || runningCellId !== null || !activeSessionId || !kernelState || !cells.some((cell) => cell.id === selectedCellId && cell.source.trim())}
            onClick={handleRunCell}
            className="flex items-center gap-1.5 rounded bg-emerald-600 px-2.5 py-1 text-xs font-medium text-white transition hover:bg-emerald-500 disabled:opacity-50"
          >
            <Play className="h-3.5 w-3.5 fill-current" />
            <span>Run Cell</span>
          </button>

          {/* Secondary Actions in ··· Menu */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowSecondaryMenu(!showSecondaryMenu)}
              className="flex h-7 w-7 items-center justify-center rounded border border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
              title="More actions"
            >
              <MoreVertical className="h-3.5 w-3.5" />
            </button>

            {showSecondaryMenu && (
              <div
                className="absolute right-0 top-8 z-50 w-44 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-lg text-xs"
                onMouseLeave={() => setShowSecondaryMenu(false)}
              >
                <button
                  type="button"
                  onClick={handleAddCell}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-emerald-400 hover:bg-[var(--color-surface-2)]"
                >
                  <Plus className="h-3.5 w-3.5" />
                  <span>Insert Code Cell</span>
                </button>
                <button
                  type="button"
                  onClick={handleResetKernel}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-amber-400 hover:bg-[var(--color-surface-2)]"
                >
                  <RotateCcw className="h-3.5 w-3.5" />
                  <span>Reset Kernel</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {(loadError || kernelError || !activeSessionId) && (
        <div className="border-b border-[var(--color-border-subtle)] px-4 py-2 text-xs text-[var(--color-fg-muted)]">
          {!activeSessionId ? 'Open a conversation to save notebook cells.' : loadError ? `Notebook unavailable: ${loadError}` : `Kernel unavailable: ${kernelError}`}
          {activeSessionId && <button type="button" onClick={() => void loadData()} className="ml-3 underline underline-offset-2">Retry</button>}
        </div>
      )}

      {/* Main Content Area */}
      <div className="flex-1 min-h-0">
        {activeSubView === 'runs' ? (
          <RunsLedgerPane sessionId={sessionId} onShowToast={onShowToast} onOpenTab={onOpenTab} />
        ) : (
          <div className="h-full overflow-y-auto p-4 space-y-3 font-mono text-xs">
            {!loading && cells.length === 0 && !loadError && (
              <div className="rounded-lg border border-[var(--color-border-subtle)] p-5 text-center text-xs text-[var(--color-fg-muted)]">
                No cells yet. Add a code cell to begin.
                <button type="button" onClick={() => void handleAddCell()} disabled={!activeSessionId} className="ml-2 underline disabled:opacity-50">Add cell</button>
              </div>
            )}
            {cells.map((cell, idx) => {
              const isSelected = cell.id === selectedCellId;
              const isExecuting = cell.id === runningCellId;

              return (
                <div
                  key={cell.id}
                  onClick={() => setSelectedCellId(cell.id)}
                  className={`rounded-xl border transition p-3 space-y-2 cursor-pointer ${
                    isSelected
                      ? 'border-emerald-500/50 bg-[var(--color-surface-1)] shadow-xs'
                      : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]/40 hover:border-[var(--color-border-default)]'
                  }`}
                >
                  {/* Cell Header */}
                  <div className="flex items-center justify-between text-[11px] text-[var(--color-fg-subtle)] select-none">
                    <span className="font-semibold text-[var(--color-fg-muted)]">
                      [{cell.execution_count !== null && cell.execution_count !== undefined ? cell.execution_count : ' '}] Cell {idx + 1}
                    </span>
                    {isExecuting && (
                      <span className="text-emerald-400 motion-safe:animate-pulse">
                        Executing...
                      </span>
                    )}
                  </div>

                  {/* Code Editor Area */}
                  <textarea
                    value={cell.source}
                    onChange={(e) => handleSourceChange(cell.id, e.target.value)}
                    onBlur={() => void saveCells()}
                    rows={Math.max(3, cell.source.split('\n').length)}
                    className="w-full resize-none rounded border border-[var(--color-border-subtle)] bg-[var(--color-canvas)] p-2.5 font-mono text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-emerald-500"
                  />

                  {/* Cell Execution Output */}
                  {(cell.stdout || cell.stderr) && (
                    <div className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] p-2.5 font-mono text-[11px] whitespace-pre-wrap space-y-1">
                      {cell.stdout && <div className="text-[var(--color-fg-muted)]">{cell.stdout}</div>}
                      {cell.stderr && <div className="text-rose-400">{cell.stderr}</div>}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
