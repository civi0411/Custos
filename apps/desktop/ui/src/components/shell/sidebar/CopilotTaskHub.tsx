import React from 'react';
import { Bot, Plus, CheckSquare } from 'lucide-react';
import type { Task } from '@/types/domain';

export interface CopilotGroupedTasks {
  needsApproval: Task[];
  inProgress: Task[];
  standbyCompleted: Task[];
}

export interface CopilotTaskHubProps {
  tasks: Task[];
  selectedTaskId?: string | null;
  onSelectTask?: (id: string) => void;
  onRequestCreateTask?: () => void;
  filterActiveOnly: boolean;
  formatRelativeTime: (iso?: string) => string;
}

export const CopilotTaskHub: React.FC<CopilotTaskHubProps> = ({
  tasks,
  selectedTaskId,
  onSelectTask,
  onRequestCreateTask,
  filterActiveOnly,
  formatRelativeTime,
}) => {
  const groupedTasks: CopilotGroupedTasks = React.useMemo(() => {
    const needsApproval = tasks.filter(
      (t) =>
        t.status === 'Blocked' ||
        String(t.status).toLowerCase() === 'blocked' ||
        t.metadata?.approval_required === 'true'
    );
    const inProgress = tasks.filter(
      (t) =>
        (t.status === 'running' || t.status === 'Active' || String(t.status).toLowerCase() === 'running') &&
        !needsApproval.some((na) => na.id === t.id)
    );
    const standbyCompleted = tasks.filter(
      (t) =>
        !needsApproval.some((na) => na.id === t.id) &&
        !inProgress.some((ip) => ip.id === t.id)
    );
    return { needsApproval, inProgress, standbyCompleted };
  }, [tasks]);

  if (tasks.length === 0) {
    return (
      <div className="p-4 text-center space-y-3 border border-dashed border-white/10 rounded-xl my-4">
        <Bot className="size-7 text-white/40 mx-auto" />
        <div className="space-y-1">
          <div className="text-white/80 font-medium text-xs">No active tasks or operations</div>
          <div className="text-white/40 text-[11px] leading-relaxed">
            Create an autonomous task or copilot mission to supervise.
          </div>
        </div>
        {onRequestCreateTask && (
          <button
            type="button"
            onClick={onRequestCreateTask}
            className="px-3 py-1.5 bg-white/[0.06] hover:bg-white/[0.1] text-white/90 border border-white/10 text-xs rounded-lg transition font-medium cursor-pointer flex items-center gap-1.5 mx-auto"
          >
            <Plus className="size-3.5" />
            <span>New Task</span>
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* Needs Review Section */}
      {groupedTasks.needsApproval.length > 0 && (
        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5 px-1 text-[11px] font-semibold text-amber-300">
            <span className="size-2 rounded-full bg-amber-400 animate-pulse" />
            <span>Needs Review ({groupedTasks.needsApproval.length})</span>
          </div>
          <div className="space-y-1.5">
            {groupedTasks.needsApproval.map((task) => {
              const isSelected = selectedTaskId === task.id;
              return (
                <div
                  key={task.id}
                  onClick={() => onSelectTask?.(task.id)}
                  className={`p-2.5 rounded-xl border transition cursor-pointer space-y-1.5 ${
                    isSelected
                      ? 'bg-amber-500/15 border-amber-500/50 text-white shadow-xs'
                      : 'bg-amber-500/5 hover:bg-amber-500/10 border-amber-500/25 text-white/90'
                  }`}
                >
                  <div className="flex items-center justify-between gap-1 text-[10px]">
                    <span className="font-mono px-1.5 py-0.5 rounded bg-amber-500/20 text-amber-300 font-semibold uppercase tracking-wider text-[9px]">
                      {task.contract?.pack_id || 'approval'}
                    </span>
                    <span className="text-amber-400 font-mono text-[9px]">Action Needed</span>
                  </div>
                  <div className="text-xs font-medium line-clamp-2">{task.title || task.id}</div>
                  <div className="flex items-center justify-between text-[10px] text-white/40 pt-1 border-t border-white/6 font-mono">
                    <span>{task.id.slice(0, 8)}</span>
                    <span className="text-amber-300 font-sans font-medium hover:underline">Review ➔</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* In Progress Section */}
      {groupedTasks.inProgress.length > 0 && (
        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5 px-1 text-[11px] font-semibold text-emerald-300">
            <span className="size-2 rounded-full bg-emerald-400 animate-pulse" />
            <span>In Progress ({groupedTasks.inProgress.length})</span>
          </div>
          <div className="space-y-1.5">
            {groupedTasks.inProgress.map((task) => {
              const isSelected = selectedTaskId === task.id;
              return (
                <div
                  key={task.id}
                  onClick={() => onSelectTask?.(task.id)}
                  className={`p-2.5 rounded-xl border transition cursor-pointer space-y-1.5 ${
                    isSelected
                      ? 'bg-white/[0.08] border-emerald-500/40 text-white shadow-xs'
                      : 'bg-white/[0.02] hover:bg-white/[0.05] border-white/8 text-white/80'
                  }`}
                >
                  <div className="flex items-center justify-between gap-1 text-[10px]">
                    <span className="font-mono px-1.5 py-0.5 rounded bg-white/8 text-white/60 text-[9px]">
                      {task.contract?.pack_id || 'task'}
                    </span>
                    <span className="flex items-center gap-1 text-emerald-400 text-[10px] font-mono">
                      <span className="size-1.5 rounded-full bg-emerald-400 animate-pulse" />
                      {String(task.status).toLowerCase()}
                    </span>
                  </div>
                  <div className="text-xs font-medium line-clamp-2 text-white/95">{task.title || task.id}</div>
                  <div className="flex items-center justify-between text-[10px] text-white/40 pt-0.5 font-mono">
                    <span>{task.id.slice(0, 8)}</span>
                    <span>{formatRelativeTime(task.updated_at || task.created_at)}</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Completed & Standby Section */}
      {groupedTasks.standbyCompleted.length > 0 && !filterActiveOnly && (
        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5 px-1 text-[11px] font-semibold text-white/50">
            <CheckSquare className="size-3 text-white/40" />
            <span>Completed & Standby ({groupedTasks.standbyCompleted.length})</span>
          </div>
          <div className="space-y-1">
            {groupedTasks.standbyCompleted.map((task) => {
              const isSelected = selectedTaskId === task.id;
              const isDone =
                String(task.status).toLowerCase() === 'done' ||
                String(task.status).toLowerCase() === 'succeeded';
              return (
                <div
                  key={task.id}
                  onClick={() => onSelectTask?.(task.id)}
                  className={`px-2.5 py-1.5 rounded-lg border transition cursor-pointer flex items-center justify-between text-xs ${
                    isSelected
                      ? 'bg-white/10 border-white/20 text-white'
                      : 'hover:bg-white/[0.03] border-transparent text-white/60'
                  }`}
                >
                  <div className="flex items-center gap-2 min-w-0 mr-2">
                    <span
                      className={`size-1.5 rounded-full shrink-0 ${isDone ? 'bg-white/40' : 'bg-rose-400/60'}`}
                    />
                    <span className="truncate">{task.title || task.id}</span>
                  </div>
                  <span className="text-[10px] text-white/30 font-mono shrink-0">
                    {formatRelativeTime(task.updated_at || task.created_at)}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
