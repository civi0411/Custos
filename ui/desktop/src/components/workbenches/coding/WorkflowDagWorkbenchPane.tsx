import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  GitBranch,
  Layers,
  ListTree,
  RefreshCw,
  Search,
  Square,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { WorkbenchPane, WorkbenchEmptyState, WorkbenchError } from '@/components/common/WorkbenchPane';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';
import type { LiveRunSnapshot } from '@/types/domain';
import type { OrcaTabId } from '../OrcaTabbedContainer';

export interface WorkflowDagWorkbenchPaneProps {
  sessionId?: string;
  initialRunId?: string;
  canCancel?: boolean;
  onShowToast?: (msg: string) => void;
  onAskAgent?: (draft: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

type SessionRun = { run_id: string; model?: string; status: string; journal_occurred_at?: string };

export interface DagNodeInfo {
  id: string;
  nodeId: string;
  agentName: string;
  task: string;
  status: 'pending' | 'running' | 'done' | 'failed' | 'skipped';
  response?: string;
  summary?: string;
  batchId?: string;
  kind?: string;
  timestamp?: string;
  children?: DagNodeInfo[];
}

/** Parses live snapshot activities into structured DAG nodes (OpenGateway DagView pattern) */
function parseActivitiesToDag(activities: any[]): DagNodeInfo[] {
  if (!activities || !activities.length) return [];
  const nodes: DagNodeInfo[] = [];

  activities.forEach((act, idx) => {
    const payload = act.payload || {};
    const kind = act.kind || 'activity';
    const nodeId = payload.node_id || payload.step_id || payload.task_id || `step-${idx + 1}`;
    const agentName = payload.agent_name || payload.harness_id || payload.actor || payload.model || (kind.includes('mcp') ? 'MCP Tool' : 'Worker');
    const task = payload.task || payload.instruction || payload.prompt || payload.command || payload.summary || kind;

    let status: DagNodeInfo['status'] = 'done';
    const rawStatus = (payload.status || act.status || '').toLowerCase();
    if (['running', 'active', 'in_progress', 'dispatching'].includes(rawStatus)) {
      status = 'running';
    } else if (['failed', 'error'].includes(rawStatus) || payload.error) {
      status = 'failed';
    } else if (['skipped', 'cancelled', 'canceled'].includes(rawStatus)) {
      status = 'skipped';
    } else if (['pending', 'queued', 'pending_approval'].includes(rawStatus)) {
      status = 'pending';
    }

    const response = typeof payload.response === 'string'
      ? payload.response
      : payload.result
        ? JSON.stringify(payload.result, null, 2)
        : payload.output
          ? typeof payload.output === 'string' ? payload.output : JSON.stringify(payload.output, null, 2)
          : payload.error || undefined;

    const summary = payload.summary || (response && response.length > 140 ? `${response.slice(0, 140)}…` : response);

    nodes.push({
      id: `act-${idx}`,
      nodeId,
      agentName,
      task: String(task),
      status,
      response,
      summary,
      batchId: payload.batch_id || payload.parent_id || undefined,
      kind,
      timestamp: act.timestamp,
    });
  });

  return nodes;
}

/** Groups nodes into execution layers: items sharing batchId are parallel siblings (OpenGateway groupLayers) */
function groupLayers(nodes: DagNodeInfo[]): DagNodeInfo[][] {
  if (!nodes.length) return [];
  const layers: DagNodeInfo[][] = [];
  const batchMap = new Map<string, DagNodeInfo[]>();

  for (const n of nodes) {
    if (n.batchId) {
      if (!batchMap.has(n.batchId)) {
        const batch: DagNodeInfo[] = [];
        batchMap.set(n.batchId, batch);
        layers.push(batch);
      }
      batchMap.get(n.batchId)!.push(n);
    } else {
      layers.push([n]);
    }
  }
  return layers;
}

const STATUS_STYLES: Record<string, { badge: string; border: string; bg: string }> = {
  pending: {
    badge: 'bg-muted/30 text-fg-muted border-border-subtle',
    border: 'border-border-subtle',
    bg: 'bg-surface-1',
  },
  running: {
    badge: 'bg-blue-500/10 text-blue-400 border-blue-500/30 animate-pulse',
    border: 'border-blue-500/40',
    bg: 'bg-blue-500/5',
  },
  done: {
    badge: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20',
    border: 'border-emerald-500/30',
    bg: 'bg-emerald-500/5',
  },
  failed: {
    badge: 'bg-rose-500/10 text-rose-400 border-rose-500/20',
    border: 'border-rose-500/30',
    bg: 'bg-rose-500/5',
  },
  skipped: {
    badge: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
    border: 'border-amber-500/30',
    bg: 'bg-amber-500/5',
  },
};

/** Interactive recursive Call Tree component (OpenGateway CallTree pattern) */
function CallTreeNodeView({ node }: { node: DagNodeInfo }) {
  const [expanded, setExpanded] = useState(false);
  const style = STATUS_STYLES[node.status] || STATUS_STYLES.pending;

  return (
    <div className="ml-2 my-1.5 font-mono text-xs">
      <div
        className={`flex items-center gap-2 p-1.5 rounded cursor-pointer transition hover:bg-surface-2 ${style.border} border`}
        onClick={() => setExpanded(!expanded)}
      >
        <span className="text-fg-muted">
          {node.response ? (expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />) : <span className="inline-block w-3" />}
        </span>
        <span className="font-semibold text-fg-default">{node.agentName}</span>
        <span className="text-fg-muted truncate max-w-xs">{node.task}</span>
        <span className="wb-toolbar-spacer" />
        <span className={`text-[10px] px-1.5 py-0.5 rounded uppercase border font-medium ${style.badge}`}>
          {node.status}
        </span>
      </div>
      {expanded && node.response && (
        <pre className="text-xs mt-1 ml-5 bg-surface-2 p-2.5 rounded border border-border-subtle overflow-auto max-h-48 text-fg-muted whitespace-pre-wrap">
          {node.response}
        </pre>
      )}
    </div>
  );
}

export function WorkflowDagWorkbenchPane({
  sessionId,
  initialRunId,
  canCancel = false,
  onShowToast,
}: WorkflowDagWorkbenchPaneProps) {
  const [runs, setRuns] = useState<SessionRun[]>([]);
  const [selectedRunId, setSelectedRunId] = useState('');
  const [snapshot, setSnapshot] = useState<LiveRunSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);
  const [viewMode, setViewMode] = useState<'dag' | 'tree'>('dag');
  const [searchFilter, setSearchFilter] = useState('');
  const [expandedNodeId, setExpandedNodeId] = useState<string | null>(null);
  const generation = useRef(0);

  const refresh = useCallback(async () => {
    const request = ++generation.current;
    setLoading(true);
    setError(null);
    try {
      const result = sessionId ? await daemonClient.listSessionRuns(sessionId) : [];
      if (request !== generation.current) return;
      setRuns(result);
      setSelectedRunId(prev => {
        if (initialRunId && result.some(r => r.run_id === initialRunId)) return initialRunId;
        if (prev && result.some(r => r.run_id === prev)) return prev;
        const active = result.find(r => ['running', 'active', 'pending'].includes(r.status.toLowerCase()));
        return active?.run_id || result[0]?.run_id || '';
      });
    } catch (err) {
      if (request !== generation.current) return;
      setError(err instanceof Error ? err.message : String(err));
      setRuns([]);
      setSelectedRunId('');
    } finally {
      if (request === generation.current) setLoading(false);
    }
  }, [sessionId, initialRunId]);

  useEffect(() => {
    void refresh();
    return () => { generation.current++; };
  }, [refresh]);

  useEffect(() => {
    if (!sessionId || !selectedRunId) {
      setSnapshot(null);
      return;
    }
    const controller = new AbortController();
    setSnapshot(null);
    daemonClient
      .watchRun(sessionId, selectedRunId, snap => {
        if (!controller.signal.aborted && snap.session_id === sessionId && snap.run_id === selectedRunId) {
          setSnapshot(snap);
        }
      }, controller.signal)
      .catch(err => {
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });
    return () => controller.abort();
  }, [sessionId, selectedRunId]);

  const selectedRun = runs.find(r => r.run_id === selectedRunId);
  const isRunActive = selectedRun ? ['running', 'active', 'pending'].includes((snapshot?.status || selectedRun.status).toLowerCase()) : false;

  const dagNodes = useMemo(() => {
    const raw = parseActivitiesToDag(snapshot?.activities || []);
    if (!searchFilter.trim()) return raw;
    const term = searchFilter.toLowerCase();
    return raw.filter(n => n.agentName.toLowerCase().includes(term) || n.task.toLowerCase().includes(term) || n.nodeId.toLowerCase().includes(term));
  }, [snapshot?.activities, searchFilter]);

  const layers = useMemo(() => groupLayers(dagNodes), [dagNodes]);

  const stopRun = async () => {
    if (!selectedRunId || !canCancel || stopping) return;
    setStopping(true);
    try {
      await daemonClient.cancelRun({ run_id: selectedRunId, reason: 'Cancelled from Workflow DAG' });
      onShowToast?.('Run cancellation requested.');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setStopping(false);
    }
  };

  return (
    <WorkbenchPane
      toolbar={
        <>
          <div className="flex items-center gap-2">
            <Layers size={14} className="text-fg-muted" />
            <span className="font-semibold text-xs text-white">Workflow & Multi-Agent DAG</span>
          </div>

          <select
            className="wb-inline-select max-w-44 font-mono text-xs"
            aria-label="Workflow run"
            value={selectedRunId}
            disabled={loading || !runs.length}
            onChange={e => setSelectedRunId(e.target.value)}
          >
            {!runs.length && <option value="">{loading ? 'Loading runs…' : 'No runs in session'}</option>}
            {runs.map(r => (
              <option key={r.run_id} value={r.run_id}>
                {r.run_id.slice(-10)} · {r.status}
              </option>
            ))}
          </select>

          <div className="wb-toolbar-spacer" />

          <div className="wb-segmented" role="group" aria-label="DAG view representation">
            <button
              type="button"
              aria-pressed={viewMode === 'dag'}
              onClick={() => setViewMode('dag')}
              className="flex items-center gap-1"
            >
              <GitBranch size={11} />
              DAG Layers
            </button>
            <button
              type="button"
              aria-pressed={viewMode === 'tree'}
              onClick={() => setViewMode('tree')}
              className="flex items-center gap-1"
            >
              <ListTree size={11} />
              Call Tree
            </button>
          </div>

          <button
            type="button"
            className="wb-icon-button"
            title="Refresh DAG"
            aria-label="Refresh workflow runs"
            disabled={loading}
            onClick={() => void refresh()}
          >
            <RefreshCw size={14} />
          </button>

          {isRunActive && canCancel && (
            <button
              type="button"
              className="canvas-open-button text-rose-400 border-rose-500/30 hover:bg-rose-500/10"
              disabled={stopping}
              onClick={() => void stopRun()}
            >
              <Square size={11} />
              {stopping ? 'Stopping…' : 'Stop run'}
            </button>
          )}
        </>
      }
    >
      <WorkbenchError message={error} onRetry={() => void refresh()} />

      <div className="flex flex-col flex-1 min-h-0 bg-surface-0">
        {/* Controls strip */}
        <div className="flex items-center justify-between border-b border-border-subtle px-4 py-2 bg-surface-1/50 text-xs">
          <div className="flex items-center gap-3">
            <span className="text-fg-muted font-mono">
              Status:{' '}
              <span className={`font-semibold ${isRunActive ? 'text-blue-400 animate-pulse' : 'text-fg-default'}`}>
                {snapshot?.status || selectedRun?.status || 'Idle'}
              </span>
            </span>
            <span className="text-fg-muted">·</span>
            <span className="text-fg-muted font-mono">
              Nodes: <span className="text-fg-default font-semibold">{dagNodes.length}</span>
            </span>
            <span className="text-fg-muted">·</span>
            <span className="text-fg-muted font-mono">
              Layers: <span className="text-fg-default font-semibold">{layers.length}</span>
            </span>
          </div>

          <div className="relative w-48">
            <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3 text-fg-muted" />
            <input
              type="text"
              placeholder="Filter nodes…"
              value={searchFilter}
              onChange={e => setSearchFilter(e.target.value)}
              className="w-full bg-surface-2 border border-border-subtle rounded py-0.5 pl-6 pr-2 text-xs text-fg-default placeholder-fg-muted/60 focus:outline-none focus:border-workbench-accent"
            />
          </div>
        </div>

        {/* Visualization area */}
        <div className="flex-1 overflow-auto p-6">
          {!selectedRunId ? (
            <WorkbenchEmptyState
              title="No run selected"
              description="Select or launch a Task run in this session to visualize its multi-agent execution DAG."
            />
          ) : !dagNodes.length ? (
            <WorkbenchEmptyState
              title={isRunActive ? 'Workflow starting…' : 'No recorded activity nodes'}
              busy={isRunActive}
              description={
                isRunActive
                  ? 'Waiting for agent steps and tool activities to be recorded…'
                  : 'This run did not record any structured activity steps or subagent DAG nodes.'
              }
            />
          ) : viewMode === 'tree' ? (
            /* Call Tree View (OpenGateway CallTree) */
            <div className="max-w-3xl mx-auto space-y-1">
              <div className="text-xs font-semibold text-fg-muted uppercase tracking-wider mb-3">
                Hierarchical Invocation Tree
              </div>
              {dagNodes.map(node => (
                <CallTreeNodeView key={node.id} node={node} />
              ))}
            </div>
          ) : (
            /* Layered DAG View (OpenGateway DagView) */
            <div className="flex flex-col items-center max-w-4xl mx-auto space-y-6">
              {layers.map((layer, layerIdx) => (
                <div key={layerIdx} className="w-full flex flex-col items-center">
                  {/* Layer nodes (parallel siblings) */}
                  <div
                    className={`grid gap-4 w-full justify-center ${
                      layer.length === 1
                        ? 'grid-cols-1 max-w-lg'
                        : layer.length === 2
                          ? 'grid-cols-2 max-w-2xl'
                          : 'grid-cols-3 max-w-3xl'
                    }`}
                  >
                    {layer.map(node => {
                      const style = STATUS_STYLES[node.status] || STATUS_STYLES.pending;
                      const isExpanded = expandedNodeId === node.id;

                      return (
                        <div
                          key={node.id}
                          className={`flex flex-col rounded-xl border ${style.border} ${style.bg} p-4 transition shadow-sm hover:shadow-md cursor-pointer`}
                          onClick={() => setExpandedNodeId(isExpanded ? null : node.id)}
                        >
                          <div className="flex items-center justify-between gap-2 mb-2">
                            <div className="flex items-center gap-2">
                              {getAgentBrandIcon(node.agentName.toLowerCase(), 14, 'shrink-0')}
                              <span className="font-semibold text-xs text-fg-default truncate">
                                {node.agentName}
                              </span>
                            </div>
                            <span
                              className={`text-[9px] font-mono px-1.5 py-0.5 rounded uppercase font-medium border ${style.badge}`}
                            >
                              {node.status}
                            </span>
                          </div>

                          <div className="text-xs text-fg-default/90 font-medium line-clamp-2 mb-2 leading-relaxed">
                            {node.task}
                          </div>

                          {node.summary && !isExpanded && (
                            <div className="text-[11px] text-fg-muted font-mono truncate bg-surface-2/60 px-2 py-1 rounded">
                              {node.summary}
                            </div>
                          )}

                          {isExpanded && node.response && (
                            <div className="mt-2 pt-2 border-t border-border-subtle/60">
                              <span className="text-[10px] text-fg-muted uppercase tracking-wider font-semibold block mb-1">
                                Output / Response:
                              </span>
                              <pre className="text-xs font-mono bg-surface-2 p-2 rounded border border-border-subtle max-h-40 overflow-auto whitespace-pre-wrap text-fg-default">
                                {node.response}
                              </pre>
                            </div>
                          )}

                          <div className="flex items-center justify-between text-[10px] text-fg-muted font-mono mt-3 pt-1 border-t border-border-subtle/30">
                            <span>{node.nodeId}</span>
                            {node.timestamp && <span>{new Date(node.timestamp).toLocaleTimeString()}</span>}
                          </div>
                        </div>
                      );
                    })}
                  </div>

                  {/* Connector arrow between sequential layers */}
                  {layerIdx < layers.length - 1 && (
                    <div className="flex flex-col items-center my-2">
                      <div className="w-0.5 h-6 bg-border-default/60" />
                      <div className="w-2 h-2 rotate-45 border-b-2 border-r-2 border-border-default/80 -mt-1" />
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </WorkbenchPane>
  );
}
