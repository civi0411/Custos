import React from 'react';
import {
  ExternalLink,
  CheckCircle2,
  XCircle,
  Unlink,
  MessageSquare,
  ArrowUpRight,
} from 'lucide-react';
import type { Session } from '@/types';
import type { Task, ExecutionWorkspace } from '@/types/domain';

export interface TaskAnchorBannerProps {
  session: Session;
  task?: Task | null;
  boundWorkspace?: ExecutionWorkspace | null;
  attentionCount?: number;
  hasMessages: boolean;
  onViewTask: () => void;
  onOpenSplitCanvas?: () => void;
  onPromoteToTask: () => void;
  onDetachTask?: () => void;
}

export const TaskAnchorBanner: React.FC<TaskAnchorBannerProps> = ({
  session,
  task,
  boundWorkspace: _boundWorkspace,
  attentionCount = 0,
  hasMessages,
  onViewTask,
  onOpenSplitCanvas: _onOpenSplitCanvas,
  onPromoteToTask,
  onDetachTask,
}) => {
  const isAttached = Boolean(session.taskId);

  if (isAttached) {
    const rawStatus = (task?.status || session.taskStatus || 'running').toLowerCase();
    const isNeedsAttention = attentionCount > 0 || rawStatus === 'blocked';
    const isDone = rawStatus === 'done' || rawStatus === 'succeeded' || rawStatus === 'completed';
    const isFailed = rawStatus === 'failed';

    return (
      <div
        className="flex h-10 items-center justify-between px-3.5 border-b border-white/10 bg-[#161619] text-xs shrink-0 select-none z-10 font-sans"
        role="region"
        aria-label="Bound task anchor banner"
      >
        {/* Left: Status badge & Task title */}
        <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
          {/* Status Badge */}
          {isNeedsAttention ? (
            <span className="flex items-center gap-1.5 px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-white/10 text-white border border-white/20 shrink-0">
              <span className="size-1.5 rounded-full bg-white/80 animate-pulse" />
              <span>NEEDS REVIEW{attentionCount > 0 ? ` (${attentionCount})` : ''}</span>
            </span>
          ) : isDone ? (
            <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-white/6 text-white/70 border border-white/10 shrink-0">
              <CheckCircle2 className="size-3 text-white/60" />
              <span>COMPLETED</span>
            </span>
          ) : isFailed ? (
            <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-rose-500/15 text-rose-300 border border-rose-500/30 shrink-0">
              <XCircle className="size-3 text-rose-400" />
              <span>FAILED</span>
            </span>
          ) : (
            <span className="flex items-center gap-1.5 px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-white/6 text-white/70 border border-white/10 shrink-0">
              <span className="size-1.5 rounded-full bg-white/70 animate-pulse" />
              <span>RUNNING</span>
            </span>
          )}

          {/* Bound Task Title */}
          <div className="flex items-center gap-1.5 truncate min-w-0">
            <span className="text-white/40 text-[11px] shrink-0">Task:</span>
            <span
              className="font-medium text-white/90 truncate text-[11.5px] hover:text-white transition-colors cursor-pointer"
              onClick={onViewTask}
              title={task?.title || session.taskId || ''}
            >
              {task?.title || (session.taskId && session.taskId.length > 24 ? `${session.taskId.slice(0, 20)}…` : session.taskId)}
            </span>
          </div>
        </div>

        {/* Right: Quick Action Controls */}
        <div className="flex items-center gap-1.5 shrink-0">

          <button
            type="button"
            onClick={onViewTask}
            className="flex items-center gap-1 px-2 py-0.5 rounded text-[11px] text-white/80 hover:text-white hover:bg-white/10 transition cursor-pointer font-medium"
            title="Open task details in sidebar"
          >
            <span>Details</span>
            <ExternalLink className="size-3" />
          </button>

          {onDetachTask && (
            <button
              type="button"
              onClick={onDetachTask}
              className="p-1 rounded text-white/30 hover:text-white/80 hover:bg-white/10 transition cursor-pointer ml-0.5"
              title="Detach session from task (revert to General Chat)"
            >
              <Unlink className="size-3" />
            </button>
          )}
        </div>
      </div>
    );
  }

  // Bare Session (General Chat)
  return (
    <div
      className="flex h-9 items-center justify-between px-3.5 border-b border-white/10 bg-[#141416] text-xs shrink-0 select-none font-sans"
      role="region"
      aria-label="General conversation status banner"
    >
      <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-medium bg-white/6 text-white/60 border border-white/10 shrink-0">
          <MessageSquare className="size-3 text-white/50" />
          <span>CHAT</span>
        </span>
        <span className="text-[11px] text-white/40 truncate hidden sm:inline">
          General conversation (read-only safe mode; source files preserved)
        </span>
      </div>

      <div className="flex items-center gap-1.5 shrink-0">
        {hasMessages && (
          <button
            type="button"
            onClick={onPromoteToTask}
            className="flex items-center gap-1.5 rounded-md px-2.5 py-0.5 text-[11px] font-medium bg-white/8 text-white hover:bg-white/15 border border-white/15 transition shadow-2xs cursor-pointer"
            title="Promote this session to a formal Task with acceptance criteria"
          >
            <ArrowUpRight className="size-3 text-white/60" />
            <span>Promote to Task</span>
          </button>
        )}
      </div>
    </div>
  );
};
