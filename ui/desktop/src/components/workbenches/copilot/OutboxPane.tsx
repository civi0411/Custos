import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AlertTriangle, Check, Copy, RefreshCw, Search, Send, ShieldAlert } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { WorkbenchPane, WorkbenchEmptyState, WorkbenchError } from '@/components/common/WorkbenchPane';
import type { OutboxActionItem } from '@/types/domain';

export interface OutboxPaneProps {
  taskId?: string;
  supportedOperations?: string[];
  onShowToast?: (message: string) => void;
}

export function outboxActions(status: string, operations: string[]) {
  return {
    approve: ['pending', 'pending_approval'].includes(status) && (operations.includes('outbox.approve') || operations.includes('approve')),
    dispatch: status === 'approved' && (operations.includes('outbox.dispatch') || operations.includes('dispatch')),
  };
}

/** Detects protocol kind from recipient URI or action identifier */
function detectProtocol(recipient: string, actionType: string): { label: string; tag: string } {
  const target = (recipient + ' ' + actionType).toLowerCase();
  if (target.includes('mcp:') || target.includes('__') || target.includes('mcp')) {
    return { label: 'MCP Tool', tag: 'mcp' };
  }
  if (target.includes('a2a:') || target.includes('a2a')) {
    return { label: 'A2A Task', tag: 'a2a' };
  }
  if (target.includes('native') || target.includes('cmd') || target.includes('shell')) {
    return { label: 'Native Cmd', tag: 'native' };
  }
  return { label: 'Outbox', tag: 'dispatch' };
}

/** Extracts readable human text from MCP parts, A2A artifacts, or structured payload */
function extractHumanText(rawJson?: string | null): string | null {
  if (!rawJson) return null;
  try {
    const data = JSON.parse(rawJson);
    if (typeof data !== 'object' || data === null) return String(data);

    // A2A artifacts format: { artifacts: [{ parts: [{ text: "..." }] }] }
    if (Array.isArray(data.artifacts)) {
      const parts: string[] = [];
      for (const art of data.artifacts) {
        if (Array.isArray(art.parts)) {
          for (const p of art.parts) {
            if (p && typeof p.text === 'string') parts.push(p.text);
          }
        }
      }
      if (parts.length) return parts.join('\n');
    }

    // Direct message / prompt / content
    if (typeof data.message === 'string') return data.message;
    if (typeof data.prompt === 'string') return data.prompt;
    if (typeof data.instruction === 'string') return data.instruction;
    if (typeof data.body === 'string') return data.body;
    if (typeof data.command === 'string') return `$ ${data.command}`;

    // MCP Call parameters
    const params = data.parameters || data.arguments || data.params;
    if (params && typeof params === 'object') {
      const entries = Object.entries(params);
      if (entries.length) {
        return entries.map(([k, v]) => `${k}: ${typeof v === 'string' ? v : JSON.stringify(v)}`).join('\n');
      }
    }
  } catch {
    return null;
  }
  return null;
}

