import React from 'react';
import { CommandPalette } from '@/components/shell/CommandPalette';
import { CreateTaskModal } from '@/components/tasks/CreateTaskModal';
import type { ActiveWorkbench } from '@/components/shell/AppHeader';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import type { ExecutionWorkspace, Task, CreateTaskParams } from '@/types/domain';
import type { Session } from '@/types';

export interface CreateTaskPrefill {
  title?: string;
  description?: string;
  workspaceId?: string;
  sessionId?: string;
  packId?: 'code' | 'research' | 'copilot';
}

interface ShellModalsProps {
  // Command Palette
  isCommandPaletteOpen: boolean;
  onCloseCommandPalette: () => void;
  onSwitchMode: (mode: ActiveWorkbench) => void;
  openTab: (id: OrcaTabId, title: string, url: string) => void;
  onNewSession: () => void;
  onOpenSettings: () => void;
  mode: ActiveWorkbench;

  // Create Task Modal
  isGlobalCreateTaskOpen: boolean;
  onCloseCreateTask: () => void;
  createTaskPrefill: CreateTaskPrefill;
  createTask: (params: CreateTaskParams) => Promise<Task>;
  setProjectData: React.Dispatch<React.SetStateAction<Record<string, Session[]>>>;
  onShowToast?: (msg: string) => void;
  onSelectTask: (id: string) => void;
  onChangeSidebarView: (view: 'chats' | 'tasks') => void;
  workspaces: ExecutionWorkspace[];
  activeWorkspaceId?: string;
}

export const ShellModals: React.FC<ShellModalsProps> = ({
  isCommandPaletteOpen,
  onCloseCommandPalette,
  onSwitchMode,
  openTab,
  onNewSession,
  onOpenSettings,
  mode,
  isGlobalCreateTaskOpen,
  onCloseCreateTask,
  createTaskPrefill,
  createTask,
  setProjectData,
  onShowToast,
  onSelectTask,
  onChangeSidebarView,
  workspaces,
  activeWorkspaceId,
}) => {
  return (
    <>
      <CommandPalette
        isOpen={isCommandPaletteOpen}
        onClose={onCloseCommandPalette}
        onSwitchMode={onSwitchMode}
        onOpenTab={openTab}
        onNewSession={onNewSession}
        onOpenSettings={onOpenSettings}
        onOpenWorktrees={() => openTab('worktrees', 'Worktrees', 'custos://worktrees')}
        currentMode={mode}
      />

      <CreateTaskModal
        isOpen={isGlobalCreateTaskOpen}
        onClose={onCloseCreateTask}
        onSubmit={async (params) => {
          const created = await createTask(params);
          if (createTaskPrefill.sessionId) {
            setProjectData((prev) => {
              const next = { ...prev };
              for (const p of Object.keys(next)) {
                next[p] = next[p].map((s) =>
                  s.id === createTaskPrefill.sessionId
                    ? { ...s, taskId: created.id, task_id: created.id }
                    : s
                );
              }
              return next;
            });
            onShowToast?.(`Task "${created.title}" created from conversation`);
          }
          onSelectTask(created.id);
          onChangeSidebarView('tasks');
          onCloseCreateTask();
        }}
        workspaces={workspaces}
        currentWorkspaceId={createTaskPrefill.workspaceId || activeWorkspaceId}
        initialTitle={createTaskPrefill.title}
        initialDescription={createTaskPrefill.description}
        initialWorkspaceId={createTaskPrefill.workspaceId || activeWorkspaceId}
        initialSessionId={createTaskPrefill.sessionId}
        initialPackId={createTaskPrefill.packId}
      />
    </>
  );
};
