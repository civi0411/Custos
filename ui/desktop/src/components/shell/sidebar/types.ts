import React from 'react';
import type { Session } from '@/types';
import type { ExecutionWorkspace, Task, WorkspaceKind } from '@/types/domain';

export const normPath = (p?: string): string => (p ? p.trim().replace(/\/+$/, '') : '');

export const getWorkspaceKindLocation = (kind?: WorkspaceKind): string | undefined => {
  if (!kind) return undefined;
  if (kind.type === 'git') return kind.repo_path;
  if (kind.type === 'folder') return kind.path;
  if (kind.type === 'remote_ssh') return kind.remote_path;
  return undefined;
};

export const matchWorkspacePath = (ws: ExecutionWorkspace, targetPath?: string): boolean => {
  if (!targetPath) return false;
  const targetNorm = normPath(targetPath);
  if (ws.path && normPath(ws.path) === targetNorm) return true;
  const kindPath = getWorkspaceKindLocation(ws.kind);
  return Boolean(kindPath && normPath(kindPath) === targetNorm);
};

export const formatRelativeTime = (iso?: string): string => {
  if (!iso) return '';
  try {
    const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
    if (diff < 60) return 'just now';
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  } catch {
    return '';
  }
};

export interface SidebarTool {
  icon: React.ElementType;
  label: string;
  onClick: () => void;
  rightElement?: React.ReactNode;
  iconColor?: string;
}

export interface WorktreeItem {
  id: string;
  name: string;
  branch: string;
  path: string;
  isMain: boolean;
  isDirty: boolean;
  dirtyCount: number;
  isRunning?: boolean;
  isBlocked?: boolean;
  sessionCount?: number;
  agentActivity?: {
    status: 'running' | 'blocked' | 'completed';
    title: string;
    model?: string;
    timeAgo?: string;
  } | null;
}

export interface WorkspaceItem {
  id: string;
  name: string;
  path: string;
  kind?: string;
  worktrees: WorktreeItem[];
}

export interface ProjectGroupItem {
  id: string;
  name: string;
  path: string;
  isMultiRepo: boolean;
  workspaces: WorkspaceItem[];
}

export interface WorkspaceSidebarProps {
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions?: Session[];
  allSessions?: Session[];
  activeSessionId?: string;
  onSelectSession?: (id: string) => void;
  tasks?: Task[];
  selectedTaskId?: string | null;
  onSelectTask?: (id: string) => void;
  onClearTask?: () => void;
  listMode?: 'chats' | 'tasks';
  onChangeListMode?: (mode: 'chats' | 'tasks') => void;
  onOpenFullChatIn?: (mode: any, sessionId?: string) => void;
  onOpenSessionHere?: (id: string) => void;
  onDeleteSession?: (id: string) => void | Promise<void>;

  // Workbench
  activeWorkbench?: 'engineering' | 'research' | 'copilot' | string;
  onSwitchWorkbench?: (wb: any) => void;
  onSwitchMode?: (mode: any) => void;

  // Header button
  newTaskLabel?: string;
  newTaskIcon?: React.ElementType;
  newTaskIconColor?: string;
  onNewSession?: () => void;
  onRequestCreateTask?: () => void;

  // Nav Tools (optional)
  tools?: SidebarTool[];

  // Lists
  sessionsTitle?: string;
  emptySessionsMessage?: string;
  projectActiveColor?: string;

  // Footer Avatar
  avatarColor?: string;
  avatarText?: string;
  avatarTextColor?: string;

  // Callbacks
  onShowToast?: (msg: string) => void;
  onOpenSettings?: (tab?: string) => void;
  onOpenAppGrid?: () => void;
  isSidebarCollapsed?: boolean;
  workspaces?: ExecutionWorkspace[];
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  refreshWorkspaces?: () => Promise<ExecutionWorkspace[]>;
  onOpenTab?: (id: any, customUrl?: string, customTitle?: string, targetPane?: 'primary' | 'split', targetWorkspaceId?: string) => void;
}
