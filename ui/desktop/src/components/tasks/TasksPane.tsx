import React, { useState, useMemo } from 'react';
import { Task, TaskStatus, CreateTaskParams, ExecutionWorkspace } from '@/types/domain';
import { Session } from '@/types';
import { TaskToolbar } from './TaskToolbar';
import { TaskKanbanBoard } from './TaskKanbanBoard';
import { TaskListView } from './TaskListView';
import { TaskDetailView } from './TaskDetailView';
import { CreateTaskModal } from './CreateTaskModal';
import { TaskViewMode, TaskFilterOptions, getTaskBucket } from './types';

interface TasksPaneProps {
  tasks: Task[];
  sessions: Session[];
  workspaces: ExecutionWorkspace[];
  selectedTaskId: string | null;
  onSelectTask: (taskId: string | null) => void;
  onOpenSession: (sessionId: string) => void;
  onNewSessionForTask: (taskId: string) => void;
  onCreateTask: (params: CreateTaskParams) => Promise<Task>;
  onAdvanceStatus: (taskId: string, status: TaskStatus) => Promise<void>;
  onCompleteTask: (taskId: string, summary?: string) => Promise<void>;
  onCancelTask: (taskId: string, reason?: string) => Promise<void>;
  onRefreshTasks: () => Promise<Task[]>;
}

export const TasksPane: React.FC<TasksPaneProps> = ({
  tasks,
  sessions,
  workspaces,
  selectedTaskId,
  onSelectTask,
  onOpenSession,
  onNewSessionForTask,
  onCreateTask,
  onAdvanceStatus,
  onCompleteTask,
  onCancelTask,
  onRefreshTasks,
}) => {
  const [viewMode, setViewMode] = useState<TaskViewMode>(() => {
    try {
      return (localStorage.getItem('custos.tasks_view_mode') as TaskViewMode) || 'board';
    } catch {
      return 'board';
    }
  });

  const [filters, setFilters] = useState<TaskFilterOptions>({
    query: '',
    packId: 'all',
    status: 'all',
  });

  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleViewModeChange = (mode: TaskViewMode) => {
    setViewMode(mode);
    try {
      localStorage.setItem('custos.tasks_view_mode', mode);
    } catch {}
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await onRefreshTasks();
    } finally {
      setIsRefreshing(false);
    }
  };

  // Filter out internal chat turn tasks
  const realTasks = useMemo(() => {
    return tasks.filter((t) => !t.metadata?.turn_task);
  }, [tasks]);

  // Apply search query, pack filter, status filter
  const filteredTasks = useMemo(() => {
    return realTasks.filter((task) => {
      // Query filter
      if (filters.query.trim()) {
        const q = filters.query.toLowerCase();
        const matchesTitle = task.title.toLowerCase().includes(q);
        const matchesDesc = task.contract?.description?.toLowerCase().includes(q);
        const matchesPack = task.contract?.pack_id?.toLowerCase().includes(q);
        if (!matchesTitle && !matchesDesc && !matchesPack) return false;
      }

      // Pack filter
      if (filters.packId !== 'all') {
        const p = (task.contract?.pack_id || task.metadata?.pack_id || '').toLowerCase();
        if (filters.packId === 'code' && !p.includes('code') && !p.includes('eng')) return false;
        if (filters.packId === 'research' && !p.includes('research') && !p.includes('science'))
          return false;
        if (filters.packId === 'copilot' && !p.includes('copilot') && !p.includes('assist'))
          return false;
      }

      // Status filter
      if (filters.status !== 'all') {
        const bucket = getTaskBucket(task.status);
        if (filters.status === 'draft' && bucket !== 'draft') return false;
        if (filters.status === 'running' && bucket !== 'running') return false;
        if (filters.status === 'blocked' && bucket !== 'attention') return false;
        if (filters.status === 'done' && bucket !== 'done') return false;
      }

      return true;
    });
  }, [realTasks, filters]);

  const selectedTask = selectedTaskId ? realTasks.find((t) => t.id === selectedTaskId) : null;

  return (
    <div className="flex h-full w-full flex-col overflow-hidden bg-surface-base px-6 py-5">
      {selectedTask ? (
        <TaskDetailView
          task={selectedTask}
          sessions={sessions}
          workspaces={workspaces}
          onBack={() => onSelectTask(null)}
          onOpenSession={onOpenSession}
          onNewSessionForTask={onNewSessionForTask}
          onAdvanceStatus={onAdvanceStatus}
          onCompleteTask={onCompleteTask}
          onCancelTask={onCancelTask}
        />
      ) : (
        <div className="flex h-full min-h-0 flex-col gap-4">
          <TaskToolbar
            filters={filters}
            onChangeFilters={setFilters}
            viewMode={viewMode}
            onChangeViewMode={handleViewModeChange}
            onCreateTask={() => setIsCreateModalOpen(true)}
            onRefresh={handleRefresh}
            isRefreshing={isRefreshing}
            totalTasks={filteredTasks.length}
          />

          <div className="flex-1 min-h-0">
            {viewMode === 'board' ? (
              <TaskKanbanBoard
                tasks={filteredTasks}
                sessions={sessions}
                workspaces={workspaces}
                onSelectTask={(id) => onSelectTask(id)}
                onCreateTask={() => setIsCreateModalOpen(true)}
              />
            ) : (
              <TaskListView
                tasks={filteredTasks}
                sessions={sessions}
                workspaces={workspaces}
                onSelectTask={(id) => onSelectTask(id)}
                onCreateTask={() => setIsCreateModalOpen(true)}
              />
            )}
          </div>
        </div>
      )}

      {/* Create Task Modal */}
      <CreateTaskModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onSubmit={async (params) => {
          const created = await onCreateTask(params);
          onSelectTask(created.id);
        }}
        workspaces={workspaces}
      />
    </div>
  );
};
