import React from 'react';
import type { AppearancePreferences } from '@/lib/appearance';
import type {
  MainTab,
  ProjectData,
  ProviderItem,
  ClientApiKey,
  Session,
  ModelCombo,
  ConnectedAccount,
} from '@/types';
import type {
  ExecutionWorkspace,
  Task,
  TaskStatus,
  CreateTaskParams,
  WorkspaceDetectionResult,
  RunHandle,
} from '@/types/domain';
import type { OrcaTab, ResourceTabsState } from '@/components/workbenches/OrcaTabbedContainer';
import type { ActiveWorkbench } from './canvas_layout';

export interface AppContextType {
  // Projects & Sessions
  projectData: ProjectData;
  setProjectData: React.Dispatch<React.SetStateAction<ProjectData>>;
  currentProject: string;
  setCurrentProject: (name: string) => void;
  activeSessionId: string;
  setActiveSessionId: (id: string) => void;
  activeSession: Session | null;
  currentSessions: Session[];
  tasks: Task[];
  refreshTasks: () => Promise<Task[]>;
  createTask: (params: CreateTaskParams) => Promise<Task>;
  advanceTask: (taskId: string, status: TaskStatus) => Promise<Task>;
  completeTask: (taskId: string, summary?: string) => Promise<Task>;
  cancelTask: (taskId: string, reason?: string) => Promise<Task>;

  // Workspaces & Physical Execution Context
  workspaces: ExecutionWorkspace[];
  activeWorkspaceId: string;
  setActiveWorkspaceId: (id: string) => void;
  activeWorkspace: ExecutionWorkspace | null;
  refreshWorkspaces: () => Promise<ExecutionWorkspace[]>;

  // Navigation & Layout
  currentTab: MainTab;
  setCurrentTab: (tab: MainTab) => void;
  activeWorkbench: ActiveWorkbench;
  setActiveWorkbench: (wb: ActiveWorkbench) => void;
  installedWorkbenches: string[];
  refreshInstalledWorkbenches: () => Promise<void>;
  isWorkbenchLibraryOpen: boolean;
  setIsWorkbenchLibraryOpen: React.Dispatch<React.SetStateAction<boolean>>;
  activeScreenId: string;
  setActiveScreenId: React.Dispatch<React.SetStateAction<string>>;
  initScreenTabs: (screenId: string, initialTabs: OrcaTab[], initialActiveTabId?: string) => void;
  isSessionsCollapsed: boolean;
  setIsSessionsCollapsed: React.Dispatch<React.SetStateAction<boolean>>;
  isRightExplorerOpen: boolean;
  setIsRightExplorerOpen: React.Dispatch<React.SetStateAction<boolean>>;
  toggleRightExplorer: () => void;

  // Providers & Keys
  providers: ProviderItem[];
  setProviders: React.Dispatch<React.SetStateAction<ProviderItem[]>>;
  clientKeys: ClientApiKey[];
  setClientKeys: React.Dispatch<React.SetStateAction<ClientApiKey[]>>;

  // Routing Combos & Usage
  combos: ModelCombo[];
  setCombos: React.Dispatch<React.SetStateAction<ModelCombo[]>>;
  handleCreateCombo: (combo: Omit<ModelCombo, 'id' | 'createdAt'>) => void;
  handleUpdateCombo: (id: string, updates: Partial<ModelCombo>) => void;
  handleDeleteCombo: (id: string) => void;
  connectedAccounts: ConnectedAccount[];
  refreshModels: () => Promise<void>;
  setConnectedAccounts: React.Dispatch<React.SetStateAction<ConnectedAccount[]>>;
  handleConnectOAuth: (providerId: string, accountName: string, email?: string) => Promise<void>;
  handleDeleteAccount: (accountId: string) => Promise<void>;

