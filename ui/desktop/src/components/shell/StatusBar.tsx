import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Scaling, GitBranch, AlertTriangle } from 'lucide-react';
import { Tooltip } from '../common/Tooltip';
import { formatKeyCombo } from '@/lib/utils';
import { daemonClient } from '@/api/daemon_client';

export interface StatusBarProps {
  activeSessionTitle: string;
  activeSessionId?: string;
  activeModel: string;
  activeWorkbench?: 'engineering' | 'research' | 'copilot';
  activeBranch?: string | null;
  isDirty?: boolean;
  uiScale: number;
  onSetUiScale: (val: number) => void;
  onStepUiScale: (delta: number) => void;
}

interface SessionMetrics {
  session_id?: string;
  tokens_in: number;
  tokens_out: number;
  total_tokens: number;
  estimated_cost_usd: number;
  active_runs: number;
  pending_approvals: number;
}

function formatTokens(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return count.toLocaleString();
}

export const StatusBar: React.FC<StatusBarProps> = ({
  activeSessionTitle,
  activeSessionId,
  activeModel,
  activeWorkbench = 'engineering',
  activeBranch,
  isDirty,
  uiScale,
  onSetUiScale,
  onStepUiScale
}) => {
  const [isScaleMenuOpen, setIsScaleMenuOpen] = useState(false);
  const scaleRef = useRef<HTMLDivElement>(null);

  // Live metrics & daemon status
  const [daemonOnline, setDaemonOnline] = useState<boolean | null>(null);
  const [metrics, setMetrics] = useState<SessionMetrics | null>(null);

  const fetchLiveMetrics = useCallback(async () => {
    try {
      const isOnline = await daemonClient.checkHealth();
      setDaemonOnline(isOnline);
      if (isOnline) {
        const data = await daemonClient.getSessionMetrics(activeSessionId);
        setMetrics(data);
      } else {
        setMetrics(null);
      }
    } catch {
      setDaemonOnline(false);
      setMetrics(null);
    }
  }, [activeSessionId]);

  useEffect(() => {
    fetchLiveMetrics();
    // Poll every 5s if active runs > 0, otherwise 15s
    const pollInterval = metrics && metrics.active_runs > 0 ? 5000 : 15000;
    const interval = setInterval(fetchLiveMetrics, pollInterval);
    return () => clearInterval(interval);
  }, [fetchLiveMetrics, metrics?.active_runs]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (scaleRef.current && !scaleRef.current.contains(e.target as Node)) {
        setIsScaleMenuOpen(false);
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, []);

  const presets = [70, 80, 90, 100, 110, 120, 130, 140, 150, 160];

  const modeBadge = {
    engineering: {
      label: '◈ Engineering',
      className: 'text-fg-editor bg-surface-2 border-border-default'
    },
    research: {
      label: '⬡ Research',
      className: 'text-fg-editor bg-surface-2 border-border-default'
    },
    copilot: {
      label: '◇ Copilot',
      className: 'text-fg-editor bg-surface-2 border-border-default'
    }
  }[activeWorkbench] || {
    label: '◈ Engineering',
    className: 'text-fg-editor bg-surface-2 border-border-default'
  };

  return (
    <footer className="h-6 bg-surface-1 border-t border-border-default px-3 flex items-center justify-between text-[11px] font-mono text-fg-muted shrink-0 z-30 select-none">
      {/* Left: Connection, Session Title, Mode Badge, Branch */}
      <div className="flex items-center gap-2.5 truncate">
        {/* Daemon Connection Indicator */}
        <Tooltip
          content={
            daemonOnline === true
              ? 'Daemon Online (127.0.0.1:3000)'
              : daemonOnline === false
                ? 'Daemon Offline — Attempting reconnect'
                : 'Checking daemon connection...'
          }
          position="top"
        >
          <div className="flex items-center gap-1.5 cursor-pointer">
            <span
              className={`w-2 h-2 rounded-full transition-colors ${daemonOnline === true
                  ? 'bg-emerald-500'
                  : daemonOnline === false
                    ? 'bg-rose-500'
                    : 'bg-amber-400 animate-pulse'
                }`}
            />
          </div>
        </Tooltip>

        {/* Active Session Title */}
        <span className="text-fg-editor font-medium truncate max-w-[140px] sm:max-w-[200px]" title={activeSessionTitle}>
          {activeSessionTitle}
        </span>

        <span className="text-border-default">|</span>

        {/* Mode Pill Badge */}
        <span
          className={`inline-flex items-center px-1.5 py-0.2 rounded text-[10px] font-semibold border ${modeBadge.className}`}
        >
          {modeBadge.label}
        </span>

        {/* Git Branch / Worktree Info */}
        {activeBranch && (
          <>
            <span className="text-border-default hidden md:inline">|</span>
            <div className="hidden md:flex items-center gap-1 text-fg-editor">
              <GitBranch className="w-3 h-3 text-fg-subtle" />
              <span className="font-mono text-[10px] truncate max-w-[120px]">{activeBranch}</span>
              {isDirty && (
                <span
                  className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0"
                  title="Uncommitted changes in worktree"
                />
              )}
            </div>
          </>
        )}
      </div>

      {/* Right: Live Metrics (Tokens, Cost, Runs, Approvals) + Scale */}
      <div className="flex items-center gap-2.5 sm:gap-3 shrink-0">
        {/* Live Token Usage */}
        <Tooltip
          content={
            metrics
              ? `Input: ${metrics.tokens_in.toLocaleString()} | Output: ${metrics.tokens_out.toLocaleString()}`
              : 'Daemon offline: Token metrics unavailable'
          }
          position="top"
        >
          <span className="flex items-center gap-1">
            <span className="text-fg-subtle">Tokens:</span>
            <span className="text-fg-editor font-semibold">
              {metrics ? `${formatTokens(metrics.total_tokens)}` : '—'}
            </span>
          </span>
        </Tooltip>

        <span className="text-border-default">|</span>

        {/* Estimated Cost */}
        <Tooltip
          content={
            metrics
              ? `Estimated Session API Cost ($3/M in, $15/M out)`
              : 'Daemon offline: Cost metrics unavailable'
          }
          position="top"
        >
          <span className="flex items-center gap-1">
            <span className="text-fg-subtle">Cost:</span>
            <span className="text-fg-editor font-semibold">
              {metrics ? `$${metrics.estimated_cost_usd.toFixed(2)}` : '—'}
            </span>
          </span>
        </Tooltip>

        <span className="text-border-default">|</span>

        {/* Active Runs / Status */}
        <Tooltip
          content={
            metrics
              ? `${metrics.active_runs} active execution run(s)`
              : 'Daemon offline: Execution status unavailable'
          }
          position="top"
        >
          <span className="flex items-center gap-1">
            {metrics ? (
              metrics.active_runs > 0 ? (
                <span className="flex items-center gap-1 text-emerald-400 font-semibold">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                  {metrics.active_runs} run{metrics.active_runs > 1 ? 's' : ''}
                </span>
              ) : (
                <span className="text-fg-subtle">idle</span>
              )
            ) : (
              <span className="text-fg-subtle">—</span>
            )}
          </span>
        </Tooltip>

        {/* Pending Approvals Badge (if any) */}
        {metrics && metrics.pending_approvals > 0 && (
          <>
            <span className="text-border-default">|</span>
            <span className="flex items-center gap-1 text-amber-400 font-semibold bg-amber-500/10 px-1 rounded border border-amber-500/20">
              <AlertTriangle className="w-3 h-3" />
              <span>{metrics.pending_approvals}</span>
            </span>
          </>
        )}

        <span className="text-border-default hidden lg:inline">|</span>

        {/* Active Model */}
        <span className="hidden lg:inline text-fg-subtle truncate max-w-[130px]" title={activeModel}>
          {activeModel || 'No model'}
        </span>

        <span className="text-border-default">|</span>

        {/* UI Scale Popover Trigger */}
        <div className="relative inline-flex items-center" ref={scaleRef}>
          <Tooltip
            content={`Scale: Fonts & Components (${formatKeyCombo({ ctrlOrCmd: true, key: '+' })} / ${formatKeyCombo({ ctrlOrCmd: true, key: '-' })})`}
            position="top"
            align="end"
          >
            <button
              onClick={(e) => {
                e.stopPropagation();
                setIsScaleMenuOpen(!isScaleMenuOpen);
              }}
              className={`flex items-center gap-1.5 px-2 py-0.5 rounded font-mono transition text-[11px] border cursor-pointer ${isScaleMenuOpen
                  ? 'bg-surface-2 border-border-default text-fg-editor shadow-xs'
                  : 'hover:bg-surface-2 text-fg-muted hover:text-fg-editor border-transparent hover:border-border-default'
                }`}
            >
              <Scaling className={`w-3 h-3 transition shrink-0 ${isScaleMenuOpen ? 'workbench-accent' : 'text-fg-subtle'}`} />
              <span>{uiScale}%</span>
            </button>
          </Tooltip>

          {isScaleMenuOpen && (
            <div className="absolute right-0 bottom-full mb-2 w-72 bg-surface-1 border border-border-default rounded-xl shadow-2xl p-1.5 text-xs z-50 select-none animate-in fade-in zoom-in-95 duration-100">
              {/* Header */}
              <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle flex items-center justify-between border-b border-border-muted pb-1.5 mb-1.5">
                <div className="flex items-center gap-1.5">
                  <Scaling className="w-3.5 h-3.5 workbench-accent" />
                  <span className="font-semibold text-fg-editor">Scale UI & Components</span>
                </div>
                <Tooltip content={`Reset to 100% (${formatKeyCombo({ ctrlOrCmd: true, key: '0' })})`} position="top">
                  <button
                    onClick={() => onSetUiScale(100)}
                    className="text-fg-muted hover:text-fg-editor font-mono bg-surface-2 hover:bg-surface-3 px-1.5 py-0.5 rounded text-[10px] border border-border-default transition cursor-pointer"
                  >
                    Reset
                  </button>
                </Tooltip>
              </div>

              {/* Stepper & Slider */}
              <div className="bg-surface-0 border border-border-default rounded-lg p-2.5 mb-1.5 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-fg-muted">Current Scale:</span>
                  <span className="font-mono text-xs font-bold text-fg-editor bg-surface-1 px-2 py-0.5 rounded border border-border-default">
                    {uiScale}%
                  </span>
                </div>

                <div className="flex items-center gap-2 pt-0.5">
                  <Tooltip content="Zoom Out (Ctrl+-)" position="top">
                    <button
                      onClick={() => onStepUiScale(-10)}
                      className="w-6 h-6 rounded-md bg-surface-1 hover:bg-surface-2 text-fg-editor border border-border-default flex items-center justify-center font-mono text-xs transition cursor-pointer"
                    >
                      -
                    </button>
                  </Tooltip>
                  <input
                    type="range"
                    aria-label="App zoom"
                    style={{ '--range-progress': `${((uiScale - 70) / 90) * 100}%` } as React.CSSProperties}
                    min="70"
                    max="160"
                    step="10"
                    value={uiScale}
                    onChange={(e) => onSetUiScale(Number(e.target.value))}
                    className="flex-1 min-w-0"
                  />
                  <Tooltip content="Zoom In (Ctrl++)" position="top">
                    <button
                      onClick={() => onStepUiScale(10)}
                      className="w-6 h-6 rounded-md bg-surface-1 hover:bg-surface-2 text-fg-editor border border-border-default flex items-center justify-center font-mono text-xs transition cursor-pointer"
                    >
                      +
                    </button>
                  </Tooltip>
                </div>
              </div>

              {/* Presets Grid */}
              <div className="space-y-1 mb-1.5 px-1">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">Presets</div>
                <div className="grid grid-cols-5 gap-1 font-mono text-[11px]">
                  {presets.map((s) => (
                    <button
                      key={s}
                      onClick={() => onSetUiScale(s)}
                      className={`px-1.5 py-1 rounded-lg border transition text-center cursor-pointer ${uiScale === s
                          ? 'bg-surface-2 border-border-emphasis text-fg-editor font-semibold shadow-xs'
                          : 'bg-transparent hover:bg-surface-2 border-border-muted text-fg-muted hover:text-fg-editor'
                        }`}
                    >
                      {s}%
                    </button>
                  ))}
                </div>
              </div>

              {/* Shortcuts Hint */}
              <div className="px-2 pt-1.5 border-t border-border-muted flex items-center justify-between text-[10px] text-fg-subtle">
                <span className="flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                  <span>Fonts & Components</span>
                </span>
                <span className="font-mono">
                  {formatKeyCombo({ ctrlOrCmd: true, key: '+' })} / {formatKeyCombo({ ctrlOrCmd: true, key: '-' })}
                </span>
              </div>
            </div>
          )}
        </div>

        <span className="text-border-default">|</span>
        <span className="text-fg-subtle">UTF-8</span>
      </div>
    </footer>
  );
};
