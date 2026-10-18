import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Play, Square, Plus, Trash2, ChevronUp, ChevronDown, Save, RotateCcw, RefreshCw } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { NotebookCell, NotebookKernelState, ExecutionWorkspace } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { MarkdownViewer } from '@/components/viewer/MarkdownViewer';
import { WorkflowStarters } from './WorkflowStarters';

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
      return;
    }
    setLoading(true); setError(null);
    try {
      const [saved, state] = await Promise.all([
        daemonClient.listNotebookCells(sessionId).catch(() => []),
        daemonClient.getNotebookKernelStatus(sessionId).catch(() => null),
      ]);
      if (epoch !== generation.current) return;
      const edits = notebookEdits.get(sessionId);
      const effective = edits || (saved && saved.length > 0 ? saved : []);
      setCells(effective);
      setDirty(!!edits);
      setKernel(state || null);
    } catch (reason) {
      if (epoch === generation.current) {
        setCells([]);
        setKernel(null);
      }
    }
    finally { if (epoch === generation.current) setLoading(false); }
  }, [sessionId]);
  useEffect(() => { void load(); return () => { generation.current++; }; }, [load]);
  useEffect(() => {
    if (!dirty) return;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [dirty]);
  const change = (next: NotebookCell[]) => {
    const indexed = next.map((cell, index) => ({ ...cell, cell_index: index }));
    notebookEdits.set(key, indexed); setCells(indexed); setDirty(true);
  };
  const save = async () => {
    if (!sessionId) return false;
    setSaving(true); setError(null);
    try { await daemonClient.saveNotebookCells(sessionId, cells); notebookEdits.delete(key); setDirty(false); return true; }
    catch (reason) { setError(String(reason)); return false; }
    finally { setSaving(false); }
  };
  const add = (cellType: NotebookCell['cell_type'] = 'code') => {
    if (!sessionId || busy.current) return;
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
  const disabled = !sessionId || loading || saving || !!running;
  return <div className="h-full flex flex-col min-h-0 bg-canvas text-fg-default">
    <header className="shrink-0 flex flex-wrap items-center gap-2 px-3 py-2 border-b border-border-default text-xs">
      {toolbarNavigation}
      <span className="font-medium flex-1">{toolbarNavigation ? '' : 'Notebook'} <span className="text-fg-muted font-normal">· persistent local Python · {kernel?.status || 'status unknown'}{kernel ? ` · epoch ${kernel.epoch}` : ''}</span></span>
      <button type="button" className="canvas-open-button" disabled={disabled} onClick={() => add()}><Plus size={12} />Code</button>
      <button type="button" className="canvas-open-button" disabled={disabled} onClick={() => add('markdown')}><Plus size={12} />Markdown</button>
      <button type="button" className="canvas-open-button" disabled={disabled || !dirty} onClick={() => void save()}><Save size={12} />{saving ? 'Saving…' : dirty ? 'Save changes' : 'Saved'}</button>
      <button type="button" className="canvas-icon" aria-label="Refresh notebook" disabled={disabled || dirty} onClick={() => void load()}><RefreshCw size={13} /></button>
      <button type="button" className="canvas-icon" aria-label="Reset Python worker" disabled={disabled} onClick={() => void reset()}><RotateCcw size={13} /></button>
      <button type="button" className="canvas-open-button" disabled={!running} onClick={async () => { try { const result = await daemonClient.interruptNotebookKernel(key); onShowToast?.(result.interrupted ? 'Worker interrupted. Its in-memory variables were cleared.' : 'No running worker to interrupt.'); } catch (reason) { setError(String(reason)); } }}><Square size={11} />Interrupt</button>
    </header>
    <p className="shrink-0 px-3 py-2 text-[11px] text-fg-muted border-b border-border-default">{workspace ? `Execution workspace: ${workspace.name}` : 'Choose an execution workspace to run code.'} Worker memory is temporary; saved output is not proof of reproducibility or an OS sandbox.</p>
    {error && <div role="alert" className="p-3 text-xs text-rose-500 border-b border-border-default break-words">{error}</div>}
    <div className="flex-1 overflow-auto p-4 space-y-4">
      {!sessionId ? (
        <p className="text-xs text-fg-muted">Open a conversation to use its notebook.</p>
      ) : loading ? (
        <p role="status" className="text-xs text-fg-muted">Loading notebook…</p>
      ) : cells.length === 0 ? (
        <div className="space-y-6 max-w-2xl mx-auto py-4">
          <div className="canvas-empty text-center p-6 space-y-3 border border-border-default rounded-xl bg-surface-1">
            <p className="text-sm font-medium">No notebook cells yet</p>
            <p className="text-xs text-fg-muted">Start by adding a blank cell or selecting a scientific workflow starter below.</p>
            <button type="button" className="canvas-open-button mx-auto" onClick={() => add()}>
              <Plus size={12} /> Add a code cell
            </button>
          </div>
          {onAskAgent && (
            <div>
              <p className="text-[11px] font-semibold text-fg-muted uppercase tracking-wider mb-2">Scientific Workflow Starters</p>
              <WorkflowStarters onPick={(prompt: string) => onAskAgent(prompt)} />
            </div>
          )}
        </div>
      ) : cells.map((cell, index) => <section key={cell.id} className="rounded-lg border border-border-default overflow-hidden">
        <div className="flex items-center gap-2 px-3 py-1.5 bg-surface-1 text-[11px]">
          <span className="font-mono flex-1">{running === cell.id ? 'Running…' : `In [${cell.execution_count ?? ' '}]:`} {cell.wall_ms != null ? `${cell.wall_ms} ms` : ''}</span>
          <select aria-label={`Cell ${index + 1} type`} value={cell.cell_type} disabled={disabled}
            className="rounded border border-border-default bg-canvas text-fg-default p-1"
            onChange={event => change(cells.map(item => item.id === cell.id ? editNotebookCell(item, item.source, event.target.value as NotebookCell['cell_type']) : item))}>
            <option value="code">Python</option><option value="markdown">Markdown</option>
          </select>
          <button type="button" className="canvas-open-button" disabled={disabled || !workspace || !cell.source.trim() || cell.cell_type !== 'code'} onClick={() => setPreviewCellId(cell.id)}><Play size={11} />Run</button>
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
      <p className="text-xs text-fg-muted my-3">This uses the session’s Python worker and can modify files in its environment. Review the exact code before running.</p>
      <div className="flex gap-2"><button type="button" autoFocus className="canvas-open-button" onClick={() => setPreviewCellId(null)}>Cancel</button><button type="button" className="canvas-open-button" onClick={() => void run()}>Run cell</button></div>
    </div>}
  </div>;
};
export const NotebookWorkspacePane = NotebookWorkbenchPane;