export function OutboxPane({ taskId, supportedOperations = [], onShowToast }: OutboxPaneProps) {
  const [items, setItems] = useState<OutboxActionItem[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<'extracted' | 'json'>('extracted');
  const epoch = useRef(0);
  const actionPending = useRef(false);

  const refresh = useCallback(async () => {
    const current = ++epoch.current;
    setLoading(true);
    try {
      const records = await daemonClient.listOutbox(taskId);
      if (current !== epoch.current) return;
      const scoped = taskId ? records.filter(item => item.task_id === taskId) : records;
      setItems(scoped);
      setSelectedId(previous => scoped.some(item => item.id === previous) ? previous : scoped[0]?.id || '');
      setError(null);
    } catch (reason) { if (current === epoch.current) setError(String(reason)); }
    finally { if (current === epoch.current) setLoading(false); }
  }, [taskId]);

  useEffect(() => { void refresh(); return () => { epoch.current++; }; }, [refresh]);

  const active = items.find(item => item.id === selectedId);
  const filtered = items.filter(item => (item.title + ' ' + item.recipient + ' ' + item.status).toLowerCase().includes(query.toLowerCase()));
  const actions = outboxActions(active?.status || '', supportedOperations);

  const protocol = useMemo(() => {
    if (!active) return { label: 'Outbox', tag: 'dispatch' };
    return detectProtocol(active.recipient || '', active.action_type || '');
  }, [active]);

  const rawJson = active?.payload_json || null;
  const humanText = useMemo(() => extractHumanText(rawJson), [rawJson]);
  const formattedJson = useMemo(() => {
    if (!rawJson) return active?.payload_summary || '';
    try { return JSON.stringify(JSON.parse(rawJson), null, 2); }
    catch { return rawJson; }
  }, [rawJson, active?.payload_summary]);

  const act = async (kind: 'approve' | 'dispatch') => {
    if (!active || actionPending.current || !actions[kind]) return;
    actionPending.current = true; setActing(true);
    try {
      if (kind === 'approve') { await daemonClient.approveOutbox(active.id); onShowToast?.('Approval recorded.'); }
      else { await daemonClient.dispatchOutbox(active.id); onShowToast?.('Dispatch requested. Check the recorded outcome.'); }
      await refresh();
    } catch (reason) { setError(String(reason)); }
    finally { actionPending.current = false; setActing(false); }
  };

  const copyPayload = () => {
    const content = rawJson || active?.payload_summary || '';
    if (!content) return;
    navigator.clipboard.writeText(content).then(() => {
      onShowToast?.('Payload copied to clipboard.');
    }).catch(() => {});
  };

  return <WorkbenchPane toolbar={<>
    <span className="wb-meta">{items.length} recorded actions{taskId ? ' · selected Task' : ''}</span>
    <span className="wb-toolbar-spacer" />
    <button type="button" className="wb-icon-button" title="Refresh" aria-label="Refresh outbox" disabled={loading || acting} onClick={() => void refresh()}><RefreshCw size={14} /></button>
  </>}>
    <WorkbenchError message={error} onRetry={() => void refresh()} />
    <div className="wb-split flex flex-1 min-h-0">
      <aside className="wb-master min-h-0 flex flex-col border-r border-border-subtle">
        <label className="wb-search"><Search size={13} aria-hidden="true" /><input aria-label="Find an outbox action" placeholder="Find an action…" value={query} onChange={event => setQuery(event.target.value)} /></label>
        <nav aria-label="Outbox actions" className="flex-1 overflow-auto">
          {loading && !items.length ? <WorkbenchEmptyState title="Loading outbox…" busy /> : !filtered.length ? <WorkbenchEmptyState title={query ? 'No matching actions' : 'No recorded actions'} /> : filtered.map(item => {
            const proto = detectProtocol(item.recipient || '', item.action_type || '');
            return (
              <div className={'wb-source-row ' + (selectedId === item.id ? 'is-selected' : '')} key={item.id}>
                <button type="button" disabled={acting} aria-current={selectedId === item.id ? 'true' : undefined} onClick={() => setSelectedId(item.id)}>
                  <div className="flex items-center gap-1.5 w-full">
                    <span className="font-mono text-[9px] px-1 py-0.2 rounded bg-surface-2 text-fg-muted uppercase tracking-wider">{proto.label}</span>
                    <span className="truncate flex-1 text-left">{item.title || item.action_type}</span>
                  </div>
                  <small>{item.status.replace(/_/g, ' ')}{item.created_at ? ' · ' + new Date(item.created_at).toLocaleString() : ''}</small>
                </button>
              </div>
            );
          })}
        </nav>
      </aside>
      <main className="min-w-0 min-h-0 flex-1 overflow-auto">
        {active ? <article className="wb-document select-text">
          <div className="wb-document-actions">
            {actions.approve && <button type="button" className="canvas-open-button" disabled={acting} onClick={() => void act('approve')}><Check size={12} />Approve action</button>}
            {actions.dispatch && <button type="button" className="canvas-open-button" disabled={acting} onClick={() => void act('dispatch')}><Send size={12} />Dispatch approved action</button>}
          </div>
          <div className="flex items-center gap-2 mb-1">
            <span className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-surface-2 text-fg-muted font-medium uppercase">{protocol.label}</span>
            <span className="wb-document-byline font-mono text-xs">{active.status.replace(/_/g, ' ')}</span>
          </div>
          <h1>{active.title || active.action_type}</h1>

          {active.status === 'uncertain' && (
            <div className="flex items-start gap-2 p-3 my-3 rounded border border-amber-500/30 bg-amber-500/10 text-amber-300 text-xs">
              <ShieldAlert size={16} className="shrink-0 mt-0.5 text-amber-400" />
              <div>
                <strong>Uncertain Delivery:</strong> Delivery has not been confirmed. Reconcile the existing attempt before sending again. Blind re-dispatch is forbidden.
              </div>
            </div>
          )}

          {(active.status === 'failed' || active.error_message) && (
            <div className="flex items-start gap-2 p-3 my-3 rounded border border-rose-500/30 bg-rose-500/10 text-rose-300 text-xs">
              <AlertTriangle size={16} className="shrink-0 mt-0.5 text-rose-400" />
              <div>
                <strong>Execution Failure:</strong> {active.error_message || 'The external target rejected or failed to acknowledge execution.'}
              </div>
            </div>
          )}

          <section>
            <div className="flex items-center justify-between mb-2">
              <h2>Recorded Payload</h2>
              <div className="flex items-center gap-1.5">
                {humanText && (
                  <div className="wb-segmented" role="group" aria-label="Payload format">
                    <button type="button" aria-pressed={viewMode === 'extracted'} onClick={() => setViewMode('extracted')}>Readable</button>
                    <button type="button" aria-pressed={viewMode === 'json'} onClick={() => setViewMode('json')}>JSON</button>
                  </div>
                )}
                <button type="button" className="wb-icon-button" title="Copy payload" aria-label="Copy payload" onClick={copyPayload}><Copy size={12} /></button>
              </div>
            </div>

            {viewMode === 'extracted' && humanText ? (
              <div className="p-3 rounded bg-surface-1 border border-border-subtle font-mono text-xs whitespace-pre-wrap leading-relaxed text-fg-default">
                {humanText}
              </div>
            ) : (
              <pre className="p-3 rounded bg-surface-1 border border-border-subtle font-mono text-xs overflow-auto max-h-72 whitespace-pre text-fg-muted">
                {formattedJson || 'Payload contents were not recorded.'}
              </pre>
            )}
          </section>

          <section>
            <h2>Delivery Details</h2>
            <dl className="wb-property-list">
              <dt>Recipient</dt><dd className="font-mono">{active.recipient || 'Not recorded'}</dd>
              <dt>Task</dt><dd className="font-mono">{active.task_id || 'Not recorded'}</dd>
              <dt>Payload digest</dt><dd className="font-mono">{active.payload_digest || 'Not recorded'}</dd>
              <dt>Permit</dt><dd className="font-mono">{active.permit_id || 'Not recorded'}</dd>
              <dt>Receipt</dt><dd className="font-mono">{active.receipt_id || 'No receipt recorded'}</dd>
            </dl>
          </section>

          {!actions.approve && !actions.dispatch && (
            <p className="wb-meta">This view shows the stored action and receipt. Available controls follow the daemon's supported operations and the action's current state.</p>
          )}
        </article> : <WorkbenchEmptyState title="Outbound actions" description="Messages and other external actions appear here with their actual delivery status and receipts." />}
      </main>
    </div>
  </WorkbenchPane>;
}
