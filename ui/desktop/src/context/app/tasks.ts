import { useState, useCallback } from 'react';
import { daemonClient } from '@/api/daemon_client';
import type { Task, TaskStatus, CreateTaskParams } from '@/types/domain';

export interface TasksSlice {
  tasks: Task[];
  setTasks: React.Dispatch<React.SetStateAction<Task[]>>;
  refreshTasks: () => Promise<Task[]>;
  createTask: (params: CreateTaskParams) => Promise<Task>;
  advanceTask: (taskId: string, status: TaskStatus) => Promise<Task>;
  completeTask: (taskId: string, summary?: string) => Promise<Task>;
  cancelTask: (taskId: string, reason?: string) => Promise<Task>;
}

export function useTasksSlice(showToast: (msg: string) => void): TasksSlice {
  const [tasks, setTasks] = useState<Task[]>([]);

  const refreshTasks = useCallback(async () => {
    const fetchedTasks = await daemonClient.listTasks();
    setTasks(fetchedTasks);
    return fetchedTasks;
  }, []);

  const refreshAfterTaskMutation = useCallback(
    async (successMessage: string) => {
      try {
        await refreshTasks();
        showToast(successMessage);
      } catch (error) {
        showToast(
          `${successMessage}, but the Task list could not refresh: ${
            error instanceof Error ? error.message : String(error)
          }`
        );
      }
    },
    [refreshTasks, showToast]
  );

  const createTask = useCallback(
    async (params: CreateTaskParams) => {
      const created = await daemonClient.createTask(params);
      await refreshAfterTaskMutation(`Task "${created.title}" created`);
      return created;
    },
    [refreshAfterTaskMutation]
  );

  const advanceTask = useCallback(
    async (taskId: string, status: TaskStatus) => {
      const advanced = await daemonClient.advanceTask(taskId, status);
      await refreshAfterTaskMutation('Task status updated');
      return advanced;
    },
    [refreshAfterTaskMutation]
  );

  const completeTask = useCallback(
    async (taskId: string, summary?: string) => {
      const completed = await daemonClient.completeTask(taskId, summary);
      await refreshAfterTaskMutation('Task marked as complete');
      return completed;
    },
    [refreshAfterTaskMutation]
  );

  const cancelTask = useCallback(
    async (taskId: string, reason?: string) => {
      const cancelled = await daemonClient.cancelTask(taskId, reason);
      await refreshAfterTaskMutation('Task cancelled');
      return cancelled;
    },
    [refreshAfterTaskMutation]
  );

  return {
    tasks,
    setTasks,
    refreshTasks,
    createTask,
    advanceTask,
    completeTask,
    cancelTask,
  };
}
