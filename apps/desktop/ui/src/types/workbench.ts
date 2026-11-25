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
  job?: unknown;
  className?: string;
  onClose?: () => void;
  onRefresh?: () => void;
}

export interface PaneContribution {
  id: string;
  title: string;
  icon?: ComponentType<{ className?: string }>;
  component?: ComponentType<PaneProps> | LazyExoticComponent<ComponentType<PaneProps>>;
  requiredCapabilities?: string[];
  defaultWidth?: PaneWidth;
  position?: PanePosition;
}

export interface SidebarContribution {
  id: string;
  title: string;
  component?: ComponentType<Record<string, unknown>> | LazyExoticComponent<ComponentType<Record<string, unknown>>>;
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
  icon: ComponentType<{ className?: string }>;
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
  job?: unknown;
  isLoading?: boolean;
  error?: string | null;
  emptyState?: ReactNode;
  toolbar?: ReactNode;
  statusBar?: ReactNode;
  children: ReactNode;
  onRefresh?: () => void;
  className?: string;
}