  // Modals & Profile
  isSettingsOpen: boolean;
  setIsSettingsOpen: (open: boolean) => void;
  settingsTab: string;
  setSettingsTab: (tab: string) => void;
  openSettings: (tab?: string) => void;
  isAiSetupOpen: boolean;
  setIsAiSetupOpen: (open: boolean) => void;
  aiSetupTab: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas' | 'tools' | 'environments' | 'diagnostics';
  setAiSetupTab: (tab: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas' | 'tools' | 'environments' | 'diagnostics') => void;
  openAiSetup: (tab?: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas' | 'tools' | 'environments' | 'diagnostics') => void;
  isNewSessionOpen: boolean;
  setIsNewSessionOpen: (open: boolean) => void;
  isOpenProjectOpen: boolean;
  setIsOpenProjectOpen: (open: boolean) => void;
  handleProjectOpened: (project: { name: string; path: string; isGit: boolean; branch?: string }) => void;
  handleOpenFolderDirect: () => Promise<void>;
  activeDetection: WorkspaceDetectionResult | null;
  refreshWorkspaceDetection: () => Promise<void>;
  isAddProviderOpen: boolean;
  setIsAddProviderOpen: (open: boolean) => void;
  isPluginsOpen: boolean;
  setIsPluginsOpen: (open: boolean) => void;
  openPlugins: () => void;
  isSkillsOpen: boolean;
  setIsSkillsOpen: (open: boolean) => void;
  openSkills: () => void;
  isHelpOpen: boolean;
  setIsHelpOpen: (open: boolean) => void;
  openHelp: () => void;
  isGitHubAuthOpen: boolean;
  setIsGitHubAuthOpen: (open: boolean) => void;
  openGitHubAuth: () => void;
  handleGitHubLogin: (account: { username: string; name?: string; avatar_url?: string }) => Promise<boolean>;
  handleGitHubLogout: () => void;
  userProfile: {
    name: string;
    initials: string;
    tier: string;
    githubUsername?: string;
    avatarUrl?: string;
    isAuthenticated?: boolean;
  };
  setUserProfile: React.Dispatch<React.SetStateAction<{
    name: string;
    initials: string;
    tier: string;
    githubUsername?: string;
    avatarUrl?: string;
    isAuthenticated?: boolean;
  }>>;
  isSidebarCollapsed: boolean;
  setIsSidebarCollapsed: React.Dispatch<React.SetStateAction<boolean>>;
  toggleSidebar: () => void;
  isRecentActivityOpen: boolean;
  setIsRecentActivityOpen: React.Dispatch<React.SetStateAction<boolean>>;

  // Canvas & Explorer Layout
  isWebTabOpen: boolean;
  setIsWebTabOpen: React.Dispatch<React.SetStateAction<boolean>>;
  toggleWebTab: () => void;
  isWebTabExpanded: boolean;
  setIsWebTabExpanded: React.Dispatch<React.SetStateAction<boolean>>;
  toggleExpandWebTab: () => void;
  isInspectorOpen: boolean;
  setIsInspectorOpen: React.Dispatch<React.SetStateAction<boolean>>;
  toggleInspector: () => void;
  isResourcePickerOpen: boolean;
  setIsResourcePickerOpen: React.Dispatch<React.SetStateAction<boolean>>;
  resourceTabs: ResourceTabsState;
  handleCloseResourceTab: (e: React.MouseEvent, tabId: string) => void;
  splitPercent: number;
  setSplitPercent: React.Dispatch<React.SetStateAction<number>>;
  splitTabId: string | null;
  setSplitTabId: React.Dispatch<React.SetStateAction<string | null>>;
  splitRatio: number;
  setSplitRatio: React.Dispatch<React.SetStateAction<number>>;
  toggleSplitTab: (tabId: string) => void;
  closeSplitTab: () => void;
  column2Width: number;
  setColumn2Width: React.Dispatch<React.SetStateAction<number>>;
  isResizingColumn: boolean;
  handleStartResizeColumn2: (e: React.MouseEvent) => void;
  handleResetColumn2Width: () => void;

  // Device-local appearance
  appearance: AppearancePreferences;
  resolvedTheme: 'light' | 'dark';
  updateAppearance: (next: Partial<AppearancePreferences>) => void;

  // UI Scale
  uiScale: number;
  handleSetUiScale: (val: number) => void;
  handleStepUiScale: (delta: number) => void;

  // Toast & Notifications
  toastMessage: string | null;
  showToast: (msg: string) => void;
  savedWorkError: string | null;
  retrySavedWork: () => void;

  // Selected Model
  selectedModel: string;
  selectedModelAvailable: boolean;
  selectedModelAccountId: string;
  selectedReasoningEffort: string;
  handleSelectModel: (model: string, effort?: string, accountId?: string) => void;

  // Actions
  startNativeConversation: (instruction: string, harness: string, workspace: string) => Promise<{ run: RunHandle; sessionId: string }>;
  handleStopRun: (runId: string) => Promise<void>;
  handleSendMessage: (text: string, lens?: 'assistant' | 'coding' | 'research' | 'quant') => Promise<boolean>;
  handleClearHistory: () => void;
  handleDeleteConversation: (sessionId: string) => Promise<void>;
  handleAcceptAndRun: () => Promise<void>;
  handleRejectDiff: () => void;
  handleCopyDiff: () => void;
  handleCreateNewSession: (title: string, pack: 'engineering' | 'research' | 'assistant' | 'quant') => Promise<void>;
  handleCreateChatSession: (pack: 'engineering' | 'research' | 'assistant' | 'quant', taskId?: string) => Promise<void>;
  handleNewProjectPrompt: () => void;
  handleSaveProvider: (
    serviceOrParams: string | { service: string; accountName?: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
    apiKey?: string
  ) => Promise<void>;
  handleDeleteProvider: (id: string) => Promise<void> | void;
  handleTestConnection: (providerNameOrId: string) => Promise<void> | void;
  handleGenerateClientKey: () => Promise<void> | void;
  handleRevokeClientKey: (id: string) => Promise<void> | void;
}
