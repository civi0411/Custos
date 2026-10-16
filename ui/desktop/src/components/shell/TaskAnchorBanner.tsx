import React from 'react';
import {
  ExternalLink,
  GitBranch,
  Columns2,
  Sparkles,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Unlink,
  MessageSquare,
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
  boundWorkspace,
  attentionCount = 0,
  hasMessages,
  onViewTask,
  onOpenSplitCanvas,
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
        className="flex h-10 items-center justify-between px-3.5 border-b border-white/10 bg-[#161619]/90 text-xs shrink-0 select-none backdrop-blur-xs transition-colors z-10"
        role="region"
        aria-label="Khung neo nhiệm vụ liên kết"
      >
        {/* Left: Status badge & Task title */}
        <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
          {/* Status Badge */}
          {isNeedsAttention ? (
            <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-amber-500/20 text-amber-300 border border-amber-500/40 animate-pulse shrink-0">
              <AlertTriangle className="size-3" />
              <span>CẦN DUYỆT{attentionCount > 0 ? ` (${attentionCount})` : ''}</span>
            </span>
          ) : isDone ? (
            <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 shrink-0">
              <CheckCircle2 className="size-3" />
              <span>HOÀN THÀNH</span>
            </span>
          ) : isFailed ? (
            <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-rose-500/20 text-rose-300 border border-rose-500/40 shrink-0">
              <XCircle className="size-3" />
              <span>THẤT BẠI</span>
            </span>
          ) : (
            <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-blue-500/20 text-blue-300 border border-blue-500/40 shrink-0">
              <span className="size-1.5 rounded-full bg-blue-400 animate-pulse" />
              <span>ĐANG CHẠY</span>
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

          {/* Git Worktree / Workspace Badge */}
          {boundWorkspace && (
            <div
              className="hidden md:flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/5 border border-white/10 text-[10px] font-mono text-white/60 shrink-0"
              title={`Không gian làm việc cách ly: ${boundWorkspace.path || boundWorkspace.name}`}
            >
              <GitBranch className="size-2.5 text-workbench-accent" />
              <span className="truncate max-w-[110px]">{boundWorkspace.name}</span>
            </div>
          )}
        </div>

        {/* Right: Quick Action Controls */}
        <div className="flex items-center gap-1.5 shrink-0">
          {onOpenSplitCanvas && (
            <button
              type="button"
              onClick={onOpenSplitCanvas}
              className="flex items-center gap-1 px-2 py-0.5 rounded text-[11px] text-white/60 hover:text-white hover:bg-white/10 transition cursor-pointer border border-transparent hover:border-white/10"
              title="Mở Canvas song song để xem tài nguyên & diff"
            >
              <Columns2 className="size-3" />
              <span className="hidden sm:inline">Split Canvas</span>
            </button>
          )}

          <button
            type="button"
            onClick={onViewTask}
            className="flex items-center gap-1 px-2 py-0.5 rounded text-[11px] text-workbench-accent hover:text-workbench-accent/90 hover:bg-workbench-accent/10 transition cursor-pointer font-medium"
            title="Mở chi tiết Task trên thanh bên"
          >
            <span>Chi tiết</span>
            <ExternalLink className="size-3" />
          </button>

          {onDetachTask && (
            <button
              type="button"
              onClick={onDetachTask}
              className="p-1 rounded text-white/30 hover:text-white/80 hover:bg-white/10 transition cursor-pointer ml-0.5"
              title="Tách phiên trò chuyện khỏi Task (Về Chat thường)"
            >
              <Unlink className="size-3" />
            </button>
          )}
        </div>
      </div>
    );
  }

  // Bare Session (Chat Thường)
  return (
    <div
      className="flex h-9 items-center justify-between px-3.5 border-b border-white/10 bg-[#141416]/80 text-xs shrink-0 select-none backdrop-blur-xs"
      role="region"
      aria-label="Khung trạng thái hội thoại độc lập"
    >
      <div className="flex items-center gap-2 min-w-0 flex-1 mr-2">
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-white/6 text-white/60 border border-white/10 shrink-0">
          <MessageSquare className="size-3 text-white/50" />
          <span>CHAT THƯỜNG</span>
        </span>
        <span className="text-[11px] text-white/40 truncate hidden sm:inline">
          Hội thoại độc lập (Chế độ an toàn, không tự ý sửa đổi file mã nguồn)
        </span>
      </div>

      <div className="flex items-center gap-1.5 shrink-0">
        {hasMessages && (
          <button
            type="button"
            onClick={onPromoteToTask}
            className="flex items-center gap-1.5 rounded-md px-2.5 py-0.5 text-[11px] font-medium bg-workbench-accent/20 text-workbench-accent border border-workbench-accent/30 hover:bg-workbench-accent hover:text-white transition shadow-2xs cursor-pointer"
            title="Thăng cấp cuộc hội thoại này thành Nhiệm vụ chính thức với tiêu chí nghiệm thu"
          >
            <Sparkles className="size-3" />
            <span>Thăng cấp thành Task</span>
          </button>
        )}
      </div>
    </div>
  );
};
