import React, { useState, useEffect } from 'react';
import {
  Sparkles,
  Terminal,
  FileCode,
  Globe,
  Bot,
  Square,
  Compass,
  Loader2,
} from 'lucide-react';

export interface AgentWorkingStatusProps {
  status?: string;
  runId?: string;
  harnessName?: string;
  currentActivity?: {
    kind?: string;
    payload?: Record<string, any>;
  };
  onStop?: () => void;
  onFocusSteer?: () => void;
  className?: string;
}

export const AgentWorkingStatus: React.FC<AgentWorkingStatusProps> = ({
  status = 'working',
  runId: _runId,
  harnessName = 'Coding Agent',
  currentActivity,
  onStop,
  onFocusSteer,
  className = '',
}) => {
  const [elapsedSeconds, setElapsedSeconds] = useState(0);

  useEffect(() => {
    setElapsedSeconds(0);
    const interval = setInterval(() => {
      setElapsedSeconds((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const formatDuration = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    const rem = seconds % 60;
    return `${mins}m ${rem}s`;
  };

  // Derive human-readable current activity label
  let activityLabel = 'Deliberating next step...';
  let ActivityIcon = Sparkles;
  let iconColor = 'text-amber-400';

  if (status === 'thinking') {
    activityLabel = 'Thinking through architecture & strategy...';
    ActivityIcon = Sparkles;
    iconColor = 'text-amber-400';
  } else if (currentActivity) {
    const kind = currentActivity.kind || '';
    const payload = currentActivity.payload || {};
    const tool = String(payload.name || payload.tool || payload.command || '').toLowerCase();

    if (tool.includes('bash') || tool.includes('exec') || tool === 'command_execution') {
      const cmd = String(payload.command || payload.cmd || '');
      activityLabel = cmd ? `Running: ${cmd.slice(0, 45)}...` : 'Running terminal command...';
      ActivityIcon = Terminal;
      iconColor = 'text-emerald-400';
    } else if (tool.includes('edit') || tool.includes('write') || tool.includes('file')) {
      const file = String(payload.path || payload.filePath || payload.file || '');
      const base = file.split(/[\\/]/).pop() || file;
      activityLabel = base ? `Editing: ${base}` : 'Applying file modifications...';
      ActivityIcon = FileCode;
      iconColor = 'text-blue-400';
    } else if (tool.includes('search') || tool.includes('pubmed') || tool.includes('arxiv')) {
      activityLabel = 'Searching research literature & evidence...';
      ActivityIcon = Globe;
      iconColor = 'text-sky-400';
    } else if (kind.includes('subagent') || tool.includes('delegate')) {
      activityLabel = 'Coordinating subagent execution...';
      ActivityIcon = Bot;
      iconColor = 'text-purple-400';
    }
  }

  return (
    <div
      className={`flex items-center justify-between px-3 py-2 rounded-lg bg-surface-1/90 border border-border-default/80 backdrop-blur-xs text-xs shadow-xs transition ${className}`}
    >
      <div className="flex items-center gap-2.5 min-w-0">
        <div className="relative flex items-center justify-center shrink-0">
          <Loader2 className="w-4 h-4 text-primary animate-spin" />
          <span className="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping" />
        </div>
        <div className="min-w-0 flex items-center gap-2">
          <span className="font-semibold text-fg-editor truncate">{harnessName}</span>
          <span className="text-border-default font-mono">/</span>
          <div className="flex items-center gap-1.5 text-fg-muted min-w-0 truncate">
            <ActivityIcon className={`w-3.5 h-3.5 ${iconColor} shrink-0`} />
            <span className="truncate">{activityLabel}</span>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0 ml-3">
        <span className="font-mono text-[11px] text-fg-subtle tabular-nums px-1.5 py-0.5 rounded bg-surface-2">
          {formatDuration(elapsedSeconds)}
        </span>

        {onFocusSteer && (
          <button
            type="button"
            onClick={onFocusSteer}
            className="flex items-center gap-1 px-2 py-1 rounded bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 font-medium text-[11px] border border-amber-500/20 transition cursor-pointer"
            title="Type guidance below to steer the agent mid-flight"
          >
            <Compass className="w-3 h-3" />
            <span className="hidden sm:inline">Steer</span>
          </button>
        )}

        {onStop && (
          <button
            type="button"
            onClick={onStop}
            className="flex items-center gap-1 px-2 py-1 rounded bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 font-medium text-[11px] border border-rose-500/20 transition cursor-pointer"
            title="Cancel / Stop current agent execution"
          >
            <Square className="w-3 h-3 fill-current" />
            <span className="hidden sm:inline">Stop</span>
          </button>
        )}
      </div>
    </div>
  );
};
