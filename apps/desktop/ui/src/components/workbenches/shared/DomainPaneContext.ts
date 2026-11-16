import React from 'react';
import type { Session } from '@/types';
import type { OrcaTab, OrcaTabId, ResourceDefinition, ResourceTabsState } from '../workbenchResources';

export interface DomainPaneContext {
  tab: OrcaTab;
  tabType: string;
  tabResource?: ResourceDefinition;
  session?: Session | null;
  tasks?: import('@/types/domain').Task[];
  allSessions?: Session[];
  workspaces?: import('@/types/domain').ExecutionWorkspace[];
  activeWorkspaceId?: string;
  initialAgentWorkspaceId?: string;
  initialFilesWorkspaceId?: string;
  initialTerminalWorkspaceId?: string;
  setInitialAgentWorkspaceId: (id: string | undefined) => void;
  setInitialFilesWorkspaceId: (id: string | undefined) => void;
  setInitialTerminalWorkspaceId: (id: string | undefined) => void;
  isResourceActive?: boolean;
  resourceTabs: ResourceTabsState;
  frameTabResource: (content: React.ReactNode) => React.ReactNode;
  onShowToast?: (msg: string) => void;
  onAskAgent?: (draft: string, sessionId?: string) => void;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string, targetPane?: 'primary' | 'split', targetWorkspaceId?: string) => void;
  onSelectWorkspace?: (id: string) => void;
  onHandoffToCoding?: (claims: unknown[], result: import('@/types/domain').HandoffToCodingResult) => void;
}
