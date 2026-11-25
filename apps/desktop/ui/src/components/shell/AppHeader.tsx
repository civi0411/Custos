import React from 'react';
import type { ActiveWorkbench, AppWorkspaceMode } from '@/types/workbench';
import { getTabIcon, type ResourceTabsState } from '@/components/workbenches/OrcaTabbedContainer';

export type { ActiveWorkbench, AppWorkspaceMode };
export { getTabIcon };

/**
 * @deprecated Legacy header props. Tab rendering is canonically handled by OrcaTabbedContainer.
 */
export interface AppHeaderProps {
  mode?: ActiveWorkbench;
  workspaceTitle?: string;
  activeWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  onSwitchMode?: (mode: ActiveWorkbench) => void;
  isSidebarCollapsed?: boolean;
  onToggleSidebar?: () => void;
  onShowToast?: (msg: string) => void;
  splitPercent?: number;
  isWebTabOpen?: boolean;
  isWebTabExpanded?: boolean;
  onToggleWebTab?: () => void;
  onToggleExpandWebTab?: () => void;
  resourceTabs: ResourceTabsState;
  onCloseTab: (e: React.MouseEvent, tabId: string) => void;
  onAddNewTab: () => void;
  lensBadge?: string;
  lensIconColor?: string;
  onOpenCommandPalette?: () => void;
  attentionCount?: number;
  onOpenAttention?: () => void;
  onOpenKanban?: () => void;
  runningJobsByTab?: Record<string, number>;
}

export type ResourceTabHeaderProps = AppHeaderProps;

/**
 * @deprecated Legacy tab header. Active tab rendering and canvas layout are handled by OrcaTabbedContainer.
 */
export const ResourceTabHeader: React.FC<ResourceTabHeaderProps> = () => null;

export const AppHeader = ResourceTabHeader;
