import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Play, Square, Plus, Trash2, ChevronUp, ChevronDown, Save, RotateCcw, RefreshCw } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { NotebookCell, NotebookKernelState, ExecutionWorkspace } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { MarkdownViewer } from '@/components/viewer/MarkdownViewer';
import { WorkflowStarters } from './WorkflowStarters';
import { WorkbenchEmptyState } from '@/components/common/WorkbenchPane';

export interface NotebookWorkbenchPaneProps {
  sessionId?: string;
  toolbarNavigation?: React.ReactNode;
  workspace?: ExecutionWorkspace | null;
  onAskAgent?: (draftText: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}
// Presentation buffers only. Executed output and saved cells remain daemon-owned.
const notebookEdits = new Map<string, NotebookCell[]>();
export function editNotebookCell(cell: NotebookCell, source: string, cellType = cell.cell_type): NotebookCell {
  return cell.source === source && cell.cell_type === cellType ? cell : { ...cell, source, cell_type: cellType, status: 'idle', execution_count: null,
    stdout: null, stderr: null, output_image: null, wall_ms: null };
}
export const NotebookWorkbenchPane: React.FC<NotebookWorkbenchPaneProps> = ({ sessionId, workspace, onAskAgent, onShowToast, toolbarNavigation }) => {
  const [cells, setCells] = useState<NotebookCell[]>([]);
  const [kernel, setKernel] = useState<NotebookKernelState | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadStatus, setLoadStatus] = useState<'idle' | 'loading' | 'loaded' | 'failed'>('idle');
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState<string | null>(null);
  const [previewCellId, setPreviewCellId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const generation = useRef(0);
  const busy = useRef(false);
  const key = sessionId || '';
  const load = useCallback(async () => {
    const epoch = ++generation.current;
    if (!sessionId) {
      setCells([]);
      setKernel(null);
      setDirty(false);
      setLoadStatus('idle');
      return;
    }
    setLoading(true); setError(null); setLoadStatus('loading');
    try {
      const [saved, state] = await Promise.all([
        daemonClient.listNotebookCells(sessionId),
        daemonClient.getNotebookKernelStatus(sessionId).catch(() => null),
      ]);
      if (epoch !== generation.current) return;
      const edits = notebookEdits.get(sessionId);
      const effective = edits || saved || [];
      setCells(effective);
      setDirty(!!edits);
      setKernel(state || null);
      setLoadStatus('loaded');
    } catch (reason) {
      if (epoch === generation.current) {
        setLoadStatus('failed');
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    }
    finally { if (epoch === generation.current) setLoading(false); }
  }, [sessionId]);
  useEffect(() => { void load(); return () => { generation.current++; }; }, [load]);
  useEffect(() => {
    if (!sessionId) return;
    const onRemoteChange = (e: Event) => {
      const ev = e as CustomEvent<{ sessionId: string; cells: NotebookCell[] }>;
      if (ev.detail?.sessionId === sessionId && ev.detail.cells) {
        setCells(ev.detail.cells);
        setDirty(true);
      }
    };
    window.addEventListener('custos:notebook-cells-change', onRemoteChange);
    return () => window.removeEventListener('custos:notebook-cells-change', onRemoteChange);
  }, [sessionId]);
  useEffect(() => {
    if (!dirty) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [dirty]);
  const change = (next: NotebookCell[]) => {
    if (loadStatus === 'failed') return;
    const indexed = next.map((cell, index) => ({ ...cell, cell_index: index }));
    notebookEdits.set(key, indexed);
    setCells(indexed);
    setDirty(true);
    window.dispatchEvent(new CustomEvent('custos:notebook-cells-change', { detail: { sessionId: key, cells: indexed } }));
  };
  const save = async () => {
    if (!sessionId || loadStatus === 'failed') return false;
    setSaving(true);
    setError(null);
    try {
      await daemonClient.saveNotebookCells(sessionId, cells);
      notebookEdits.delete(key);
      setDirty(false);
      return true;
    } catch (reason) {
      setError(String(reason));
      return false;
    } finally {
      setSaving(false);
    }
  };
  const add = (cellType: NotebookCell['cell_type'] = 'code') => {
    if (!sessionId || busy.current || loadStatus === 'failed') return;
    change([...cells, { id: crypto.randomUUID(), session_id: sessionId, cell_index: cells.length, cell_type: cellType, source: '', execution_count: null, status: 'idle', epoch: kernel?.epoch || 0, updated_at: Math.floor(Date.now() / 1000) }]);
  };
  const run = async () => {
    const cell = cells.find(item => item.id === previewCellId);
    if (!sessionId || !workspace || !cell || cell.cell_type !== 'code' || busy.current) return;
    busy.current = true; setPreviewCellId(null); setRunning(cell.id);
    const epoch = generation.current;
    try {
      if (!await save()) return;
      const result = await daemonClient.executeNotebookCell({ session_id: sessionId, cell_id: cell.id, code: cell.source, workspace_id: workspace.id });
      if (epoch !== generation.current) return;
      setCells(previous => previous.map(item => item.id === cell.id && item.source === cell.source ? { ...item,
        execution_count: result.execution_count, status: result.status, stdout: result.stdout, stderr: result.stderr,
        output_image: result.output_image, wall_ms: result.wall_ms, epoch: result.epoch } : item));
      setKernel(await daemonClient.getNotebookKernelStatus(sessionId));
      onShowToast?.(`Cell ${result.status} · ${result.wall_ms} ms`);
    } catch (reason) { if (epoch === generation.current) setError(String(reason)); }
    finally { busy.current = false; if (epoch === generation.current) setRunning(null); }
  };
  const reset = async () => {
    if (!sessionId || running || saving) return;
    try { setKernel(await daemonClient.resetNotebookKernel(sessionId)); setError(null); }
    catch (reason) { setError(String(reason)); }
  };
  const preview = cells.find(cell => cell.id === previewCellId);
  const disabled = !sessionId || loading || saving || !!running || loadStatus === 'failed';
  return <div className="h-full flex flex-col min-h-0 bg-canvas text-fg-default">
    <header className="wb-pane-toolbar shrink-0 flex items-center gap-1.5 px-3 py-1.5 border-b border-border-default bg-surface-1 text-xs">
      {toolbarNavigation}
      <button type="button" className="canvas-open-button" disabled={disabled} onClick={() => add()}><Plus size={12} />Code</button>
      <button type="button" className="canvas-open-button" disabled={disabled} onClick={() => add('markdown')}><Plus size={12} />Markdown</button>
      <div className="wb-toolbar-spacer" />
      <span className="wb-meta whitespace-nowrap">{saving ? 'Saving…' : dirty ? 'Unsaved' : 'Saved'}</span>
      <button type="button" className="wb-icon-button" aria-label="Save notebook" title="Save notebook" disabled={disabled || !dirty} onClick={() => void save()}><Save size={13} /></button>
      <button type="button" className="canvas-icon" aria-label="Refresh notebook" disabled={disabled || dirty} onClick={() => void load()}><RefreshCw size={13} /></button>
      <button type="button" className="canvas-icon" aria-label="Reset Python worker" disabled={disabled} onClick={() => void reset()}><RotateCcw size={13} /></button>
      <button type="button" className="wb-icon-button" aria-label="Interrupt notebook execution" title="Interrupt execution" disabled={!running} onClick={async () => { try { const result = await daemonClient.interruptNotebookKernel(key); onShowToast?.(result.interrupted ? 'Worker interrupted.' : 'No running worker.'); } catch (reason) { setError(String(reason)); } }}><Square size={13} /></button>
    </header>
    <div className="wb-context-line">
      <span className="truncate" title={workspace?.path}>{workspace ? workspace.name : 'No execution workspace'}</span>
      <span className="wb-toolbar-spacer" />
      {kernel && <span className="whitespace-nowrap">Python · {kernel.status} · epoch {kernel.epoch}</span>}
    </div>
    {error && <div role="alert" className="p-2 text-xs text-rose-400 bg-rose-500/10 border-b border-rose-500/20 flex items-center justify-between">
      <span className="truncate">{error}</span>
      {loadStatus === 'failed' && (
        <button type="button" className="canvas-open-button ml-2 shrink-0" onClick={() => void load()}>
          <RefreshCw size={11} /> Retry
        </button>
      )}
    </div>}
    <div className="wb-notebook-body flex-1 min-h-0 overflow-auto">
      {!sessionId ? (
        <p className="text-xs text-fg-muted p-4">Select a session to view notebook.</p>
      ) : loading ? (
        <p role="status" className="text-xs text-fg-muted p-4">Loading notebook…</p>
      ) : loadStatus === 'failed' ? (
        <div className="text-center p-6 space-y-2">
          <p className="text-xs text-rose-400 font-medium">Notebook failed to load</p>
          <p className="text-[11px] text-fg-muted">Edits locked to prevent overwriting server state.</p>
          <button type="button" className="canvas-open-button mx-auto" onClick={() => void load()}>
            <RefreshCw size={12} /> Reload
          </button>
        </div>
      ) : cells.length === 0 ? (
        <div className="space-y-3 max-w-lg mx-auto py-6">
          <WorkbenchEmptyState title="Your notebook" description="Add code or a note. Results stay linked to the code and execution epoch." action={<button type="button" className="canvas-open-button" onClick={() => add()}><Plus size={12} />Add a code cell</button>} />
          {onAskAgent && (
            <div className="pt-1">
              <WorkflowStarters onPick={(prompt: string) => onAskAgent(prompt)} />
            </div>
          )}
        </div>
      ) : cells.map((cell, index) => <section key={cell.id} className="wb-notebook-cell">
        <div className="wb-notebook-cell-header">
          <span className="wb-notebook-count">{running === cell.id ? 'Running…' : `In [${cell.execution_count ?? ' '}]`}</span>
          {cell.wall_ms != null && <span className="wb-meta whitespace-nowrap">{cell.wall_ms} ms</span>}
          <div className="wb-toolbar-spacer" />
          <select aria-label={`Cell ${index + 1} type`} value={cell.cell_type} disabled={disabled}
            className="wb-inline-select"
            onChange={event => change(cells.map(item => item.id === cell.id ? editNotebookCell(item, item.source, event.target.value as NotebookCell['cell_type']) : item))}>
            <option value="code">Python</option><option value="markdown">Markdown</option>
          </select>
          <button type="button" className="wb-icon-button" aria-label={`Run cell ${index + 1}`} title="Run cell" disabled={disabled || !workspace || !cell.source.trim() || cell.cell_type !== 'code'} onClick={() => setPreviewCellId(cell.id)}><Play size={13} /></button>
          {onAskAgent && <button type="button" className="canvas-icon" aria-label="Ask about this cell" onClick={() => onAskAgent(`Explain notebook cell ${cell.id} in session ${key}:\n\n${cell.source}`)}>?</button>}
          <button type="button" className="canvas-icon" aria-label="Move cell up" disabled={disabled || index === 0} onClick={() => { const next = [...cells]; [next[index-1], next[index]] = [next[index], next[index-1]]; change(next); }}><ChevronUp size={12} /></button>
          <button type="button" className="canvas-icon" aria-label="Move cell down" disabled={disabled || index === cells.length-1} onClick={() => { const next = [...cells]; [next[index+1], next[index]] = [next[index], next[index+1]]; change(next); }}><ChevronDown size={12} /></button>
          <button type="button" className="canvas-icon" aria-label="Remove cell" disabled={disabled} onClick={() => change(cells.filter(item => item.id !== cell.id))}><Trash2 size={12} /></button>
        </div>
        <textarea aria-label={`Cell ${index+1} source`} value={cell.source} disabled={disabled} spellCheck={false} rows={Math.max(3, Math.min(18, cell.source.split('\n').length+1))} onChange={event => change(cells.map(item => item.id === cell.id ? editNotebookCell(item, event.target.value) : item))} className="w-full resize-y p-3 text-xs font-mono bg-canvas outline-none focus:ring-1 focus:ring-inset focus:ring-accent select-text" />
        {cell.cell_type === 'markdown' && cell.source.trim() && <div className="border-t border-border-default max-h-96 overflow-auto"><MarkdownViewer content={cell.source} embedded /></div>}
        {(cell.stdout || cell.stderr || cell.output_image) && <div className="border-t border-border-default p-3 text-xs select-text space-y-2">
          {cell.stdout && <pre className="whitespace-pre-wrap break-words">{cell.stdout}</pre>}{cell.stderr && <pre className="text-rose-500 whitespace-pre-wrap break-words">{cell.stderr}</pre>}
          {cell.output_image && <img src={cell.output_image} alt={`Output of cell ${index+1}`} className="max-h-96 max-w-full object-contain" />}
          {cell.epoch !== kernel?.epoch && <p className="text-fg-muted">Output belongs to a previous worker epoch.</p>}
        </div>}
      </section>)}
    </div>
    {preview && <div className="absolute inset-0 z-40 bg-canvas/95 p-5 overflow-auto" role="dialog" aria-modal="true" aria-label="Review notebook execution">
      <h3 className="text-sm font-medium mb-3">Run this code locally?</h3><p className="text-xs text-fg-muted mb-3 break-all">Workspace: {workspace?.name} · {workspace?.path}</p>
      <pre className="rounded border border-border-default bg-surface-1 p-3 text-xs whitespace-pre-wrap select-text">{preview.source}</pre>
      <p className="text-[11px] text-fg-muted my-2">Runs in session Python worker; can modify workspace files.</p>
      <div className="flex gap-2"><button type="button" autoFocus className="canvas-open-button" onClick={() => setPreviewCellId(null)}>Cancel</button><button type="button" className="canvas-open-button" onClick={() => void run()}>Run cell</button></div>
    </div>}
  </div>;
};
export const NotebookWorkspacePane = NotebookWorkbenchPane;
