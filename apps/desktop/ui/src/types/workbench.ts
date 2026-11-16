import type { ComponentType, LazyExoticComponent, ReactNode } from 'react';

// Canonical definition of the four workbench lenses in Custos SADE.
export type ActiveWorkbench = 'copilot' | 'engineering' | 'research' | 'quant';
export type AppWorkspaceMode = ActiveWorkbench;
export type CanonicalWorkbenchId = 'assistant' | 'coding' | 'research' | 'quant' | 'unsupported';
export type WorkbenchId = CanonicalWorkbenchId | 'copilot' | 'engineering';

export type PanePosition = 'main' | 'bottom' | 'side';
export type PaneWidth = 'full' | 'half' | 'third';

export interface PaneProps {
  workspaceId?: string;
  taskId?: string;
  job?: any;
  className?: string;
  onClose?: () => void;
  onRefresh?: () => void;
}

export interface PaneContribution {
  id: string;
  title: string;
  icon?: any;
  component?: ComponentType<any> | LazyExoticComponent<any>;
  requiredCapabilities?: string[];
  defaultWidth?: PaneWidth;
  position?: PanePosition;
}

export interface SidebarContribution {
  id: string;
  title: string;
  component?: ComponentType<any> | LazyExoticComponent<any>;
}

export interface WorkbenchCommandContribution {
  id: string;
  label: string;
  shortcut?: string;
  action: () => void | Promise<void>;
}

export interface WorkbenchContribution {
  id: WorkbenchId;
  canonicalId: CanonicalWorkbenchId;
  label: string;
  icon?: any;
  shortcut: string;
  builtin: boolean;
  description: string;
  color?: string;
  supportedTabTypes: string[];
  panes?: PaneContribution[];
  sidebar?: SidebarContribution;
  commands?: WorkbenchCommandContribution[];
  defaultLayout?: string;
}

export interface WorkbenchPaneProps {
  title?: string;
  paneId?: string;
  workspaceId?: string;
  taskId?: string;
  job?: any;
  isLoading?: boolean;
  error?: string | null;
  emptyState?: ReactNode;
  toolbar?: ReactNode;
  statusBar?: ReactNode;
  children: ReactNode;
  onRefresh?: () => void;
  className?: string;
}
