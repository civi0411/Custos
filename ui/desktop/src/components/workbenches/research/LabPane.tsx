import React, { useEffect, useState, useCallback } from 'react';
import {
  NotebookTabs,
  Play,
  RotateCcw,
  Plus,
  Trash2,
  MoreVertical,
  FlaskConical,
  BarChart3,
  Clock,
  Terminal,
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
        fetchedCells.some((cell) => cell.id === previous)
          ? previous
          : fetchedCells[0]?.id || null
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
  const handleRunCell = async (cellIdToRun?: string) => {
    if (!activeSessionId) {
      onShowToast?.('Select an active session to execute code cells.');
      return;
    }
    const targetId = cellIdToRun || selectedCellId;
    const targetCell = cells.find((c) => c.id === targetId) || cells[0];
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

  const handleDeleteCell = async (cellId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!activeSessionId) return;
    const next = cells.filter((c) => c.id !== cellId);
    try {
      await daemonClient.saveNotebookCells(activeSessionId, next);
      setCells(next);
      if (selectedCellId === cellId) {
        setSelectedCellId(next[0]?.id || null);
      }
      onShowToast?.('Cell removed.');
    } catch (err: any) {
      onShowToast?.(`Failed to delete cell: ${err?.message || err}`);
    }
  };

  const handleResetKernel = async () => {
    setShowSecondaryMenu(false);
    try {
      await daemonClient.resetNotebookKernel(activeSessionId);
      onShowToast?.('Kernel state reset completed.');
      await loadData();
    } catch (err: any) {
      onShowToast?.(`Reset failed: ${err?.message || err}`);
    }
  };

  const handleSourceChange = (cellId: string, newSource: string) => {
    setCells((prev) =>
      prev.map((c) =>
        c.id === cellId && c.source !== newSource
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
          : c
      )
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
    <div className="flex h-full w-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden font-sans">
      {/* ── Surface Header ── */}
      <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] px-3 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-2.5">
          <FlaskConical className="h-4 w-4 text-workbench-accent shrink-0" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)]">Lab</span>

          {/* Sub-view switcher */}
          <div className="flex rounded-lg border border-[var(--color-border-muted)] p-0.5 bg-[var(--color-surface-2)]">
            <button
              type="button"
              onClick={() => setActiveSubView('notebook')}
              className={`flex items-center gap-1.5 px-2.5 py-0.5 text-[11px] rounded-md transition ${
                activeSubView === 'notebook'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] font-semibold shadow-xs'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <NotebookTabs className="h-3 w-3" />
              Notebook ({cells.length})
            </button>
            <button
              type="button"
              onClick={() => setActiveSubView('runs')}
              className={`flex items-center gap-1.5 px-2.5 py-0.5 text-[11px] rounded-md transition ${
                activeSubView === 'runs'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] font-semibold shadow-xs'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <BarChart3 className="h-3 w-3" />
              Run Ledger
            </button>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Kernel Status Badge */}
          {kernelState && (
            <span className="hidden sm:flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2 py-0.5 font-mono text-[9.5px] text-emerald-400 border border-emerald-500/20">
              <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" />
              {kernelState.status === 'idle' ? 'Kernel Ready' : kernelState.status}
            </span>
          )}

          {/* Run Selected Cell */}
          <button
            type="button"
            disabled={
              activeSubView !== 'notebook' ||
              runningCellId !== null ||
              !activeSessionId ||
              !cells.some((cell) => cell.id === selectedCellId && cell.source.trim())
            }
            onClick={() => handleRunCell()}
            className="flex items-center gap-1.5 rounded-lg bg-teal-600 px-3 py-1 text-xs font-medium text-white transition hover:bg-teal-500 disabled:opacity-50"
          >
            <Play className="h-3 w-3 fill-current" />
            <span>{runningCellId ? 'Running…' : 'Run Cell'}</span>
          </button>

          {/* Secondary Actions in ··· Menu */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowSecondaryMenu(!showSecondaryMenu)}
              className="flex h-7 w-7 items-center justify-center rounded-md border border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
              title="More actions"
            >
              <MoreVertical className="h-3.5 w-3.5" />
            </button>

            {showSecondaryMenu && (
              <div
                className="absolute right-0 top-8 z-50 w-44 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-2xl text-xs"
                onMouseLeave={() => setShowSecondaryMenu(false)}
              >
                <button
                  type="button"
                  onClick={handleAddCell}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-teal-400 hover:bg-[var(--color-surface-2)]"
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
        <div className="border-b border-[var(--color-border-muted)] bg-[var(--color-surface-2)] px-4 py-2 text-xs text-[var(--color-fg-muted)] flex items-center justify-between">
          <span>
            {!activeSessionId
              ? 'Select or start a chat session to bind a live execution notebook.'
              : loadError
              ? `Notebook data: ${loadError}`
              : `Kernel: ${kernelError}`}
          </span>
          {activeSessionId && (
            <button
              type="button"
              onClick={() => void loadData()}
              className="text-teal-400 hover:underline text-[11px]"
            >
              Retry
            </button>
          )}
        </div>
      )}

      {/* ── Main Content Area ── */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {activeSubView === 'runs' ? (
          <RunsLedgerPane sessionId={sessionId} onShowToast={onShowToast} onOpenTab={onOpenTab} />
        ) : (
          <div className="h-full overflow-y-auto p-4 lg:p-6 space-y-4 max-w-4xl mx-auto w-full no-scrollbar">
            {!loading && cells.length === 0 && !loadError && (
              <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-8 text-center text-xs text-[var(--color-fg-subtle)] space-y-3">
                <Terminal className="h-8 w-8 mx-auto text-[var(--color-border-default)]" />
                <div className="font-medium text-[var(--color-editor-fg)]">Notebook is empty</div>
                <p className="text-[11px] text-[var(--color-fg-muted)] max-w-xs mx-auto">
                  Create code cells to run Python, verify claims, and extract empirical artifacts.
                </p>
                <button
                  type="button"
                  onClick={() => void handleAddCell()}
                  disabled={!activeSessionId}
                  className="rounded-lg bg-teal-600 px-3.5 py-1.5 text-xs font-medium text-white hover:bg-teal-500 disabled:opacity-50 transition"
                >
                  Insert First Cell
                </button>
              </div>
            )}

            {cells.map((cell, idx) => {
              const isSelected = cell.id === selectedCellId;
              const isExecuting = cell.id === runningCellId;

              return (
                <div
                  key={cell.id}
                  onClick={() => setSelectedCellId(cell.id)}
                  className={`rounded-xl border transition-all duration-150 p-3.5 space-y-2.5 cursor-pointer shadow-xs ${
                    isSelected
                      ? 'border-teal-500/50 bg-[var(--color-surface-1)] ring-1 ring-teal-500/20'
                      : 'border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/60 hover:border-[var(--color-border-default)]'
                  }`}
                >
                  {/* Cell Header */}
                  <div className="flex items-center justify-between text-xs select-none">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-[11px] font-semibold text-teal-400">
                        In [{cell.execution_count !== null && cell.execution_count !== undefined ? cell.execution_count : ' '}]:
                      </span>
                      <span className="text-[10px] text-[var(--color-fg-subtle)] uppercase tracking-wider font-semibold">
                        Cell {idx + 1}
                      </span>
                    </div>

                    <div className="flex items-center gap-2">
                      {isExecuting && (
                        <span className="flex items-center gap-1 font-mono text-[10px] text-teal-400 animate-pulse">
                          <span className="h-1.5 w-1.5 rounded-full bg-teal-400" />
                          Executing…
                        </span>
                      )}
                      {cell.wall_ms !== null && cell.wall_ms !== undefined && !isExecuting && (
                        <span className="flex items-center gap-1 font-mono text-[10px] text-[var(--color-fg-subtle)]">
                          <Clock size={10} />
                          {cell.wall_ms}ms
                        </span>
                      )}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          void handleRunCell(cell.id);
                        }}
                        disabled={isExecuting || !cell.source.trim()}
                        className="p-1 rounded text-[var(--color-fg-muted)] hover:text-teal-400 hover:bg-[var(--color-surface-2)] transition disabled:opacity-40"
                        title="Run this cell (Shift+Enter)"
                      >
                        <Play size={12} className="fill-current" />
                      </button>
                      <button
                        type="button"
                        onClick={(e) => handleDeleteCell(cell.id, e)}
                        className="p-1 rounded text-[var(--color-fg-subtle)] hover:text-rose-400 hover:bg-[var(--color-surface-2)] transition"
                        title="Delete cell"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </div>

                  {/* Code Editor Area */}
                  <div className="relative rounded-lg overflow-hidden border border-[var(--color-border-muted)] bg-[var(--color-canvas)]">
                    <textarea
                      value={cell.source}
                      onChange={(e) => handleSourceChange(cell.id, e.target.value)}
                      onBlur={() => void saveCells()}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' && e.shiftKey) {
                          e.preventDefault();
                          void handleRunCell(cell.id);
                        }
                      }}
                      rows={Math.max(3, cell.source.split('\n').length)}
                      placeholder="# Enter Python code here... (Shift+Enter to run)"
                      className="w-full resize-none p-3 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] bg-transparent focus:outline-none"
                    />
                  </div>

                  {/* Cell Execution Output */}
                  {(cell.stdout || cell.stderr) && (
                    <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-3 font-mono text-xs whitespace-pre-wrap space-y-1 overflow-x-auto">
                      <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] pb-1 border-b border-[var(--color-border-muted)] flex items-center justify-between">
                        <span>Out [{cell.execution_count || ''}]:</span>
                        <span className={cell.status === 'success' ? 'text-emerald-400' : 'text-rose-400'}>
                          {cell.status}
                        </span>
                      </div>
                      {cell.stdout && <div className="text-[var(--color-editor-fg)] pt-1">{cell.stdout}</div>}
                      {cell.stderr && <div className="text-rose-400 pt-1">{cell.stderr}</div>}
                    </div>
                  )}
                </div>
              );
            })}

            {/* Bottom Add Cell Button */}
            {cells.length > 0 && (
              <div className="pt-2 flex justify-center">
                <button
                  type="button"
                  onClick={handleAddCell}
                  className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] px-4 py-1.5 text-xs text-[var(--color-editor-fg)] transition shadow-xs"
                >
                  <Plus size={13} className="text-teal-400" />
                  <span>Insert Code Cell</span>
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default LabPane;
