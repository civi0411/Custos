import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  Activity,
  Play,
  StopCircle,
  CheckCircle2,
  AlertTriangle,
  Clock,
  Cpu,
  Layers,
  ShieldCheck,
  Terminal,
  Sparkles,
  RefreshCw,
  MessageSquare,
  X,
  ChevronRight,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { StartRunParams, CancelRunParams } from '@/types/domain';

export interface WorkflowDagWorkbenchPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onAskAgent?: (draft: string) => void;
  onOpenTab?: (id: any) => void;
}

export type DagNodeStatus = 'idle' | 'running' | 'completed' | 'failed' | 'skipped';

export interface DagNode {
  id: string;
  stage: string;
  label: string;
  description: string;
  icon: React.ElementType;
  status: DagNodeStatus;
  durationMs?: number;
  tokensUsed?: number;
  toolsInvoked?: number;
  evidenceId?: string;
  dependencies: string[];
  outputSummary?: string;
  logs?: string[];
}

interface SessionRunItem {
  run_id: string;
  model?: string;
  status: string;
  journal_occurred_at?: string;
}

export const WorkflowDagWorkbenchPane: React.FC<WorkflowDagWorkbenchPaneProps> = ({
  sessionId,
  onShowToast,
  onAskAgent,
  onOpenTab,
}) => {
  const [runs, setRuns] = useState<SessionRunItem[]>([]);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [selectedNodeId, setSelectedNodeId] = useState<string>('');
  const [nodes, setNodes] = useState<DagNode[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [isStartingRun, setIsStartingRun] = useState<boolean>(false);
  const [isStartModalOpen, setIsStartModalOpen] = useState<boolean>(false);

  // New run params
  const [requestedModel, setRequestedModel] = useState<string>('claude-3-7-sonnet');
  const [actorName, setActorName] = useState<string>('developer');

  // Watch selected run and project real DAG nodes from live activities
  useEffect(() => {
    if (!sessionId || !selectedRunId) {
      setNodes([]);
      setSelectedNodeId('');
      return;
    }

    const controller = new AbortController();
    daemonClient
      .watchRun(
        sessionId,
        selectedRunId,
        (snapshot) => {
          if (!snapshot || !snapshot.activities || snapshot.activities.length === 0) {
            setNodes([]);
            setSelectedNodeId('');
            return;
          }

          const projected: DagNode[] = snapshot.activities.map((act, idx) => {
            const id = `node-${idx}`;
            const kind = act.kind || 'activity';
            const payloadStr = typeof act.payload === 'object' && act.payload !== null
              ? JSON.stringify(act.payload, null, 2)
              : String(act.payload || '');

            return {
              id,
              stage: `Stage ${idx + 1}`,
              label: kind === 'tool_input' ? 'Tool Call' : kind === 'tool_output' ? 'Tool Result' : kind === 'reasoning_summary' ? 'Reasoning' : kind,
              description: payloadStr.slice(0, 140) || 'Execution activity event',
              icon: kind.includes('tool')
                ? Terminal
                : kind.includes('reason') || kind.includes('plan')
                ? Layers
                : kind.includes('test') || kind.includes('verify')
                ? ShieldCheck
                : Activity,
              status: 'completed' as DagNodeStatus,
              dependencies: idx > 0 ? [`node-${idx - 1}`] : [],
              outputSummary: payloadStr.slice(0, 200),
              logs: [payloadStr],
            };
          });

          setNodes(projected);
          setSelectedNodeId((prev) => (projected.some((n) => n.id === prev) ? prev : projected[0]?.id || ''));
        },
        controller.signal
      )
      .catch(() => {});

    return () => controller.abort();
  }, [sessionId, selectedRunId]);

  const loadRuns = useCallback(async () => {
    if (!sessionId) return;
    setIsLoading(true);
    try {
      const list = await daemonClient.listSessionRuns(sessionId);
      setRuns(list);
      if (list.length > 0 && !selectedRunId) {
        setSelectedRunId(list[0].run_id);
      }
    } catch {
      // Backend may return empty if no runs started yet
      setRuns([]);
    } finally {
      setIsLoading(false);
    }
  }, [sessionId, selectedRunId]);

  useEffect(() => {
    loadRuns();
  }, [loadRuns]);

  const selectedNode = useMemo(
    () => (nodes.length > 0 ? (nodes.find((n) => n.id === selectedNodeId) ?? nodes[0]) : null),
    [nodes, selectedNodeId]
  );

  const totalTokens = useMemo(
    () => nodes.reduce((sum, n) => sum + (n.tokensUsed ?? 0), 0),
    [nodes]
  );

  const totalDurationMs = useMemo(
    () => nodes.reduce((sum, n) => sum + (n.durationMs ?? 0), 0),
    [nodes]
  );

  const handleStartRun = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!sessionId) {
      onShowToast?.('No active session connected.');
      return;
    }
    setIsStartingRun(true);
    try {
      const params: StartRunParams = {
        session_id: sessionId,
        task_id: `task_${Date.now()}`,
        model: requestedModel,
        actor: actorName,
        background: false,
      };
      const handle = await daemonClient.startRun(params);
      onShowToast?.(`Workflow run started (ID: ${handle.run_id}).`);
      setIsStartModalOpen(false);
      await loadRuns();
      setSelectedRunId(handle.run_id);
    } catch (err) {
      onShowToast?.(`Failed to start run: ${String(err)}`);
    } finally {
      setIsStartingRun(false);
    }
  };

  const handleCancelRun = async () => {
    if (!selectedRunId) return;
    try {
      const params: CancelRunParams = {
        run_id: selectedRunId,
        reason: 'User canceled via Workflow DAG Workbench',
      };
      await daemonClient.cancelRun(params);
      onShowToast?.(`Run ${selectedRunId} cancellation requested.`);
      await loadRuns();
    } catch (err) {
      onShowToast?.(`Failed to cancel run: ${String(err)}`);
    }
  };

  const handleAskAboutStep = () => {
    if (!onAskAgent || !selectedNode) return;
    const prompt = `Review workflow step "${selectedNode.label}" (${selectedNode.stage}):
Status: ${selectedNode.status}
Evidence: ${selectedNode.evidenceId || 'None'}
Output: ${selectedNode.outputSummary || 'None'}

Please explain the execution state and suggest the next optimal action.`;
    onAskAgent(prompt);
    onShowToast?.('Sent step context to chat assistant.');
  };

  const getStatusBadge = (status: DagNodeStatus) => {
    switch (status) {
      case 'completed':
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
            <CheckCircle2 size={11} />
            <span>Passed</span>
          </span>
        );
      case 'running':
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono bg-amber-500/10 text-amber-300 border border-amber-500/20">
            <RefreshCw size={11} className="animate-spin" />
            <span>Running</span>
          </span>
        );
      case 'failed':
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono bg-red-500/10 text-red-400 border border-red-500/20">
            <AlertTriangle size={11} />
            <span>Failed</span>
          </span>
        );
      default:
        return (
          <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-muted)]">
            <Clock size={11} />
            <span>Idle</span>
          </span>
        );
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] text-xs overflow-hidden select-none font-sans relative">
      {/* 48px Standard Pane Header */}
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5 z-10">
        <div className="flex items-center gap-2.5">
          <Activity className="w-4 h-4 workbench-accent" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
            Workflow Execution DAG
          </span>
          <span className="text-[10px] text-[var(--color-fg-muted)] font-mono font-medium bg-[var(--color-surface-2)] px-2 py-0.5 rounded border border-[var(--color-border-muted)] ml-1">
            {nodes.length} stages • {totalTokens.toLocaleString()} tokens • {(totalDurationMs / 1000).toFixed(1)}s
          </span>
        </div>

        <div className="flex items-center gap-2">
          {selectedRunId && (
            <button
              onClick={handleCancelRun}
              className="flex items-center gap-1 px-2.5 py-1 bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/20 rounded-md text-[11px] font-medium transition"
              title="Cancel the active workflow run"
            >
              <StopCircle size={12} />
              <span>Cancel Run</span>
            </button>
          )}

          <button
            onClick={() => setIsStartModalOpen(true)}
            className="flex items-center gap-1.5 px-3 py-1 bg-[var(--workbench-accent)] hover:opacity-90 rounded-md text-white font-medium text-[11px] transition shadow-xs"
          >
            <Play size={11} />
            <span>Dispatch Run</span>
          </button>

          <button
            onClick={loadRuns}
            className="p-1.5 hover:bg-[var(--color-surface-2)] rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
            title="Refresh session runs"
          >
            <RefreshCw size={13} className={isLoading ? 'animate-spin' : ''} />
          </button>
        </div>
      </header>

      {/* Main DAG Content Split */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: DAG Graph Visualizer */}
        <div className="flex-1 flex flex-col bg-[var(--color-canvas)] border-r border-[var(--color-border-muted)] overflow-hidden">
          {/* Top Metric Bar */}
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/50">
            <div className="flex items-center gap-4 text-[11px] text-[var(--color-fg-muted)] font-mono">
              <span className="flex items-center gap-1.5">
                <Cpu size={12} className="workbench-accent" />
                <span>Model: {requestedModel}</span>
              </span>
              <span>•</span>
              <span className="flex items-center gap-1.5">
                <Clock size={12} className="workbench-accent" />
                <span>Elapsed: {(totalDurationMs / 1000).toFixed(2)}s</span>
              </span>
              <span>•</span>
              <span className="flex items-center gap-1.5">
                <Sparkles size={12} className="workbench-accent" />
                <span>Tokens: {totalTokens.toLocaleString()}</span>
              </span>
            </div>

            <div className="flex items-center gap-1.5">
              <span
                className={`w-2 h-2 rounded-full ${
                  nodes.length > 0 ? 'bg-emerald-400 animate-pulse' : 'bg-fg-subtle'
                }`}
              />
              <span
                className={`text-[10px] font-mono font-medium uppercase tracking-wider ${
                  nodes.length > 0 ? 'text-emerald-400' : 'text-fg-muted'
                }`}
              >
                {nodes.length > 0 ? 'DAG Flow Active' : 'Pipeline Idle'}
              </span>
            </div>
          </div>

          {/* Graph Pipeline View */}
          <div className="flex-1 overflow-y-auto p-5 space-y-4">
            {nodes.length === 0 ? (
              <div className="h-full flex flex-col items-center justify-center p-8 text-center text-fg-muted min-h-[280px]">
                <div className="w-12 h-12 rounded-xl bg-surface-2 border border-border-default flex items-center justify-center mb-3 text-fg-muted">
                  <Activity size={22} />
                </div>
                <h3 className="text-sm font-medium text-fg-editor">No Active Workflow Graph</h3>
                <p className="text-xs text-fg-muted mt-1 max-w-sm">
                  This session has no running or completed execution DAG. Start a task or dispatch a run to visualize the multi-stage pipeline.
                </p>
                <button
                  type="button"
                  onClick={() => setIsStartModalOpen(true)}
                  className="mt-4 flex items-center gap-1.5 px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border-default rounded-lg text-xs font-medium text-fg-editor transition cursor-pointer"
                >
                  <Play size={12} className="workbench-accent" />
                  <span>Dispatch Run</span>
                </button>
              </div>
            ) : (
              <div className="max-w-2xl mx-auto space-y-3">
              {nodes.map((node, idx) => {
                const Icon = node.icon;
                const isSelected = node.id === selectedNodeId;
                const isLast = idx === nodes.length - 1;

                return (
                  <React.Fragment key={node.id}>
                    {/* DAG Node Card */}
                    <div
                      onClick={() => setSelectedNodeId(node.id)}
                      className={`p-4 rounded-xl border transition-all cursor-pointer relative shadow-xs ${isSelected
                          ? 'bg-[var(--color-surface-0)] border-[var(--workbench-accent)] ring-1 ring-[var(--workbench-accent)]/30'
                          : 'bg-[var(--color-surface-1)] border-[var(--color-border-default)] hover:border-[var(--color-border-muted)] hover:bg-[var(--color-surface-2)]/60'
                        }`}
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex items-start gap-3">
                          <div
                            className={`p-2.5 rounded-lg border ${isSelected
                                ? 'bg-[var(--workbench-accent)]/10 border-[var(--workbench-accent)]/30 text-[var(--workbench-accent)]'
                                : 'bg-[var(--color-surface-2)] border-[var(--color-border-muted)] text-[var(--color-fg-muted)]'
                              }`}
                          >
                            <Icon size={18} />
                          </div>

                          <div className="space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="text-[10px] font-mono text-[var(--color-fg-muted)] uppercase tracking-wider">
                                {node.stage}
                              </span>
                              <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
                                {node.label}
                              </span>
                            </div>
                            <p className="text-[11px] text-[var(--color-fg-muted)] leading-relaxed">
                              {node.description}
                            </p>
                          </div>
                        </div>

                        <div className="flex flex-col items-end gap-2 shrink-0">
                          {getStatusBadge(node.status)}
                          {node.durationMs != null && (
                            <span className="text-[10px] font-mono text-[var(--color-fg-subtle)]">
                              {node.durationMs}ms
                            </span>
                          )}
                        </div>
                      </div>

                      {/* Node Metadata Footer */}
                      <div className="mt-3 pt-2.5 border-t border-[var(--color-border-muted)] flex items-center justify-between text-[10px] text-[var(--color-fg-muted)] font-mono">
                        <div className="flex items-center gap-3">
                          {node.evidenceId && (
                            <span className="flex items-center gap-1 text-emerald-400">
                              <ShieldCheck size={11} />
                              <span>{node.evidenceId}</span>
                            </span>
                          )}
                          {node.tokensUsed != null && (
                            <span>{node.tokensUsed} tok</span>
                          )}
                          {node.toolsInvoked != null && (
                            <span>{node.toolsInvoked} tools</span>
                          )}
                        </div>

                        <span className="flex items-center gap-1 text-[var(--workbench-accent)]">
                          <span>Inspect Step</span>
                          <ChevronRight size={11} />
                        </span>
                      </div>
                    </div>

                    {/* Directed Edge Connector Line */}
                    {!isLast && (
                      <div className="flex items-center justify-center py-0.5">
                        <div className="flex flex-col items-center">
                          <div className="w-0.5 h-3 bg-[var(--color-border-default)]" />
                          <div className="w-1.5 h-1.5 rounded-full bg-[var(--workbench-accent)] opacity-60" />
                          <div className="w-0.5 h-3 bg-[var(--color-border-default)]" />
                        </div>
                      </div>
                    )}
                  </React.Fragment>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Right: Step Inspector Drawer */}
      <div className="w-80 flex flex-col bg-[var(--color-surface-1)] border-l border-[var(--color-border-muted)] overflow-hidden">
        {/* Drawer Header */}
        <div className="p-3.5 border-b border-[var(--color-border-muted)] flex items-center justify-between bg-[var(--color-surface-1)]">
          <div className="space-y-0.5">
            <span className="text-[10px] font-mono text-[var(--color-fg-muted)] uppercase tracking-wider">
              Step Inspector
            </span>
            <h3 className="text-xs font-semibold text-[var(--color-editor-fg)] truncate max-w-[200px]">
              {selectedNode ? selectedNode.label : 'Inspection Panel'}
            </h3>
          </div>
          {selectedNode && getStatusBadge(selectedNode.status)}
        </div>

        {/* Drawer Body */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {!selectedNode ? (
            <div className="h-44 flex flex-col items-center justify-center text-center text-fg-subtle p-4">
              <p className="text-xs text-fg-muted">No step selected</p>
              <p className="text-[11px] text-fg-subtle mt-1 max-w-[200px]">
                Select a stage from the pipeline graph to inspect trace, metrics, and tokens.
              </p>
            </div>
          ) : (
            <>
              {/* Step Overview */}
              <div className="space-y-1.5">
                <span className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                  Stage Objective
                </span>
                <p className="text-xs text-[var(--color-editor-fg)] leading-relaxed bg-[var(--color-surface-0)] p-3 rounded-lg border border-[var(--color-border-muted)]">
                  {selectedNode.description}
                </p>
              </div>

              {/* Output Summary */}
              {selectedNode.outputSummary && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                    Materialized Output
                  </span>
                  <div className="text-xs text-emerald-300 font-mono bg-emerald-500/5 p-3 rounded-lg border border-emerald-500/20">
                    {selectedNode.outputSummary}
                  </div>
                </div>
              )}

              {/* Step Metrics Grid */}
              <div className="grid grid-cols-2 gap-2 text-center">
                <div className="p-2.5 rounded-lg bg-[var(--color-surface-0)] border border-[var(--color-border-muted)]">
                  <div className="text-[10px] text-[var(--color-fg-muted)] font-mono">Duration</div>
                  <div className="text-xs font-bold text-[var(--color-editor-fg)] font-mono mt-0.5">
                    {selectedNode.durationMs ?? 0}ms
                  </div>
                </div>
                <div className="p-2.5 rounded-lg bg-[var(--color-surface-0)] border border-[var(--color-border-muted)]">
                  <div className="text-[10px] text-[var(--color-fg-muted)] font-mono">Tokens</div>
                  <div className="text-xs font-bold text-[var(--color-editor-fg)] font-mono mt-0.5">
                    {selectedNode.tokensUsed ?? 0}
                  </div>
                </div>
              </div>

              {/* Execution Trace Logs */}
              {selectedNode.logs && selectedNode.logs.length > 0 && (
                <div className="space-y-1.5">
                  <span className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                    Trace Log
                  </span>
                  <div className="p-2.5 rounded-lg bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] font-mono text-[10px] text-[var(--color-fg-muted)] space-y-1 overflow-x-auto">
                    {selectedNode.logs.map((log, i) => (
                      <div key={i} className="whitespace-pre-wrap">
                        {log}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Context Steer Action */}
              <div className="pt-2">
                <button
                  type="button"
                  onClick={handleAskAboutStep}
                  className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-lg text-xs font-medium text-[var(--color-editor-fg)] transition shadow-xs cursor-pointer"
                >
                  <MessageSquare size={13} className="workbench-accent" />
                  <span>Ask Agent About Step</span>
                </button>
              </div>
            </>
          )}

            {/* Session Runs History */}
            <div className="pt-3 border-t border-[var(--color-border-muted)] space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                  Session Runs ({runs.length})
                </span>
                <div className="flex items-center gap-2">
                  {onOpenTab && (
                    <button
                      onClick={() => onOpenTab('runs')}
                      className="text-[10px] text-[var(--workbench-accent)] hover:underline"
                    >
                      Runs Ledger
                    </button>
                  )}
                  <button
                    onClick={loadRuns}
                    className="text-[10px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
                  >
                    Refresh
                  </button>
                </div>
              </div>

              {runs.length === 0 ? (
                <div className="p-3 text-center text-[10px] text-[var(--color-fg-muted)] bg-[var(--color-surface-0)] rounded-lg border border-[var(--color-border-muted)]">
                  No previous runs in this session. Click &quot;Dispatch Run&quot; to execute.
                </div>
              ) : (
                <div className="space-y-1.5">
                  {runs.map((r) => (
                    <div
                      key={r.run_id}
                      onClick={() => setSelectedRunId(r.run_id)}
                      className={`p-2 rounded-lg border cursor-pointer text-[10px] font-mono transition ${r.run_id === selectedRunId
                          ? 'bg-[var(--color-surface-0)] border-[var(--workbench-accent)] text-[var(--color-editor-fg)]'
                          : 'bg-[var(--color-surface-0)]/60 border-[var(--color-border-subtle)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-0)]'
                        }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-semibold truncate max-w-[140px]">{r.run_id}</span>
                        <span className="text-emerald-400 capitalize">{r.status}</span>
                      </div>
                      {r.model && (
                        <div className="text-[9px] text-[var(--color-fg-subtle)] mt-0.5 truncate">
                          Model: {r.model}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Modal: Dispatch Run */}
      {isStartModalOpen && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
              <span className="font-semibold text-[var(--color-editor-fg)] text-xs">
                Dispatch Workflow Run
              </span>
              <button
                onClick={() => setIsStartModalOpen(false)}
                className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleStartRun} className="space-y-3">
              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">
                  Requested Model / Harness
                </label>
                <input
                  type="text"
                  value={requestedModel}
                  onChange={(e) => setRequestedModel(e.target.value)}
                  placeholder="e.g. claude-3-7-sonnet or gpt-4o"
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                  required
                />
              </div>

              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">
                  Actor Identity
                </label>
                <input
                  type="text"
                  value={actorName}
                  onChange={(e) => setActorName(e.target.value)}
                  placeholder="e.g. developer or custos_agent"
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                  required
                />
              </div>

              <div className="p-3 rounded-lg bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] text-[11px] text-[var(--color-fg-muted)] space-y-1">
                <span className="font-semibold text-[var(--color-editor-fg)] block">
                  Admission Policy & Guarantees
                </span>
                <p>
                  Execution will pass through the admission gate, verifying session integrity,
                  evidence tracking, and fail-closed security invariants before tools are dispatched.
                </p>
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
                <button
                  type="button"
                  onClick={() => setIsStartModalOpen(false)}
                  className="px-3 py-1.5 rounded-md bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] text-xs border border-[var(--color-border-default)]"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={isStartingRun}
                  className="px-3.5 py-1.5 rounded-md bg-[var(--workbench-accent)] hover:opacity-90 disabled:opacity-50 text-white text-xs font-medium shadow-xs flex items-center gap-1.5"
                >
                  <Play size={11} />
                  <span>{isStartingRun ? 'Dispatching...' : 'Dispatch Run'}</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
