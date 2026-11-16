import React, { useEffect, useMemo, useState } from 'react';
import {
  Activity,
  Clock,
  ExternalLink,
  RefreshCw,
  Square,
  X,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { RunWaterfallView, RunStepSpan } from '@/types/workflow_telemetry';
import { RunWaterfallTimeline } from './RunWaterfallTimeline';
import { FindingsFeed } from './FindingsFeed';
import { ErrorGroupsTable } from './ErrorGroupsTable';
import { StructuredLogStream } from './StructuredLogStream';
import { resolveWorkbenchStrategy } from './workbenchWatchdogStrategy';

export interface RunWaterfallInspectorProps {
  runId: string;
  sessionId?: string;
  taskId?: string;
  projectName?: string;
  workspaceName?: string;
  worktreeBranch?: string;
  onRevealSession?: (sessionId: string, taskId?: string) => void;
  onCancelRun?: (runId: string) => Promise<void>;
  onClose?: () => void;
}

export const RunWaterfallInspector: React.FC<RunWaterfallInspectorProps> = ({
  runId,
  sessionId,
  taskId,
  projectName,
  workspaceName,
  worktreeBranch,
  onRevealSession,
  onCancelRun,
  onClose,
}) => {
  const [detail, setDetail] = useState<RunWaterfallView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [activeTab, setActiveTab] = useState<'timeline' | 'findings' | 'logs' | 'errors'>('timeline');
  const [selectedSpan, setSelectedSpan] = useState<RunStepSpan | null>(null);
  const [cancelling, setCancelling] = useState(false);

  const fetchWaterfall = async () => {
    if (!runId) return;
    setLoading(true);
    setError('');
    try {
      const data = await daemonClient.getRunWaterfall(runId);
      setDetail(data);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    setSelectedSpan(null);
    void fetchWaterfall();
  }, [runId]);

  const handleCancel = async () => {
    if (!runId || cancelling) return;
    setCancelling(true);
    try {
      if (onCancelRun) {
        await onCancelRun(runId);
      } else {
        await daemonClient.cancelRun({ run_id: runId, reason: 'User requested cancellation from Inspector' });
      }
      await fetchWaterfall();
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setCancelling(false);
    }
  };

  const strategy = resolveWorkbenchStrategy(detail?.run.workbench_id);

  // Latency breakdown calculation
  const latency = useMemo(() => {
    if (!detail?.spans) return { llm: 0, tool: 0, harness: 0, other: 0, total: 1 };
    let llm = 0;
    let tool = 0;
    let harness = 0;
    let other = 0;
    for (const s of detail.spans) {
      const d = s.duration_ms ?? 0;
      if (s.kind === 'llm') llm += d;
      else if (s.kind === 'tool') tool += d;
      else if (s.kind === 'harness') harness += d;
      else other += d;
    }
    const total = Math.max(1, llm + tool + harness + other);
    return { llm, tool, harness, other, total };
  }, [detail?.spans]);

  const llmPct = Math.round((latency.llm / latency.total) * 100);
  const toolPct = Math.round((latency.tool / latency.total) * 100);
  const harnessPct = Math.round((latency.harness / latency.total) * 100);
  const otherPct = Math.max(0, 100 - llmPct - toolPct - harnessPct);

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-[#101113] text-[#f4f4f5]">
      {/* Top Header */}
      <header className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-white/10 bg-[#141518] px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <Activity size={18} className="shrink-0 text-sky-400" />
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-mono text-xs font-semibold text-white">{runId}</span>
              <span
                className={`rounded-full px-2 py-0.5 font-mono text-[9px] uppercase font-semibold border ${
                  detail?.run.status === 'running'
                    ? 'border-amber-500/30 bg-amber-500/15 text-amber-300 animate-pulse'
                    : detail?.run.status === 'success'
                    ? 'border-emerald-500/30 bg-emerald-500/15 text-emerald-300'
                    : 'border-rose-500/30 bg-rose-500/15 text-rose-300'
                }`}
              >
                {detail?.run.status ?? 'unknown'}
              </span>
              <span
                className={`rounded px-1.5 py-0.5 font-mono text-[9px] uppercase border ${strategy.badgeClass}`}
              >
                {strategy.name}
              </span>
            </div>
            {/* 3-Tier Lineage Chip */}
            <div className="mt-1 flex flex-wrap items-center gap-2 font-mono text-[10px] text-white/50">
              {projectName && <span>📁 {projectName}</span>}
              {workspaceName && <span>/ {workspaceName}</span>}
              {worktreeBranch && (
                <span className="rounded bg-white/[0.06] border border-white/10 px-1 text-white/80">
                  🌿 {worktreeBranch}
                </span>
              )}
              {detail?.run.agent && <span>· {detail.run.agent}</span>}
            </div>
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2">
          {sessionId && onRevealSession && (
            <button
              type="button"
              onClick={() => onRevealSession(sessionId, taskId)}
              className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.04] px-2.5 py-1.5 text-xs font-medium text-white/80 hover:bg-white/[0.08] hover:text-white transition"
            >
              <span>Open in Conversation</span>
              <ExternalLink size={13} />
            </button>
          )}

          {detail?.run.status === 'running' && (
            <button
              type="button"
              onClick={() => void handleCancel()}
              disabled={cancelling}
              className="inline-flex items-center gap-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 px-2.5 py-1.5 text-xs font-medium text-rose-300 hover:bg-rose-500/20 transition disabled:opacity-50"
            >
              <Square size={13} className="fill-current" />
              <span>{cancelling ? 'Cancelling…' : 'Cancel Run'}</span>
            </button>
          )}

          <button
            type="button"
            onClick={() => void fetchWaterfall()}
            disabled={loading}
            aria-label="Refresh run"
            className="rounded p-1.5 text-white/60 hover:bg-white/10 hover:text-white transition disabled:opacity-50"
          >
            <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          </button>

          {onClose && (
            <button
              type="button"
              onClick={onClose}
              aria-label="Close inspector"
              className="rounded p-1.5 text-white/60 hover:bg-white/10 hover:text-white transition"
            >
              <X size={16} />
            </button>
          )}
        </div>
      </header>

      {/* Latency Breakdown Bar */}
      <div className="flex shrink-0 items-center justify-between border-b border-white/8 bg-[#141518]/70 px-4 py-2 text-[10px] font-mono">
        <div className="flex items-center gap-3">
          <Clock size={13} className="text-white/40" />
          <span className="text-white/60">
            Duration:{' '}
            <strong className="text-white">
              {detail?.run.duration_ms
                ? `${(detail.run.duration_ms / 1000).toFixed(2)}s`
                : 'Active'}
            </strong>
          </span>
          <span className="text-white/30">|</span>
          <span className="text-violet-400">LLM: {llmPct}%</span>
          <span className="text-sky-400">Tool: {toolPct}%</span>
          <span className="text-white/70">Harness: {harnessPct}%</span>
          {otherPct > 0 && <span className="text-white/40">Other: {otherPct}%</span>}
        </div>

        {/* Progress track */}
        <div className="flex h-1.5 w-32 overflow-hidden rounded-full bg-white/10">
          <div style={{ width: `${llmPct}%` }} className="bg-violet-500" title={`LLM: ${llmPct}%`} />
          <div style={{ width: `${toolPct}%` }} className="bg-sky-500" title={`Tool: ${toolPct}%`} />
          <div style={{ width: `${harnessPct}%` }} className="bg-white/50" title={`Harness: ${harnessPct}%`} />
          <div style={{ width: `${otherPct}%` }} className="bg-white/20" title={`Other: ${otherPct}%`} />
        </div>
      </div>

      {/* Tab Selector */}
      <div className="flex shrink-0 items-center gap-2 border-b border-white/8 bg-[#101113] px-4 py-1.5 text-xs">
        <button
          type="button"
          aria-pressed={activeTab === 'timeline'}
          onClick={() => setActiveTab('timeline')}
          className={`rounded-md px-2.5 py-1 font-medium transition ${
            activeTab === 'timeline'
              ? 'bg-white/[0.12] text-white shadow-xs'
              : 'text-white/60 hover:text-white'
          }`}
        >
          Waterfall Timeline ({detail?.spans.length ?? 0})
        </button>
        <button
          type="button"
          aria-pressed={activeTab === 'findings'}
          onClick={() => setActiveTab('findings')}
          className={`rounded-md px-2.5 py-1 font-medium transition ${
            activeTab === 'findings'
              ? 'bg-white/[0.12] text-white shadow-xs'
              : 'text-white/60 hover:text-white'
          }`}
        >
          Watchdog Findings ({detail?.findings.length ?? 0})
        </button>
        <button
          type="button"
          aria-pressed={activeTab === 'logs'}
          onClick={() => setActiveTab('logs')}
          className={`rounded-md px-2.5 py-1 font-medium transition ${
            activeTab === 'logs'
              ? 'bg-white/[0.12] text-white shadow-xs'
              : 'text-white/60 hover:text-white'
          }`}
        >
          Correlated Logs ({detail?.logs.length ?? 0})
        </button>
        <button
          type="button"
          aria-pressed={activeTab === 'errors'}
          onClick={() => setActiveTab('errors')}
          className={`rounded-md px-2.5 py-1 font-medium transition ${
            activeTab === 'errors'
              ? 'bg-white/[0.12] text-white shadow-xs'
              : 'text-white/60 hover:text-white'
          }`}
        >
          Errors ({detail?.errors.length ?? 0})
        </button>
      </div>

      {/* Error state */}
      {error && (
        <div className="border-b border-rose-500/20 bg-rose-500/10 px-4 py-2 text-xs text-rose-300">
          {error}
        </div>
      )}

      {/* Main Content Area */}
      <div className="min-h-0 flex-1 p-3">
        {loading && !detail ? (
          <div className="flex h-full items-center justify-center text-xs text-white/40">
            <RefreshCw size={16} className="mr-2 animate-spin" />
            Loading execution waterfall…
          </div>
        ) : !detail ? (
          <div className="flex h-full items-center justify-center text-xs text-white/40">
            No waterfall data found for run {runId}.
          </div>
        ) : (
          <div className="grid h-full min-h-0 grid-cols-1 gap-3 lg:grid-cols-3">
            {/* Left 2 Cols: Main Active View */}
            <div className={`h-full min-h-0 ${selectedSpan ? 'lg:col-span-2' : 'lg:col-span-3'}`}>
              {activeTab === 'timeline' && (
                <RunWaterfallTimeline
                  spans={detail.spans}
                  onSelectSpan={setSelectedSpan}
                  selectedSpanId={selectedSpan?.id}
                />
              )}
              {activeTab === 'findings' && (
                <div className="h-full overflow-y-auto pr-1">
                  <FindingsFeed findings={detail.findings} workbenchId={detail.run.workbench_id} />
                </div>
              )}
              {activeTab === 'logs' && <StructuredLogStream logs={detail.logs} />}
              {activeTab === 'errors' && (
                <div className="h-full overflow-y-auto pr-1">
                  <ErrorGroupsTable groups={detail.error_groups} />
                </div>
              )}
            </div>

            {/* Right Col: Selected Step Span Inspector (when open) */}
            {selectedSpan && (
              <div className="flex h-full min-h-0 flex-col rounded-lg border border-white/10 bg-[#141518] p-3 text-xs space-y-3 overflow-y-auto">
                <div className="flex items-center justify-between border-b border-white/8 pb-2">
                  <span className="font-semibold text-white">Step Inspector</span>
                  <button
                    type="button"
                    onClick={() => setSelectedSpan(null)}
                    aria-label="Close step details"
                    className="rounded p-1 text-white/50 hover:bg-white/10 hover:text-white"
                  >
                    <X size={14} />
                  </button>
                </div>

                <div className="space-y-1.5 font-mono text-[11px]">
                  <div className="flex justify-between text-white/60">
                    <span>Span ID:</span>
                    <span className="text-white">{selectedSpan.id}</span>
                  </div>
                  <div className="flex justify-between text-white/60">
                    <span>Kind:</span>
                    <span className="uppercase text-sky-400">{selectedSpan.kind}</span>
                  </div>
                  <div className="flex justify-between text-white/60">
                    <span>Status:</span>
                    <span
                      className={
                        selectedSpan.status === 'error' ? 'text-rose-400' : 'text-emerald-400'
                      }
                    >
                      {selectedSpan.status}
                    </span>
                  </div>
                  <div className="flex justify-between text-white/60">
                    <span>Duration:</span>
                    <span className="text-white">{selectedSpan.duration_ms ?? 0}ms</span>
                  </div>
                </div>

                {/* Metadata / Input / Output payload */}
                {selectedSpan.metadata && Object.keys(selectedSpan.metadata).length > 0 && (
                  <div className="space-y-1">
                    <span className="font-mono text-[10px] uppercase font-semibold text-white/50">
                      Step Payload
                    </span>
                    <pre className="max-h-60 overflow-y-auto rounded bg-[#0d0e11] p-2 font-mono text-[10px] leading-relaxed text-white/80 border border-white/8">
                      {JSON.stringify(selectedSpan.metadata, null, 2)}
                    </pre>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
