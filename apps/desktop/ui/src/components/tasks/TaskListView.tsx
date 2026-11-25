import React from 'react';
import { Task, ExecutionWorkspace } from '@/types/domain';
import { Session } from '@/types';
import { getPackInfo, getTaskBucket } from './types';
import {
  ListTodo,
  AlertCircle,
  CheckCircle2,
  FolderGit2,
  MessageSquare,
  Clock,
  ChevronRight,
} from 'lucide-react';

interface TaskListViewProps {
  tasks: Task[];
  sessions: Session[];
  workspaces: ExecutionWorkspace[];
  onSelectTask: (taskId: string) => void;
  onCreateTask: () => void;
}

export const TaskListView: React.FC<TaskListViewProps> = ({
  tasks,
  sessions,
  workspaces,
  onSelectTask,
  onCreateTask,
}) => {
  const workspaceMap = React.useMemo(() => {
    const map = new Map<string, string>();
    for (const w of workspaces) {
      map.set(w.id, w.name);
    }
    return map;
  }, [workspaces]);

  const sessionCountByTask = React.useMemo(() => {
    const map = new Map<string, number>();
    for (const s of sessions) {
      const tId = s.task_id || s.taskId;
      if (tId) {
        map.set(tId, (map.get(tId) || 0) + 1);
      }
    }
    return map;
  }, [sessions]);

  if (tasks.length === 0) {
    return (
      <div className="flex h-64 flex-col items-center justify-center rounded-xl border border-dashed border-border-default p-8 text-center">
        <ListTodo className="size-8 text-fg-muted/50 mb-3" />
        <h3 className="text-sm font-medium text-fg-editor">No tasks match your filters</h3>
        <p className="mt-1 text-xs text-fg-muted">Create a new task to organize your workspace workflow.</p>
        <button
          type="button"
          onClick={onCreateTask}
          className="mt-4 rounded-lg bg-workbench-accent px-4 py-2 text-xs font-medium text-white shadow-sm hover:opacity-90"
        >
          + Create Task
        </button>
      </div>
    );
  }

  return (
    <div className="h-full overflow-hidden rounded-xl border border-border-default/80 bg-surface-base/60 backdrop-blur-sm">
      <div className="overflow-x-auto">
        <table className="w-full text-left text-xs">
          <thead>
            <tr className="border-b border-border-default bg-surface-1/70 text-[11px] font-medium uppercase tracking-wider text-fg-muted select-none">
              <th className="px-4 py-3">Status</th>
              <th className="px-4 py-3">Task Title & Spec</th>
              <th className="px-4 py-3">Domain</th>
              <th className="px-4 py-3">Criteria</th>
              <th className="px-4 py-3">Workspace</th>
              <th className="px-4 py-3">Chats</th>
              <th className="px-4 py-3 text-right">Updated</th>
              <th className="w-10 px-2 py-3" />
            </tr>
          </thead>
          <tbody className="divide-y divide-border-muted/40">
            {tasks.map((task) => {
              const bucket = getTaskBucket(task.status);
              const packInfo = getPackInfo(task.contract?.pack_id || task.metadata?.pack_id);
              const criteria = task.contract?.criteria || [];
              const sessionCount = sessionCountByTask.get(task.id) || 0;
              const wsName =
                task.metadata?.workspace_id && workspaceMap.get(task.metadata.workspace_id);

              return (
                <tr
                  key={task.id}
                  onClick={() => onSelectTask(task.id)}
                  className="group hover:bg-surface-2/60 cursor-pointer transition-colors duration-100"
                >
                  {/* Status */}
                  <td className="px-4 py-3 whitespace-nowrap">
                    {bucket === 'running' && (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-sky-500/10 px-2.5 py-1 text-[10px] font-medium text-sky-400">
                        <span className="relative flex h-1.5 w-1.5">
                          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-sky-400 opacity-75" />
                          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-sky-500" />
                        </span>
                        In Progress
                      </span>
                    )}
                    {bucket === 'attention' && (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-amber-500/10 px-2.5 py-1 text-[10px] font-medium text-amber-400">
                        <AlertCircle className="size-3" />
                        Review
                      </span>
                    )}
                    {bucket === 'done' && (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-1 text-[10px] font-medium text-emerald-400">
                        <CheckCircle2 className="size-3" />
                        Done
                      </span>
                    )}
                    {bucket === 'draft' && (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-surface-3 px-2.5 py-1 text-[10px] font-medium text-fg-muted">
                        <ListTodo className="size-3" />
                        Draft
                      </span>
                    )}
                  </td>

                  {/* Title & Description */}
                  <td className="px-4 py-3 max-w-md">
                    <div className="font-medium text-fg-editor group-hover:text-workbench-accent truncate">
                      {task.title}
                    </div>
                    {task.contract?.description && (
                      <div className="text-[11px] text-fg-muted truncate mt-0.5">
                        {task.contract.description}
                      </div>
                    )}
                  </td>

                  {/* Domain Pack */}
                  <td className="px-4 py-3 whitespace-nowrap">
                    <span
                      className={`inline-flex items-center rounded border px-2 py-0.5 text-[10px] font-medium ${packInfo.badgeClass}`}
                    >
                      {packInfo.label}
                    </span>
                  </td>

                  {/* Criteria */}
                  <td className="px-4 py-3 whitespace-nowrap text-fg-muted">
                    {criteria.length > 0 ? (
                      <span className="font-mono text-[11px] tabular-nums">
                        {criteria.length} {criteria.length === 1 ? 'criterion' : 'criteria'}
                      </span>
                    ) : (
                      <span className="text-[11px] text-fg-muted/60">—</span>
                    )}
                  </td>

                  {/* Workspace */}
                  <td className="px-4 py-3 whitespace-nowrap text-fg-muted">
                    {wsName ? (
                      <span className="inline-flex items-center gap-1.5 truncate max-w-[140px]">
                        <FolderGit2 className="size-3 shrink-0 text-fg-muted" />
                        <span className="truncate">{wsName}</span>
                      </span>
                    ) : (
                      <span className="text-fg-muted/60">—</span>
                    )}
                  </td>

                  {/* Chats */}
                  <td className="px-4 py-3 whitespace-nowrap text-fg-muted">
                    {sessionCount > 0 ? (
                      <span className="inline-flex items-center gap-1 text-fg-editor font-mono">
                        <MessageSquare className="size-3" />
                        {sessionCount}
                      </span>
                    ) : (
                      <span className="text-fg-muted/60">0</span>
                    )}
                  </td>

                  {/* Updated */}
                  <td className="px-4 py-3 whitespace-nowrap text-right text-[11px] text-fg-muted tabular-nums">
                    <span className="inline-flex items-center gap-1">
                      <Clock className="size-2.5 opacity-60" />
                      {new Date(task.updated_at || task.created_at).toLocaleDateString(undefined, {
                        month: 'short',
                        day: 'numeric',
                      })}
                    </span>
                  </td>

                  {/* Action arrow */}
                  <td className="px-2 py-3 text-right">
                    <ChevronRight className="size-4 text-fg-muted/50 group-hover:text-fg-editor group-hover:translate-x-0.5 transition-transform" />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
};
