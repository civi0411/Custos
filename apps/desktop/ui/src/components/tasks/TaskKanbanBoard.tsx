import React from 'react';
import { Task, ExecutionWorkspace } from '@/types/domain';
import { Session } from '@/types';
import { TaskCard } from './TaskCard';
import { TaskBucket, getTaskBucket } from './types';
import {
  ListTodo,
  PlayCircle,
  AlertCircle,
  CheckCircle2,
} from 'lucide-react';

interface TaskKanbanBoardProps {
  tasks: Task[];
  sessions: Session[];
  workspaces: ExecutionWorkspace[];
  onSelectTask: (taskId: string) => void;
  onCreateTask: () => void;
}

const COLUMNS: {
  bucket: TaskBucket;
  title: string;
  icon: React.ElementType;
  colorClass: string;
  countBadgeClass: string;
}[] = [
  {
    bucket: 'draft',
    title: 'Backlog / Draft',
    icon: ListTodo,
    colorClass: 'text-fg-muted',
    countBadgeClass: 'bg-surface-3 text-fg-muted',
  },
  {
    bucket: 'running',
    title: 'In Progress',
    icon: PlayCircle,
    colorClass: 'text-sky-400',
    countBadgeClass: 'bg-sky-500/20 text-sky-400',
  },
  {
    bucket: 'attention',
    title: 'Needs Review',
    icon: AlertCircle,
    colorClass: 'text-amber-400',
    countBadgeClass: 'bg-amber-500/20 text-amber-400',
  },
  {
    bucket: 'done',
    title: 'Completed',
    icon: CheckCircle2,
    colorClass: 'text-emerald-400',
    countBadgeClass: 'bg-emerald-500/20 text-emerald-400',
  },
];

export const TaskKanbanBoard: React.FC<TaskKanbanBoardProps> = ({
  tasks,
  sessions,
  workspaces,
  onSelectTask,
  onCreateTask,
}) => {
  // Group tasks by bucket
  const groupedTasks = React.useMemo(() => {
    const map: Record<TaskBucket, Task[]> = {
      draft: [],
      running: [],
      attention: [],
      done: [],
    };
    for (const t of tasks) {
      const bucket = getTaskBucket(t.status);
      map[bucket].push(t);
    }
    return map;
  }, [tasks]);

  // Map workspace names by task/workspace ID
  const workspaceMap = React.useMemo(() => {
    const map = new Map<string, string>();
    for (const w of workspaces) {
      map.set(w.id, w.name);
    }
    return map;
  }, [workspaces]);

  // Count sessions linked to each task
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

  return (
    <div className="flex h-full min-h-0 w-full gap-4 overflow-x-auto pb-4 pt-1">
      {COLUMNS.map((col) => {
        const colTasks = groupedTasks[col.bucket];
        const Icon = col.icon;
        return (
          <div
            key={col.bucket}
            className="flex h-full min-w-[280px] flex-1 flex-col rounded-xl border border-border-default/70 bg-surface-base/50 backdrop-blur-sm"
          >
            {/* Column Header */}
            <div className="flex items-center justify-between border-b border-border-muted/50 px-3.5 py-3">
              <div className="flex items-center gap-2">
                <Icon className={`size-4 ${col.colorClass}`} />
                <span className="text-xs font-semibold tracking-tight text-fg-editor">
                  {col.title}
                </span>
              </div>
              <span
                className={`rounded-full px-2 py-0.5 text-[10px] font-mono font-medium tabular-nums ${col.countBadgeClass}`}
              >
                {colTasks.length}
              </span>
            </div>

            {/* Column Task Cards */}
            <div className="flex-1 space-y-2.5 overflow-y-auto p-3 scrollbar-sleek">
              {colTasks.length === 0 ? (
                <div className="flex h-32 flex-col items-center justify-center rounded-lg border border-dashed border-border-muted/50 p-4 text-center">
                  <p className="text-[11px] text-fg-muted/70">No tasks</p>
                  {col.bucket === 'draft' && (
                    <button
                      type="button"
                      onClick={onCreateTask}
                      className="mt-2 text-[11px] font-medium text-workbench-accent hover:underline"
                    >
                      + Create first task
                    </button>
                  )}
                </div>
              ) : (
                colTasks.map((task) => {
                  const wsName =
                    task.metadata?.workspace_id &&
                    workspaceMap.get(task.metadata.workspace_id);
                  return (
                    <TaskCard
                      key={task.id}
                      task={task}
                      linkedSessionCount={sessionCountByTask.get(task.id) || 0}
                      workspaceName={wsName || undefined}
                      onClick={() => onSelectTask(task.id)}
                    />
                  );
                })
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
};
