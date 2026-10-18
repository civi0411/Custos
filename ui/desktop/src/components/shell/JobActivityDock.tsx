import React, { useState, useEffect } from 'react';
import {
  Minimize2,
  Loader2,
  CheckCircle2,
  AlertTriangle,
  Bot,
  Trash2,
  Sparkles,
  PlayCircle,
} from 'lucide-react';
import { useJobRegistry, JobEntry } from '@/hooks/useJobRegistry';

interface JobActivityDockProps {
  onRestoreSplit: () => void;
  onOpenKanban?: () => void;
  lensBadge?: string;
  lensColor?: string;
}

function formatElapsed(startedAt: number, endedAt?: number): string {
  const end = endedAt || Date.now();
  const seconds = Math.max(0, Math.floor((end - startedAt) / 1000));
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const mins = Math.floor(seconds / 60);
  const remainingSecs = seconds % 60;
  return `${mins}m ${remainingSecs}s`;
}

export const JobActivityDock: React.FC<JobActivityDockProps> = ({
  onRestoreSplit,
  onOpenKanban,
  lensBadge = 'Engineering',
  lensColor = 'bg-emerald-500',
}) => {
  const { jobs, activeCount, clearCompleted } = useJobRegistry();
  const [, setTick] = useState(0);
  const [isDismissed, setIsDismissed] = useState(false);

  // Update timer tick every second for running jobs
  useEffect(() => {
    if (activeCount === 0) return;
    setIsDismissed(false); // Re-open when a new job starts running
    const interval = setInterval(() => {
      setTick((t) => t + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [activeCount]);

  const hasCompletedJobs = jobs.some((j) => j.status === 'done' || j.status === 'started' || j.status === 'failed');

  // Do NOT render when dismissed or when there are no active/recent jobs
  if (isDismissed || (activeCount === 0 && !hasCompletedJobs)) {
    return null;
  }

  return (
    <div className="absolute z-50 bottom-5 right-5 w-[360px] pointer-events-auto animate-in slide-in-from-bottom-2 duration-150">
      <div className="bg-surface-1/95 backdrop-blur-xl border border-border-default rounded-xl p-3 shadow-[0_12px_45px_-10px_rgba(0,0,0,0.85)] focus-within:border-border-emphasis relative">
        {/* Header */}
        <div className="flex items-center justify-between pb-2 mb-2 border-b border-border-muted text-[11px] text-fg-muted">
          <div className="flex items-center gap-1.5">
            <span className={`w-2 h-2 rounded-full ${lensColor}`} />
            <span className="font-semibold text-fg-editor tracking-wide">
              {lensBadge} Activity Monitor
            </span>
            {activeCount > 0 && (
              <span className="font-mono text-[10px] bg-emerald-500/15 text-emerald-400 px-1.5 py-0.2 rounded border border-emerald-500/30">
                {activeCount} running
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            <button
              onClick={onRestoreSplit}
              className="hover:text-fg-editor flex items-center gap-1 text-[10px] bg-surface-2 hover:bg-surface-3 px-2 py-0.5 rounded transition text-fg-muted border border-border-subtle cursor-pointer"
              title="Restore split layout to access chat"
            >
              <Minimize2 size={11} /> Restore Split
            </button>
            <button
              onClick={() => setIsDismissed(true)}
              className="hover:text-fg-editor flex items-center justify-center size-5 rounded hover:bg-surface-2 transition text-fg-subtle cursor-pointer"
              title="Dismiss activity monitor"
              aria-label="Dismiss monitor"
            >
              <Trash2 size={11} className="hidden" />
              <span className="text-xs leading-none">×</span>
            </button>
          </div>
        </div>

        {/* Body */}
        {jobs.length > 0 ? (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-[11px] px-0.5">
              <span className="font-mono text-fg-subtle uppercase text-[10px] tracking-wider">
                {activeCount > 0 ? `Active Jobs (${activeCount})` : 'Recent Jobs'}
              </span>
              {hasCompletedJobs && (
                <button
                  onClick={clearCompleted}
                  className="flex items-center gap-1 text-[10px] text-fg-muted hover:text-fg-editor transition"
                  title="Clear finished jobs"
                >
                  <Trash2 size={10} /> Clear finished
                </button>
              )}
            </div>

            {/* Job List */}
            <div className="max-h-[220px] overflow-y-auto space-y-1.5 pr-0.5">
              {jobs.map((job: JobEntry) => {
                const isRunning = job.status === 'running' || job.status === 'queued';
                const isStarted = job.status === 'started';
                const isDone = job.status === 'done';
                const isFailed = job.status === 'failed';

                return (
                  <div
                    key={job.id}
                    className="flex items-center justify-between p-2 rounded-xl bg-surface-2/60 border border-border-subtle hover:border-border-default transition text-xs"
                  >
                    <div className="flex items-center gap-2 min-w-0 pr-2">
                      {isRunning && (
                        <Loader2 className="w-3.5 h-3.5 animate-spin text-emerald-400 shrink-0" />
                      )}
                      {isStarted && (
                        <PlayCircle className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                      )}
                      {isDone && (
                        <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                      )}
                      {isFailed && (
                        <AlertTriangle className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                      )}

                      <div className="min-w-0">
                        <div className="font-medium text-fg-editor truncate text-[12px]">
                          {job.jobName}
                        </div>
                        <div className="text-[10px] text-fg-subtle truncate flex items-center gap-1.5 font-mono">
                          {job.sourceTabId && (
                            <span className="capitalize">{job.sourceTabId}</span>
                          )}
                          {job.taskId && <span>· Task {job.taskId.slice(0, 8)}</span>}
                          {job.error && (
                            <span className="text-rose-400 truncate">· {job.error}</span>
                          )}
                        </div>
                      </div>
                    </div>

                    <div className="shrink-0 text-right">
                      <span className="font-mono text-[10px] text-fg-subtle">
                        {formatElapsed(job.startedAt, job.endedAt)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Footer Quick Action */}
            <div className="pt-2 border-t border-border-muted flex items-center justify-between gap-2">
              <span className="text-[10px] text-fg-subtle">
                Local launch activity · inspect runtime status in Supervision
              </span>
              {onOpenKanban && (
                <button
                  onClick={onOpenKanban}
                  className="flex items-center gap-1 px-2.5 py-1 rounded-lg bg-surface-2 hover:bg-surface-3 text-fg-editor hover:text-emerald-400 border border-border-subtle transition text-xs font-medium"
                >
                  <Bot size={12} className="text-emerald-400" />
                  <span>View in Kanban</span>
                </button>
              )}
            </div>
          </div>
        ) : (
          <div className="py-4 px-2 text-center space-y-2">
            <div className="w-8 h-8 rounded-full bg-surface-2 flex items-center justify-center mx-auto text-fg-subtle">
              <Sparkles size={16} className="text-emerald-400" />
            </div>
            <div>
              <p className="text-xs font-medium text-fg-editor">No active jobs</p>
              <p className="text-[11px] text-fg-subtle mt-0.5 max-w-[280px] mx-auto leading-relaxed">
                Use toolbar actions in the active pane to trigger contextual SADE workbench jobs.
              </p>
            </div>
            <div className="pt-2 border-t border-border-muted text-[10px] text-fg-subtle flex items-center justify-center gap-1">
              <span>Restore split (<Minimize2 size={9} className="inline" />) to access chat or slash commands</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default JobActivityDock;
