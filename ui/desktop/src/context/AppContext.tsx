import { APPEARANCE_KEY, readAppearance, applyAppearance, type AppearancePreferences } from '@/lib/appearance';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import React, { createContext, useContext, useState, useEffect, useRef, useCallback, useMemo } from 'react';
import {
  MainTab,
  ProjectData,
  ProviderItem,
  ClientApiKey,
  Session,
  ModelCombo,
  ConnectedAccount,
  ProviderServiceId,
} from '../types';
import { daemonClient } from '../api/daemon_client';
import {
  ExecutionWorkspace,
  ModelCatalogOption,
  OAuthTokenRecord,
  ProviderConfigRecord,
  Task,
  TaskStatus,
  CreateTaskParams,
  WorkspaceDetectionResult,
} from '../types/domain';
import type { OrcaTab, ResourceTabsState } from '@/components/workbenches/OrcaTabbedContainer';

export type ActiveWorkbench = 'copilot' | 'engineering' | 'research';

interface AppContextType {
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
  // Workspaces & Physical Execution Context (SSOT matching Left and Right)
  workspaces: ExecutionWorkspace[];
  activeWorkspaceId: string;
  setActiveWorkspaceId: (id: string) => void;
  activeWorkspace: ExecutionWorkspace | null;
  refreshWorkspaces: () => Promise<ExecutionWorkspace[]>;

  // Navigation & Layout
  currentTab: MainTab;
  setCurrentTab: (tab: MainTab) => void;
  activeWorkbench: 'engineering' | 'research' | 'copilot';
  setActiveWorkbench: (wb: 'engineering' | 'research' | 'copilot') => void;
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

  // Routing Combos & Usage (OmniRoute / 9Router style)
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

  // Modals
  isSettingsOpen: boolean;
  setIsSettingsOpen: (open: boolean) => void;
  settingsTab: string;
  setSettingsTab: (tab: string) => void;
  openSettings: (tab?: string) => void;
  isAiSetupOpen: boolean;
  setIsAiSetupOpen: (open: boolean) => void;
  aiSetupTab: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas';
  setAiSetupTab: (tab: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas') => void;
  openAiSetup: (tab?: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas') => void;
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
  handleGitHubLogin: (account: { username: string; token?: string; name?: string; avatar_url?: string }) => Promise<boolean>;
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
  isWebTabOpen: boolean;
  setIsWebTabOpen: React.Dispatch<React.SetStateAction<boolean>>;
  toggleWebTab: () => void;
  isWebTabExpanded: boolean;
  setIsWebTabExpanded: React.Dispatch<React.SetStateAction<boolean>>;
  toggleExpandWebTab: () => void;
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

  // Toast
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
  startNativeConversation: (instruction: string, harness: string, workspace: string) => Promise<{ run: import('../types/domain').RunHandle; sessionId: string }>;
  handleStopRun: (runId: string) => Promise<void>;
  handleSendMessage: (text: string, lens?: 'assistant' | 'coding' | 'research') => Promise<void>;
  handleClearHistory: () => void;
  handleDeleteConversation: (sessionId: string) => Promise<void>;
  handleAcceptAndRun: () => Promise<void>;
  handleRejectDiff: () => void;
  handleCopyDiff: () => void;
  handleCreateNewSession: (title: string, pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
  handleCreateChatSession: (pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
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

function mapBackendProvider(p: any): ProviderItem {
  const pType = (p.service_type || p.provider_type || '').toLowerCase();
  const iconType = (
    pType.includes('claude') || pType.includes('anthropic') ? 'anthropic' :
    pType.includes('openai') || pType.includes('gpt') ? 'openai' :
    pType.includes('gemini') ? 'gemini' :
    'deepseek'
  ) as ProviderItem['iconType'];

  const status = (['primary', 'standby', 'failover', 'offline'].includes(p.status)
    ? p.status
    : p.status === 'configured' || p.status === 'active' || p.is_active ? 'primary' : 'offline') as ProviderItem['status'];

  const modelName = p.default_model || p.model || 'Model not reported';

  return {
    id: p.id,
    name: p.name,
    model: modelName,
    status,
    statusLabel: p.status === 'configured' ? 'Configured' : p.status_label || (status === 'primary' ? 'Enabled' : 'Offline'),
    badgeColor: status === 'primary' ? '#10b981' : status === 'standby' ? '#3b82f6' : '#6b7280',
    apiKey: p.api_key_masked || (p.api_key && p.api_key.trim().length > 0 ? `${p.api_key.slice(0, 6)}••••••••` : 'Chưa cấu hình'),
    quotaUsed: p.quota_used || undefined,
    quotaTotal: p.quota_total || undefined,
    quotaPercent: typeof p.quota_percent === 'number' ? p.quota_percent : undefined,
    endpoint: p.endpoint_url || p.endpoint || undefined,
    defaultModel: p.default_model || p.model || undefined,
    contextWindow: p.context_window || undefined,
    fastMode: p.fast_mode || undefined,
    latency: typeof p.latency_ms === 'number' ? `${p.latency_ms}ms` : 'Not measured',
    iconType,
  };
}

function mapBackendKey(k: any): ClientApiKey {
  let createdDate = new Date().toISOString().split('T')[0];
  if (k.created_at) {
    const epoch = typeof k.created_at === 'number' && k.created_at < 1e11 ? k.created_at * 1000 : k.created_at;
    createdDate = new Date(epoch).toISOString().split('T')[0];
  }
  return {
    id: k.id,
    name: k.name,
    token: k.token,
    created: createdDate,
    icon: (k.name || '').toLowerCase().includes('cli') ? 'terminal' : 'laptop',
  };
}

const PROVIDER_LABELS: Record<ProviderServiceId, string> = {
  antigravity: 'Google Antigravity',
  copilot: 'GitHub Copilot',
  gemini: 'Google Gemini',
  openai: 'OpenAI Platform',
  anthropic: 'Anthropic Claude',
  deepseek: 'DeepSeek',
  openrouter: 'OpenRouter Gateway',
  groq: 'Groq Cloud',
  local: 'Local Inference',
  custom: 'Custom Gateway',
};

const normalizeProviderServiceId = (service?: string): ProviderServiceId => {
  const s = (service || '').toLowerCase();
  if (s.includes('antigravity')) return 'antigravity';
  if (s.includes('copilot')) return 'copilot';
  if (s.includes('anthropic') || s.includes('claude')) return 'anthropic';
  if (s.includes('openai') || s.includes('gpt') || s.includes('codex')) return 'openai';
  if (s.includes('gemini') || s.includes('google')) return 'gemini';
  if (s.includes('deepseek')) return 'deepseek';
  if (s.includes('openrouter')) return 'openrouter';
  if (s.includes('groq')) return 'groq';
  if (s.includes('local') || s.includes('ollama')) return 'local';
  return 'custom';
};

const defaultProbeBaseUrl = (service: ProviderServiceId): string | undefined => {
  switch (service) {
    case 'copilot':
      return 'https://api.githubcopilot.com';
    case 'antigravity':
      return 'https://cloudcode-pa.googleapis.com';
    case 'openai':
      return 'https://api.openai.com/v1';
    case 'anthropic':
      return 'https://api.anthropic.com/v1';
    case 'gemini':
      return 'https://generativelanguage.googleapis.com';
    case 'deepseek':
      return 'https://api.deepseek.com/v1';
    case 'local':
      return 'http://localhost:11434';
    default:
      return undefined;
  }
};

const isProviderConnected = (p: ProviderConfigRecord): boolean => {
  const status = (p.status || '').toLowerCase();
  const statusConnected =
    status === 'configured' ||
    status === 'active' ||
    status === 'primary' ||
    status === 'standby';
  const localProvider = normalizeProviderServiceId(p.service_type) === 'local';
  return statusConnected && (localProvider || p.api_key_masked !== 'none');
};

const safeIsoFromMillis = (value?: number): string => {
  if (!value || value > 253402300799000) {
    return new Date().toISOString();
  }
  return new Date(value).toISOString();
};

const tokenLooksOauth = (token?: OAuthTokenRecord | null): boolean =>
  Boolean(token?.has_refresh_token || token?.scope?.trim());

const uniqueModels = (models: string[]): string[] => {
  const seen = new Set<string>();
  return models.filter((model) => {
    const key = model.trim().toLowerCase();
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
};

const AI_LOCAL_ACCOUNT_ID = 'ai-local-auto';
const AI_LOCAL_ENDPOINT = 'http://localhost:11434';

const isTransientAiLocalAccount = (account?: ConnectedAccount): boolean =>
  account?.id === AI_LOCAL_ACCOUNT_ID;

export const getDefaultTabForMode = (m: ActiveWorkbench): { tabs: OrcaTab[]; activeTabId: string } => {
  if (m === 'engineering') {
    return {
      tabs: [
        { id: 'files', type: 'files', title: 'Files & Changes', url: 'custos://resource/files' },
        { id: 'activity', type: 'activity', title: 'Activity', url: 'custos://resource/activity' },
      ],
      activeTabId: 'files',
    };
  }
  if (m === 'research') {
    return {
      tabs: [
        { id: 'literature', type: 'literature', title: 'Sources', url: 'custos://resource/literature' },
        { id: 'findings', type: 'findings', title: 'Findings', url: 'custos://resource/findings' },
        { id: 'lab', type: 'lab', title: 'Lab', url: 'custos://resource/lab' },
      ],
      activeTabId: 'literature',
    };
  }
  return {
    tabs: [
      { id: 'drafts', type: 'drafts', title: 'Drafts', url: 'custos://resource/drafts' },
      { id: 'outbox', type: 'outbox', title: 'Outbox', url: 'custos://resource/outbox' },
      { id: 'notes', type: 'notes', title: 'Notes', url: 'custos://resource/notes' },
    ],
    activeTabId: 'drafts',
  };
};

const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Projects & Sessions state
  const [projectData, setProjectData] = useState<ProjectData>({ 'Custos Workspace': [] });
  const [currentProject, setCurrentProject] = useState<string>('Custos Workspace');
  const [activeDetection, setActiveDetection] = useState<WorkspaceDetectionResult | null>(null);
  const [activeSessionId, setActiveSessionId] = useState<string>('');
  const [tasks, setTasks] = useState<Task[]>([]);

  // Workspaces SSOT state (Authoritative execution context matching Left and Right)
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceIdState] = useState<string>(() => {
    try {
      return localStorage.getItem('custos.active_workspace_id') || '';
    } catch {
      return '';
    }
  });

  const setActiveWorkspaceId = useCallback((id: string) => {
    setActiveWorkspaceIdState(id);
    try {
      if (id) {
        localStorage.setItem('custos.active_workspace_id', id);
      }
    } catch {}
  }, []);

  const refreshWorkspaces = useCallback(async () => {
    try {
      const list = await daemonClient.listWorkspaces();
      const valid = list.filter((w) => {
        const s = typeof w.status === 'string' ? w.status : (w.status as any)?.setup_failed ? 'failed' : 'unknown';
        return s !== 'archived';
      });
      // Live sync dirty manifest for active workspace so left badge, header, and diff are always synchronized
      const currentActive = localStorage.getItem('custos.active_workspace_id') || '';
      if (currentActive && valid.some((w) => w.id === currentActive)) {
        try {
          const dirty = await daemonClient.inspectWorkspaceDirty(currentActive);
          const idx = valid.findIndex((w) => w.id === currentActive);
          if (idx >= 0) {
            valid[idx] = { ...valid[idx], dirty_manifest: dirty, dirtyManifest: dirty };
          }
        } catch {}
      }
      setWorkspaces(valid);
      if (valid.length > 0) {
        setActiveWorkspaceIdState((prev) => {
          if (prev && valid.some((w) => w.id === prev)) return prev;
          const ready = valid.find((w) => w.status === 'ready') || valid[0];
          return ready.id;
        });
      }
      return valid;
    } catch (e) {
      console.warn('Failed to load workspaces:', e);
      return [];
    }
  }, []);

  const activeWorkspace = React.useMemo(() => {
    return workspaces.find((w) => w.id === activeWorkspaceId) || null;
  }, [workspaces, activeWorkspaceId]);

  // Toast state
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [savedWorkError, setSavedWorkError] = useState<string | null>(null);
  const [savedWorkAttempt, setSavedWorkAttempt] = useState(0);
  const retrySavedWork = useCallback(() => setSavedWorkAttempt((attempt) => attempt + 1), []);
  const toastTimeoutRef = useRef<number | null>(null);

  const showToast = useCallback((msg: string) => {
    if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current);
    setToastMessage(msg);
    toastTimeoutRef.current = window.setTimeout(() => {
      setToastMessage(null);
    }, 2500);
  }, []);

  const refreshTasks = useCallback(async () => {
    const fetchedTasks = await daemonClient.listTasks();
    setTasks(fetchedTasks);
    return fetchedTasks;
  }, []);

  const refreshAfterTaskMutation = useCallback(async (successMessage: string) => {
    try {
      await refreshTasks();
      showToast(successMessage);
    } catch (error) {
      showToast(`${successMessage}, but the Task list could not refresh: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, [refreshTasks, showToast]);

  const handleCreateTask = useCallback(
    async (params: CreateTaskParams) => {
      const created = await daemonClient.createTask(params);
      await refreshAfterTaskMutation(`Task "${created.title}" created`);
      return created;
    },
    [refreshAfterTaskMutation]
  );

  const handleAdvanceTask = useCallback(
    async (taskId: string, status: TaskStatus) => {
      const advanced = await daemonClient.advanceTask(taskId, status);
      await refreshAfterTaskMutation('Task status updated');
      return advanced;
    },
    [refreshAfterTaskMutation]
  );

  const handleCompleteTask = useCallback(
    async (taskId: string, summary?: string) => {
      const completed = await daemonClient.completeTask(taskId, summary);
      await refreshAfterTaskMutation('Task marked as complete');
      return completed;
    },
    [refreshAfterTaskMutation]
  );

  const handleCancelTask = useCallback(
    async (taskId: string, reason?: string) => {
      const cancelled = await daemonClient.cancelTask(taskId, reason);
      await refreshAfterTaskMutation('Task cancelled');
      return cancelled;
    },
    [refreshAfterTaskMutation]
  );

  useEffect(() => {
    void refreshWorkspaces();
    const interval = setInterval(() => {
      void refreshWorkspaces();
    }, 4000);
    const onFocus = () => {
      void refreshWorkspaces();
    };
    window.addEventListener('focus', onFocus);
    return () => {
      clearInterval(interval);
      window.removeEventListener('focus', onFocus);
    };
  }, [refreshWorkspaces]);

  const refreshWorkspaceDetection = useCallback(async () => {
    const currentPath = localStorage.getItem('custos_current_project_path');
    if (currentPath) {
      try {
        const det = await daemonClient.detectWorkspace(currentPath);
        setActiveDetection(det);
      } catch (e) {
        console.warn('Failed to detect workspace:', e);
      }
    }
  }, []);

  useEffect(() => {
    const currentPath = localStorage.getItem('custos_current_project_path');
    if (currentPath) {
      daemonClient.detectWorkspace(currentPath).then((det) => {
        setActiveDetection(det);
        if (det.name && det.name !== 'Project') {
          setCurrentProject(det.name);
        }
      }).catch(() => {});
    } else {
      daemonClient.listWorkspaces().then((list) => {
        const active = list.find((w) => w.status !== 'archived');
        if (active && active.path) {
          daemonClient.detectWorkspace(active.path).then((det) => {
            setActiveDetection(det);
            if (det.name) setCurrentProject(det.name);
          }).catch(() => {});
        }
      }).catch(() => {});
    }
  }, []);

  // Navigation & Layout state
  const [currentTab, setCurrentTab] = useState<MainTab>('studio');
  const [activeWorkbench, setActiveWorkbenchState] = useState<'engineering' | 'research' | 'copilot'>(() => {
    try {
      const saved = localStorage.getItem('custos.workspace.mode.v2');
      if (saved === 'copilot' || saved === 'chat') return 'copilot';
      if (saved === 'engineering' || saved === 'code') return 'engineering';
      if (saved === 'research') return 'research';
    } catch { }
    return 'engineering';
  });

  const setActiveWorkbench = useCallback((wb: 'engineering' | 'research' | 'copilot') => {
    setActiveWorkbenchState(wb);
    try {
      localStorage.setItem('custos.workspace.mode.v2', wb);
    } catch { }
  }, []);

  const [isSessionsCollapsed, setIsSessionsCollapsed] = useState<boolean>(false);
  const [isRightExplorerOpen, setIsRightExplorerOpen] = useState<boolean>(true);

  const toggleRightExplorer = useCallback(() => {
    setIsRightExplorerOpen((prev) => !prev);
  }, []);

  // Providers & Keys state
  const [providers, setProviders] = useState<ProviderItem[]>([]);
  const [clientKeys, setClientKeys] = useState<ClientApiKey[]>([]);

  // Combos & Usage state (OmniRoute / 9Router style)
  const [combos, setCombos] = useState<ModelCombo[]>(() => {
    try {
      const saved = localStorage.getItem('custos_combos');
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_combos', JSON.stringify(combos));
    } catch (e) {
      console.warn('Failed to save combos to localStorage', e);
    }
  }, [combos]);

  const [connectedAccounts, setConnectedAccounts] = useState<ConnectedAccount[]>(() => {
    return [];
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_connected_accounts', JSON.stringify(connectedAccounts));
    } catch (e) {
      console.warn('Failed to save connected accounts to localStorage', e);
    }
  }, [connectedAccounts]);

  // Modals state
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [settingsTab, setSettingsTab] = useState<string>('general');
  const [isAiSetupOpen, setIsAiSetupOpen] = useState<boolean>(false);
  const [aiSetupTab, setAiSetupTab] = useState<'local' | 'cloud' | 'routing' | 'subagents' | 'quotas'>('local');
  const openAiSetup = useCallback((tab?: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas') => {
    if (tab) setAiSetupTab(tab);
    setIsAiSetupOpen(true);
  }, []);
  const [isNewSessionOpen, setIsNewSessionOpen] = useState<boolean>(false);
  const [isOpenProjectOpen, setIsOpenProjectOpen] = useState<boolean>(false);
  const [isAddProviderOpen, setIsAddProviderOpen] = useState<boolean>(false);
  const [isPluginsOpen, setIsPluginsOpen] = useState<boolean>(false);
  const [isSkillsOpen, setIsSkillsOpen] = useState<boolean>(false);
  const [isHelpOpen, setIsHelpOpen] = useState<boolean>(false);

  const openPlugins = useCallback(() => setIsPluginsOpen(true), []);
  const openSkills = useCallback(() => setIsSkillsOpen(true), []);
  const openHelp = useCallback(() => setIsHelpOpen(true), []);

  const [isGitHubAuthOpen, setIsGitHubAuthOpen] = useState<boolean>(false);
  const openGitHubAuth = useCallback(() => setIsGitHubAuthOpen(true), []);

  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(false);
  const toggleSidebar = useCallback(() => setIsSidebarCollapsed((prev) => !prev), []);

  const [isWebTabOpen, setIsWebTabOpen] = useState<boolean>(() => {
    try {
      const saved = localStorage.getItem('custos_resource_pane_open');
      if (saved !== null) return saved === 'true';
    } catch {}
    return false;
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_resource_pane_open', String(isWebTabOpen));
    } catch {}
  }, [isWebTabOpen]);

  const toggleWebTab = useCallback(() => setIsWebTabOpen((prev) => !prev), []);

  const [isWebTabExpanded, setIsWebTabExpanded] = useState<boolean>(false);
  const toggleExpandWebTab = useCallback(() => setIsWebTabExpanded((prev) => !prev), []);
  const [isResourcePickerOpen, setIsResourcePickerOpen] = useState<boolean>(false);

  const [tabStates, setTabStates] = useState<Record<string, { tabs: OrcaTab[]; activeTabId: string }>>({});
  const tabScope = `${currentProject}:${activeSessionId || 'no-session'}:${activeWorkbench}`;

  const currentTabState = useMemo(() => {
    return tabStates[tabScope] ?? getDefaultTabForMode(activeWorkbench);
  }, [tabStates, tabScope, activeWorkbench]);

  const setTabs = useCallback((update: React.SetStateAction<OrcaTab[]>) => {
    setTabStates((previous) => {
      const current = previous[tabScope] ?? getDefaultTabForMode(activeWorkbench);
      const tabs = typeof update === 'function' ? update(current.tabs) : update;
      return { ...previous, [tabScope]: { ...current, tabs } };
    });
  }, [tabScope, activeWorkbench]);

  const setActiveTabId = useCallback((update: React.SetStateAction<string>) => {
    setTabStates((previous) => {
      const current = previous[tabScope] ?? getDefaultTabForMode(activeWorkbench);
      const activeTabId = typeof update === 'function' ? update(current.activeTabId) : update;
      return { ...previous, [tabScope]: { ...current, activeTabId } };
    });
  }, [tabScope, activeWorkbench]);

  const [splitPercent, setSplitPercent] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('custos_split_pct');
      if (saved) {
        const val = parseFloat(saved);
        if (!isNaN(val) && val >= 20 && val <= 80) return val;
      }
    } catch {}
    return 42;
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_split_pct', splitPercent.toString());
    } catch {}
  }, [splitPercent]);

  const [splitTabId, setSplitTabId] = useState<string | null>(null);
  const [splitRatio, setSplitRatio] = useState<number>(50);
  const [column2Width, setColumn2Width] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('custos_column2_width');
      if (saved) {
        const val = parseInt(saved, 10);
        if (!isNaN(val) && val >= 260 && val <= 1600) return val;
      }
    } catch {}
    return 480;
  });

  const [isResizingColumn, setIsResizingColumn] = useState<boolean>(false);

  useEffect(() => {
    try {
      localStorage.setItem('custos_column2_width', column2Width.toString());
    } catch {}
  }, [column2Width]);

  const handleStartResizeColumn2 = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsResizingColumn(true);
  }, []);

  const handleResetColumn2Width = useCallback(() => {
    const def = Math.max(320, Math.min(540, Math.round(window.innerWidth * 0.38)));
    setColumn2Width(def);
  }, []);

  // Global mousemove and mouseup listeners when isResizingColumn is true
  useEffect(() => {
    if (!isResizingColumn) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = Math.round(window.innerWidth - e.clientX);
      const minCol2 = 260;
      const leftReserved = (isSidebarCollapsed ? 44 : 44 + 256) + 320;
      const maxCol2 = Math.max(minCol2, window.innerWidth - leftReserved);
      const clamped = Math.min(Math.max(newWidth, minCol2), maxCol2);
      setColumn2Width(clamped);
      setSplitPercent(Math.round(((window.innerWidth - clamped) / window.innerWidth) * 100));
    };

    const handleMouseUp = () => {
      setIsResizingColumn(false);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingColumn, isSidebarCollapsed]);

  // Window resize clamp
  useEffect(() => {
    const handleWindowResize = () => {
      const minCol2 = 260;
      const leftReserved = (isSidebarCollapsed ? 44 : 44 + 256) + 320;
      const maxCol2 = Math.max(minCol2, window.innerWidth - leftReserved);
      setColumn2Width((prev) => Math.min(Math.max(prev, minCol2), maxCol2));
    };

    window.addEventListener('resize', handleWindowResize);
    return () => window.removeEventListener('resize', handleWindowResize);
  }, [isSidebarCollapsed]);

  const toggleSplitTab = useCallback((tabId: string) => {
    setSplitTabId((prev) => {
      if (prev === tabId) return null;
      if (tabId === currentTabState.activeTabId) {
        if (prev) return null;
        const other = currentTabState.tabs.find((t) => t.id !== tabId);
        return other ? other.id : null;
      }
      return tabId;
    });
  }, [currentTabState.activeTabId, currentTabState.tabs]);

  const closeSplitTab = useCallback(() => {
    setSplitTabId(null);
  }, []);

  const resourceTabs: ResourceTabsState = useMemo(() => ({
    ...currentTabState,
    setTabs,
    setActiveTabId,
    splitTabId,
    setSplitTabId,
    splitRatio,
    setSplitRatio,
    toggleSplitTab,
    closeSplitTab,
  }), [currentTabState, setTabs, setActiveTabId, splitTabId, splitRatio, toggleSplitTab, closeSplitTab]);

  const handleCloseResourceTab = useCallback((e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    if (splitTabId === tabId) {
      setSplitTabId(null);
    }
    const nextTabs = resourceTabs.tabs.filter((t) => t.id !== tabId);
    resourceTabs.setTabs(nextTabs);
    if (nextTabs.length === 0) {
      setIsWebTabOpen(false);
      return;
    }
    if (resourceTabs.activeTabId === tabId) {
      const closedIndex = resourceTabs.tabs.findIndex((t) => t.id === tabId);
      const nextActive = nextTabs[Math.max(0, closedIndex - 1)];
      resourceTabs.setActiveTabId(nextActive.id);
    }
  }, [resourceTabs, splitTabId]);

  const [userProfile, setUserProfile] = useState<{
    name: string;
    initials: string;
    tier: string;
    githubUsername?: string;
    avatarUrl?: string;
    isAuthenticated?: boolean;
  }>(() => {
    try {
      const saved = localStorage.getItem('custos_user_profile');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed && typeof parsed === 'object') return parsed;
      }
    } catch {}
    return {
      name: 'Trần Vĩ',
      initials: 'TV',
      tier: 'Plus',
      githubUsername: 'tranchivi',
      avatarUrl: 'https://github.com/tranchivi.png',
      isAuthenticated: true,
    };
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_user_profile', JSON.stringify(userProfile));
    } catch {}
  }, [userProfile]);

  const handleGitHubLogin = useCallback(
    async (account: { username: string; token?: string; name?: string; avatar_url?: string }) => {
      try {
        let finalName = account.name || account.username;
        let finalAvatar = account.avatar_url;

        if (account.token && !finalAvatar) {
          try {
            const res = await fetch('https://api.github.com/user', {
              headers: {
                Authorization: `Bearer ${account.token}`,
                Accept: 'application/vnd.github.v3+json',
              },
            });
            if (res.ok) {
              const data = await res.json();
              finalName = data.name || data.login || finalName;
              finalAvatar = data.avatar_url;
            }
          } catch {}
        } else if (!finalAvatar) {
          try {
            const res = await fetch(`https://api.github.com/users/${encodeURIComponent(account.username)}`);
            if (res.ok) {
              const data = await res.json();
              finalName = data.name || data.login || finalName;
              finalAvatar = data.avatar_url;
            }
          } catch {}
        }

        const initials =
          finalName
            .split(' ')
            .map((p: string) => p[0])
            .join('')
            .slice(0, 2)
            .toUpperCase() || account.username.slice(0, 2).toUpperCase();

        const updated = {
          name: finalName,
          initials,
          tier: 'GitHub Pro',
          githubUsername: account.username,
          avatarUrl: finalAvatar || `https://github.com/${encodeURIComponent(account.username)}.png`,
          isAuthenticated: true,
        };

        setUserProfile(updated);
        try {
          localStorage.setItem('custos_user_profile', JSON.stringify(updated));
          if (account.token) {
            localStorage.setItem('custos_github_token', account.token);
          }
        } catch {}
        showToast(`Connected to GitHub as @${account.username}`);
        return true;
      } catch (err: any) {
        showToast(`GitHub login failed: ${err?.message || err}`);
        return false;
      }
    },
    [showToast]
  );

  const handleGitHubLogout = useCallback(() => {
    const guestProfile = {
      name: 'Guest User',
      initials: 'GU',
      tier: 'Free',
      githubUsername: undefined,
      avatarUrl: undefined,
      isAuthenticated: false,
    };
    setUserProfile(guestProfile);
    try {
      localStorage.setItem('custos_user_profile', JSON.stringify(guestProfile));
      localStorage.removeItem('custos_github_token');
    } catch {}
    showToast('Signed out of GitHub');
  }, [showToast]);

  // Sync projects with daemon ExecutionWorkspaces on mount
  useEffect(() => {
    daemonClient.listWorkspaces().then((workspaces) => {
      const active = (workspaces || []).filter((w) => {
        const s = typeof w.status === 'string' ? w.status : (w.status as any)?.setup_failed ? 'failed' : 'unknown';
        return s !== 'archived' && s !== 'failed';
      });
      if (active.length > 0) {
        setProjectData((prev) => {
          const next = { ...prev };
          for (const ws of active) {
            const wsName = ws.name || ws.path.split('/').pop() || ws.id;
            if (!next[wsName]) {
              next[wsName] = [];
            }
          }
          return next;
        });
        const saved = localStorage.getItem('custos_current_project');
        if (saved && active.some((w) => (w.name || w.path.split('/').pop()) === saved)) {
          setCurrentProject(saved);
        } else {
          setCurrentProject(active[0].name || active[0].path.split('/').pop() || 'Custos Workspace');
        }
      }
    }).catch((err) => {
      console.warn('Failed to load daemon workspaces on boot:', err);
    });
  }, []);

  const openSettings = useCallback((tab?: string) => {
    if (tab === 'providers' || tab === 'runtime') {
      openAiSetup(tab === 'runtime' ? 'local' : 'cloud');
      return;
    }
    if (tab === 'subagents') {
      openAiSetup('subagents');
      return;
    }
    if (tab === 'quotas') {
      openAiSetup('quotas');
      return;
    }
    if (tab) setSettingsTab(tab);
    setIsSettingsOpen(true);
  }, [openAiSetup]);

  const [appearance, setAppearance] = useState<AppearancePreferences>(readAppearance);
  const [systemDark, setSystemDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches);
  const resolvedTheme = appearance.theme === 'system' ? (systemDark ? 'dark' : 'light') : appearance.theme;
  useEffect(() => {
    if (appearance.theme !== 'system') return;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => setSystemDark(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, [appearance.theme]);
  useEffect(() => {
    const sync = (event: StorageEvent) => {
      if (event.key !== APPEARANCE_KEY && event.key !== null) return;
      const saved = readAppearance();
      if (saved.theme === 'system') setSystemDark(window.matchMedia('(prefers-color-scheme: dark)').matches);
      setAppearance(saved);
    };
    window.addEventListener('storage', sync);
    return () => window.removeEventListener('storage', sync);
  }, []);
  useEffect(() => { applyAppearance(appearance, resolvedTheme); }, [appearance, resolvedTheme]);
  const themeQueue = useRef<Promise<void>>(Promise.resolve());
  useEffect(() => {
    if (!isTauri()) return;
    const theme = appearance.theme === 'system' ? null : resolvedTheme;
    themeQueue.current = themeQueue.current
      .then(async () => {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setTheme(theme);
      })
      .catch(error => console.warn('Native window theme could not be applied', error));
  }, [appearance.theme, resolvedTheme]);
  const updateAppearance = useCallback((next: Partial<AppearancePreferences>) => {
    if (next.theme === 'system') setSystemDark(window.matchMedia('(prefers-color-scheme: dark)').matches);
    setAppearance(previous => {
      const updated = { ...previous, ...next };
      try { localStorage.setItem(APPEARANCE_KEY, JSON.stringify(updated)); }
      catch (error) { console.warn('Device appearance could not be saved', error); }
      return updated;
    });
  }, []);

  // UI Scale state
  const [uiScale, setUiScale] = useState<number>(() => {
    const saved = localStorage.getItem('custos_ui_scale');
    const value = Number(saved || 100);
    return Math.min(160, Math.max(70, Math.round((Number.isFinite(value) ? value : 100) / 10) * 10));
  });

  const zoomQueue = useRef<Promise<void>>(Promise.resolve());
  const applyUiScale = useCallback((scaleVal: number) => {
    const rounded = Math.round((Number.isFinite(scaleVal) ? scaleVal : 100) / 10) * 10;
    const clamped = Math.min(160, Math.max(70, rounded));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
  }, []);

  useEffect(() => {
    const factor = uiScale / 100;
    document.documentElement.style.zoom = '';
    document.body.style.zoom = '';
    document.documentElement.style.setProperty('--app-browser-zoom', '1');
    if (isTauri()) {
      // Serialize native zoom changes so rapid clicks settle at the latest level.
      zoomQueue.current = zoomQueue.current.then(() => getCurrentWebview().setZoom(factor)).catch(error => {
        showToast(`Unable to zoom app: ${String(error)}`);
      });
    } else {
      document.documentElement.style.zoom = String(factor);
      document.documentElement.style.setProperty('--app-browser-zoom', String(factor));
    }
  }, [uiScale, showToast]);

  const handleSetUiScale = (val: number) => {
    applyUiScale(val);
  };

  const handleStepUiScale = (delta: number) => {
    handleSetUiScale(uiScale + delta);
  };

  // Check active session object
  const currentSessions = projectData[currentProject] || [];
  const activeSession: Session | null =
    currentSessions.find((s) => s.id === activeSessionId) || null;

  // Selected Model state: default empty string means "No model selected"
  const [selectedModel, setSelectedModel] = useState<string>(() => {
    return localStorage.getItem('custos_selected_model') || '';
  });

  const [selectedModelAccountId, setSelectedModelAccountId] = useState(() => localStorage.getItem('custos_selected_model_account') || '');
  const [selectedReasoningEffort, setSelectedReasoningEffort] = useState(() => localStorage.getItem('custos_selected_reasoning_effort') || '');

  const selectedModelAvailable = (() => {
    if (!selectedModel) return false;
    if (selectedModel.startsWith('native:')) return true;
    return connectedAccounts.some(account => account.status === 'active' && account.supportedModels.includes(selectedModel) &&
      (!selectedReasoningEffort || account.catalogModels?.some(model => model.id === selectedModel && model.efforts.includes(selectedReasoningEffort))));
  })();

  const refreshCatalogAvailability = useCallback(async () => {
    const updates = await Promise.all(localAccountsRef.current.filter(account => !isTransientAiLocalAccount(account)).map(async account => {
      try { return { id: account.id, endpoint: account.endpointUrl, catalog: await daemonClient.getModelCatalog(account.providerId, account.id) }; } catch { return null; }
    }));
    setConnectedAccounts(previous => previous.map(account => {
      const update = updates.find(value => value?.id === account.id && value.endpoint === account.endpointUrl);
      if (!update) return account;
      return { ...account, supportedModels: update.catalog.models.map(model => model.id), catalogModels: update.catalog.models,
        unavailableModels: update.catalog.unavailable_models, catalogError: Object.values(update.catalog.unavailable_models || {}).map(value => value.reason).join('; ') || (account.status !== 'active' ? account.catalogError : undefined) };
    }));
  }, []);

  const refreshProviderState = useCallback(async () => {
    const [fetchedProviders, fetchedKeys] = await Promise.all([
      daemonClient.listProviders(),
      daemonClient.listClientKeys(),
    ]);

    let providerRecords = fetchedProviders as ProviderConfigRecord[];
    let repairedOAuthProvider = false;

    const recoverableOAuthProviders: ProviderServiceId[] = ['openai', 'anthropic', 'gemini', 'deepseek'];
    for (const providerId of recoverableOAuthProviders) {
      const hasProvider = providerRecords.some((provider) =>
        provider.id === providerId ||
        normalizeProviderServiceId(provider.service_type || provider.id) === providerId
      );
      if (hasProvider) {
        continue;
      }

      const token = await daemonClient.getOAuthToken(providerId).catch(() => null);
      if (!tokenLooksOauth(token) || !token?.connected) {
        continue;
      }

      await daemonClient.saveProvider({
        id: providerId,
        name: `${PROVIDER_LABELS[providerId]} (OAuth)`,
        service_type: providerId,
        provider_type: providerId,
        api_key_masked: 'oauth',
        endpoint_url: defaultProbeBaseUrl(providerId),
        status: 'active',
        status_label: 'OAuth Active',
        fast_mode: false,
        is_active: true,
      }).catch(() => undefined);
      repairedOAuthProvider = true;
    }

    if (repairedOAuthProvider) {
      providerRecords = (await daemonClient.listProviders().catch(() => providerRecords)) as ProviderConfigRecord[];
    }

    setProviders(providerRecords.map(mapBackendProvider));
    setClientKeys(fetchedKeys.map(mapBackendKey));

    const accounts = await Promise.all(
      providerRecords.map(async (provider): Promise<ConnectedAccount | null> => {
        const serviceId = normalizeProviderServiceId(provider.service_type || provider.id);
        let connected = isProviderConnected(provider);
        const endpointUrl = provider.endpoint_url || defaultProbeBaseUrl(serviceId);

        let tokenMeta: OAuthTokenRecord | null = null;
        let credentialError: string | undefined;
        tokenMeta = await daemonClient.getOAuthToken(provider.id).catch((error) => { credentialError = error instanceof Error ? error.message : String(error); return null; });
        if (serviceId !== 'local') connected = connected && tokenMeta?.connected === true;

        let catalogModels: ModelCatalogOption[] = [];
        let catalogSource: 'live' | 'cached' | 'unavailable' = 'unavailable';
        let catalogError: string | undefined = credentialError;
        let catalogFetchedAt: number | undefined;
        let unavailableModels: ConnectedAccount['unavailableModels'];
        if (connected && endpointUrl) {
          try {
            const probed = await daemonClient.probeModels(
              endpointUrl,
              serviceId === 'local' && provider.default_model?.startsWith('ollama/') ? 'ollama' : provider.service_type || serviceId,
              undefined,
              provider.id
            );
            catalogSource = 'live';
            catalogFetchedAt = Date.now();
            catalogModels = probed.models.map((model, index) => ({
              id: model.id,
              label: model.name || model.id,
              description: model.description,
              provider_type: provider.service_type || serviceId,
              is_default: index === 0,
              default_effort: model.default_effort,
              efforts: model.efforts || [],
              supports_fast_mode: false,
              context_window: model.context_window || provider.context_window,
              pricing: undefined,
            }));
            const verified = await daemonClient.getModelCatalog(provider.service_type || serviceId, provider.id);
            catalogModels = verified.models;
            unavailableModels = verified.unavailable_models;
            catalogError = Object.values(unavailableModels || {}).map(value => value.reason).join('; ') || undefined;
          } catch (err) {
            catalogError = err instanceof Error ? err.message : String(err);
          }
        }

        if (connected && catalogSource !== 'live') {
          try {
            const cached = await daemonClient.getModelCatalog(provider.service_type || serviceId, provider.id);
            if (cached.origin === 'cached' && cached.models.length) {
              catalogModels = cached.models;
              catalogSource = 'cached';
              catalogFetchedAt = cached.fetched_at;
            }
          } catch (err) {
            catalogError = err instanceof Error ? err.message : String(err);
          }
        }

        const supportedModels = uniqueModels(catalogModels.map((model) => model.id));

        if (!credentialError && (!connected || (provider.api_key_masked === 'oauth' && !tokenMeta?.connected))) {
          return null;
        }

        const savedDefault = serviceId === 'local' && provider.default_model && !provider.default_model.startsWith('ollama/') && supportedModels.includes(`ollama/${provider.default_model}`)
          ? `ollama/${provider.default_model}` : provider.default_model;
        const defaultModel = savedDefault && supportedModels.includes(savedDefault)
          ? savedDefault : supportedModels[0] || '';
        const contextWindow =
          catalogModels.find((model) => model.id === defaultModel)?.context_window ||
          provider.context_window ||
          catalogModels[0]?.context_window;
        const available = connected && catalogSource === 'live';
        const authType = tokenLooksOauth(tokenMeta) || provider.api_key_masked === 'oauth'
          ? 'oauth'
          : 'api_key';

        return {
          id: provider.id,
          providerId: serviceId,
          providerName: PROVIDER_LABELS[serviceId] || provider.name,
          accountName: provider.name || `${PROVIDER_LABELS[serviceId]} Account`,
          authType,
          apiKeyMasked: authType === 'api_key' ? provider.api_key_masked : undefined,
          endpointUrl,
          oauthEmail: authType === 'oauth' && provider.name.includes('@') ? provider.name : undefined,
          oauthTier: authType === 'oauth' ? (serviceId === 'copilot' ? 'Connected via Device Flow' : 'Connected via PKCE') : undefined,
          oauthExpiresAt: tokenMeta?.expires_at ? safeIsoFromMillis(tokenMeta.expires_at) : undefined,
          status: available ? 'active' : 'offline',
          statusLabel: available ? 'Connected' : credentialError || (serviceId === 'local' ? 'Ollama offline — start Ollama and retry' : 'Reconnect provider'),
          badgeColor: available ? '#10b981' : '#6b7280',
          defaultModel,
          supportedModels,
          catalogModels, catalogSource, catalogError, catalogFetchedAt, unavailableModels,
          contextWindow,
          fastMode: provider.fast_mode,
          latencyMs: 0,
          createdAt: safeIsoFromMillis(provider.created_at),
          lastUsedAt: connected ? 'Connected' : undefined,
        };
      })
    );

    const liveAccounts = accounts.filter((account): account is ConnectedAccount => account !== null);
    const hasSavedLocalAccount = providerRecords.some(
      (provider) => normalizeProviderServiceId(provider.service_type || provider.id) === 'local'
    );

    if (!hasSavedLocalAccount) {
      try {
        const discovered = await daemonClient.probeModels(AI_LOCAL_ENDPOINT, 'ollama');
        const localModels = uniqueModels(discovered.models.map((model) => model.id));
        if (localModels.length > 0) {
          liveAccounts.unshift({
            id: AI_LOCAL_ACCOUNT_ID,
            providerId: 'local',
            providerName: 'AI Local',
            accountName: 'AI Local (Ollama)',
            authType: 'api_key',
            endpointUrl: AI_LOCAL_ENDPOINT,
            status: 'active',
            statusLabel: 'Ready without sign-in',
            badgeColor: '#10b981',
            defaultModel: localModels[0],
            supportedModels: localModels,
            catalogModels: discovered.models.map((model, index) => ({
              id: model.id,
              label: model.name || model.id,
              description: model.description,
              provider_type: 'local',
              is_default: index === 0,
              efforts: [],
              supports_fast_mode: false,
              context_window: model.context_window,
            })),
            catalogSource: 'live',
            catalogFetchedAt: Date.now(),
            contextWindow: discovered.models[0]?.context_window,
            latencyMs: 0,
            createdAt: new Date().toISOString(),
            lastUsedAt: 'Available locally',
          });
        }
      } catch {
        // AI Local is optional. Keep provider hydration usable when Ollama is not running.
      }
    }
    setConnectedAccounts(liveAccounts);

    const fallbackModel =
      liveAccounts.find((account) => account.status === 'active' && account.defaultModel)?.defaultModel ||
      liveAccounts.flatMap((account) => account.supportedModels)[0] ||
      '';

    setSelectedModel((previous) => {
      const current = previous || localStorage.getItem('custos_selected_model') || '';
      if (current) {
        return current;
      }

      localStorage.setItem('custos_selected_model', fallbackModel);
      return fallbackModel;
    });

    return { providers: providerRecords, keys: fetchedKeys, accounts: liveAccounts };
  }, []);

  const localAccountsRef = useRef(connectedAccounts);
  useEffect(() => {
    localAccountsRef.current = connectedAccounts;
  }, [connectedAccounts]);

  const localAccountsKey = useMemo(() => {
    return connectedAccounts
      .filter(a => a.providerId === 'local' && a.endpointUrl && (a.endpointUrl?.includes(':11434') || a.defaultModel?.startsWith('ollama/') || a.supportedModels.some(m => m.startsWith('ollama/'))))
      .map(a => `${a.id}:${a.endpointUrl}`)
      .join('|');
  }, [connectedAccounts]);

  // Availability does not mutate saved configuration or switch the chosen model.
  useEffect(() => {
    const accounts = localAccountsRef.current.filter(a => a.providerId === 'local' && a.endpointUrl && (a.endpointUrl?.includes(':11434') || a.defaultModel?.startsWith('ollama/') || a.supportedModels.some(m => m.startsWith('ollama/'))));
    if (!accounts.length) return;
    let cancelled = false;
    let probing = false;
    const check = async () => {
      if (probing || document.hidden) return;
      probing = true;
      try {
        await Promise.all(accounts.map(async account => {
          let updates: Partial<ConnectedAccount>;
          try {
            const result = await daemonClient.probeModels(account.endpointUrl!, 'ollama');
            updates = {
              status: 'active', statusLabel: 'Connected', badgeColor: '#10b981', catalogSource: 'live', catalogError: undefined, catalogFetchedAt: Date.now(),
              supportedModels: result.models.map(m => m.id),
              catalogModels: result.models.map((m, i) => ({ id: m.id, label: m.name || m.id, provider_type: 'local', is_default: i === 0, efforts: [], supports_fast_mode: false, context_window: m.context_window })),
            };
          } catch {
            updates = { status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: account.catalogModels?.length ? 'cached' : 'unavailable', catalogError: 'Ollama is unavailable. Start Ollama, then retry. The selected model has not changed.' };
          }
          if (!cancelled) setConnectedAccounts(previous => previous.map(a => a.id === account.id && a.endpointUrl === account.endpointUrl ? { ...a, ...updates } : a));
        }));
      } finally { probing = false; }
    };
    const timer = window.setInterval(() => { void check(); }, 15000);
    const onVisible = () => { if (!document.hidden) void check(); };
    window.addEventListener('focus', onVisible);
    document.addEventListener('visibilitychange', onVisible);
    return () => { cancelled = true; window.clearInterval(timer); window.removeEventListener('focus', onVisible); document.removeEventListener('visibilitychange', onVisible); };
  }, [localAccountsKey]);

  const handleSelectModel = useCallback((modelName: string, effort?: string, accountId?: string) => {
    setSelectedModelAccountId(accountId || '');
    setSelectedReasoningEffort(effort || '');
    setSelectedModel(modelName);
    try {
      localStorage.setItem('custos_selected_model_account', accountId || '');
      localStorage.setItem('custos_selected_reasoning_effort', effort || '');
      localStorage.setItem('custos_selected_model', modelName);
    } catch { /* Keep in-memory selection available without device storage. */ }
    if (activeSessionId) {
      try { localStorage.setItem(`custos.conversation_model.${activeSessionId}`, JSON.stringify({ model: modelName, accountId, effort })); } catch { /* Device storage is optional. */ }
      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSessionId) {
            return {
              ...s,
              model: modelName,
              modelAccountId: accountId,
              reasoningEffort: effort,
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });
    }
  }, [activeSessionId, currentProject, showToast]);

  useEffect(() => {
    if (!activeSession?.model || activeSession.nativeHarnessId) return;
    setSelectedModel(activeSession.model);
    setSelectedModelAccountId(activeSession.modelAccountId || '');
    setSelectedReasoningEffort(activeSession.reasoningEffort || '');
  }, [activeSessionId, activeSession?.model, activeSession?.modelAccountId, activeSession?.reasoningEffort]);

  useEffect(() => {
    const timer = window.setInterval(() => { if (!document.hidden) void refreshCatalogAvailability(); }, 5000);
    return () => window.clearInterval(timer);
  }, [refreshCatalogAvailability]);

  // Hydrate state from Custos Daemon on Mount
  useEffect(() => {
    applyUiScale(uiScale);

    async function hydrateFromDaemon() {
      try {
        setSavedWorkError(null);
        const isOnline = await daemonClient.checkHealth();
        if (!isOnline) {
          setSavedWorkError('Custos daemon is offline. Saved work could not be loaded.');
          return;
        }

        const [fetchedTasks, fetchedSessions] = await Promise.all([
          daemonClient.listTasks(),
          daemonClient.listSessions(),
        ]);

        // Hydrate provider models in background without blocking task and session rendering
        void refreshProviderState().catch((error) => {
          showToast(`Could not load provider settings: ${error instanceof Error ? error.message : String(error)}`);
        });

        const journals = await Promise.all(
          fetchedSessions.map(async (session) =>
            [session.id, await daemonClient.getSessionJournal(session.id)] as const
          )
        );
        const journalBySession = new Map(journals);

        setTasks(fetchedTasks);

        if (fetchedSessions.length > 0) {
          // Keep every canonical session, including multiple sessions bound to one Task.
          const projectedSessions: Session[] = fetchedSessions.map((domainSession) => {
            const attachedTaskId = domainSession.task_id || domainSession.attached_to
              || (typeof domainSession.mode === 'object' ? domainSession.mode.attached?.task_id : undefined);
            const t = fetchedTasks.find((task) => task.id === attachedTaskId);
            const taskId = t?.id;
            const sessionId = domainSession.id;
            let selection: { model?: string; accountId?: string; effort?: string } = {};
            try { const saved = JSON.parse(localStorage.getItem(`custos.conversation_model.${sessionId}`) || '{}'); if (saved && typeof saved.model === 'string') selection = { model: saved.model, accountId: typeof saved.accountId === 'string' ? saved.accountId : undefined, effort: typeof saved.effort === 'string' ? saved.effort : undefined }; } catch { /* Ignore invalid device preferences. */ }
            const rawJournal = journalBySession.get(domainSession.id) || domainSession.journal || [];
            const journalList: any[] = Array.isArray(rawJournal)
              ? rawJournal
              : Array.isArray((rawJournal as any)?.entries)
              ? (rawJournal as any).entries
              : Array.isArray((rawJournal as any)?.journal)
              ? (rawJournal as any).journal
              : [];

            return {
              id: sessionId,
              nativeHarnessId: t?.metadata?.kind === 'native_coding_agent' ? t.metadata.harness_id : undefined,
              workspaceId: t?.metadata?.workspace_id,
              openedLenses: Array.from(new Set(journalList
                .filter((entry) => entry.entry_type === 'workbench_opened')
                .map((entry) => entry.entry_data)
                .filter((lens): lens is 'chat' | 'code' | 'research' =>
                  lens === 'chat' || lens === 'code' || lens === 'research'))),
              taskId,
              taskTitle: t?.title,
              source: 'daemon',
              taskStatus: t ? String(t.status) : undefined,
              sessionId,
              pack: t?.metadata?.kind === 'native_coding_agent' ? 'engineering' : t?.contract?.pack_id || t?.contract?.pack || 'assistant',
              title: t?.title || domainSession.title || domainSession.current_goal || `Conversation ${sessionId.slice(0, 8)}`,
              time: 'Live',
              preview: t ? `Status: ${t.status} | Contract: ${t.contract?.pack_id || 'general'}` : 'Unbound conversation',
              model: t?.metadata?.kind === 'native_coding_agent' ? t.metadata.harness_id : selection.model ?? (t?.contract as any)?.model ?? selectedModel ?? '',
              modelAccountId: selection.accountId || selectedModelAccountId,
              reasoningEffort: selection.effort || selectedReasoningEffort,
              fileName: '',
              diffHunk: '',
              diffLinesCount: '',
              summary: t ? `Task ${t.id}: ${t.status}` : 'No Task bound',
              messages: journalList.filter((j) =>
                j.entry_type === 'user_message' || j.entry_type === 'assistant_message' ||
                j.entry_type === 'user' || j.entry_type === 'assistant' ||
                j.role === 'user' || j.role === 'assistant'
              ).map((j) => {
                const isUser =
                  j.role === 'user' ||
                  j.entry_type === 'user' ||
                  j.entry_type === 'user_message';
                return {
                  id: j.id || String(j.entry_id || `${sessionId}-${j.occurred_at}`),
                  recordedAt: j.occurred_at,
                  role: isUser ? 'user' : 'assistant',
                  author: isUser ? 'You' : (j.author || 'Custos Kernel'),
                  badge: j.badge,
                  stepName: j.step_name,
                  duration: j.duration,
                  text: j.content || j.entry_data || '',
                };
              }),
              diffCode: [],
            };
          });

          // A Task without a real Session remains a Task. Never invent a
          // conversation whose session ID is actually a Task ID.
          const allProjected = projectedSessions;

          setProjectData((prev) => ({
            ...prev,
            'Custos Workspace': allProjected,
          }));

          for (const chat of projectedSessions) {
            let runs;
            try {
              runs = await daemonClient.listSessionRuns(chat.sessionId || chat.id);
            } catch (error) {
              showToast(`Could not load run status: ${error instanceof Error ? error.message : String(error)}`);
              continue;
            }
            for (const run of runs) {
              if (run.journal_occurred_at) {
                setProjectData(previous => ({...previous, 'Custos Workspace': (previous['Custos Workspace'] || []).map(item => item.id !== chat.id ? item : {...item, messages: item.messages.map(message => message.recordedAt === run.journal_occurred_at ? {...message, runId: run.run_id, runStatus: run.status, badge: run.status, author: run.model || message.author} : message)})}));
              }
              followLiveRun('Custos Workspace', chat.id, chat.sessionId || chat.id, run.run_id, run.model || 'Custos');
            }
          }

          setCurrentProject('Custos Workspace');
          setActiveSessionId((previous) => allProjected.some((session) => session.id === previous)
            ? previous : (allProjected[0]?.id || ''));
        } else {
          setProjectData((previous) => ({ ...previous, 'Custos Workspace': [] }));
          setActiveSessionId('');
        }
      } catch (err) {
        console.warn('[DaemonClient] Could not load data from daemon:', err);
        setSavedWorkError(`Could not load saved work: ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    hydrateFromDaemon();
  }, [savedWorkAttempt]);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle settings (Ctrl+, or Cmd+,)
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        setIsSettingsOpen((prev) => !prev);
      }
      // Toggle Workspace Sidebar (Ctrl+B or Cmd+B)
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        setIsSidebarCollapsed((prev) => !prev);
      }
      // UI Zoom In (Ctrl++ or Ctrl+=)
      if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handleStepUiScale(10);
      }
      // UI Zoom Out (Ctrl+-)
      if ((e.metaKey || e.ctrlKey) && (e.key === '-' || e.key === '_')) {
        e.preventDefault();
        handleStepUiScale(-10);
      }
      // Reset UI Scale (Ctrl+0)
      if ((e.metaKey || e.ctrlKey) && e.key === '0') {
        e.preventDefault();
        handleSetUiScale(100);
      }
      // Open AI Setup (Ctrl+3 or Cmd+3)
      if ((e.metaKey || e.ctrlKey) && e.key === '3') {
        e.preventDefault();
        openAiSetup('local');
      }
      // Close open modals on Escape
      if (e.key === 'Escape') {
        setIsSettingsOpen(false);
        setIsAiSetupOpen(false);
        setIsNewSessionOpen(false);
        setIsAddProviderOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [uiScale]);

  // Actions
  const pendingTurns = useRef(new Set<string>());
  const liveConnections = useRef(new Map<string, AbortController>());
  const livePresentations = useRef(new Map<string, () => void>());
  const activeLiveView = useRef({ project: currentProject, conversation: activeSessionId });
  activeLiveView.current = { project: currentProject, conversation: activeSessionId };
  const streamReducedMotion = useRef(appearance.reduceMotion);
  streamReducedMotion.current = appearance.reduceMotion;
  useEffect(() => {
    const sync = () => livePresentations.current.forEach(refresh => refresh());
    sync();
    document.addEventListener('visibilitychange', sync);
    return () => document.removeEventListener('visibilitychange', sync);
  }, [currentProject, activeSessionId, appearance.reduceMotion]);
  useEffect(() => () => { liveConnections.current.forEach(controller => controller.abort()); liveConnections.current.clear(); }, []);
  const handleStopRun = async (runId: string) => {
    try { await daemonClient.cancelRun({ run_id: runId, reason: 'Stopped from chat' }); }
    catch (error) { showToast(error instanceof Error ? error.message : String(error)); }
  };
  const followLiveRun = (project: string, conversation: string, session: string, run: string, author: string) => {
    if (liveConnections.current.has(run)) return;
    const controller = new AbortController(); liveConnections.current.set(run, controller);
    let timer: number | undefined;
    let frame: number | undefined;
    let lastPaint = -Infinity;
    let latest: import('../types/domain').LiveRunSnapshot | null = null;
    let displayedText = '';
    let initialFlush = true;
    const systemMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
    const isVisible = () => !document.hidden && activeLiveView.current.project === project && activeLiveView.current.conversation === conversation;
    const cancelPending = () => {
      if (timer !== undefined) window.clearTimeout(timer);
      if (frame !== undefined) window.cancelAnimationFrame(frame);
      timer = undefined; frame = undefined;
    };
    const schedule = () => {
      if (controller.signal.aborted || frame !== undefined || timer !== undefined) return;
      if (isVisible()) frame = window.requestAnimationFrame(paint);
      else timer = window.setTimeout(() => { timer = undefined; flush(true); }, 200);
    };
    const paint = (timestamp: number) => {
      frame = undefined;
      if (controller.signal.aborted) return;
      if (!isVisible()) { flush(true); return; }
      if (timestamp - lastPaint >= 40) flush();
      else schedule();
    };
    const flush = (immediate = false) => {
      const snapshot = latest; if (!snapshot || controller.signal.aborted) return;
      const bypass = immediate || initialFlush || !isVisible() || streamReducedMotion.current || systemMotion.matches || !snapshot.text.startsWith(displayedText);
      const remaining = snapshot.text.length - displayedText.length;
      let end = bypass || remaining > 4096 ? snapshot.text.length : Math.min(snapshot.text.length, displayedText.length + Math.max(12, Math.ceil(remaining * 0.6)));
      // Avoid splitting a UTF-16 surrogate pair while revealing text.
      const trailing = snapshot.text.charCodeAt(end - 1);
      if (end < snapshot.text.length && trailing >= 0xD800 && trailing <= 0xDBFF) end += 1;
      displayedText = snapshot.text.slice(0, end);
      initialFlush = false;
      lastPaint = performance.now();
      const text = displayedText;
      setProjectData(previous => ({ ...previous, [project]: (previous[project] || []).map(chat => chat.id !== conversation ? chat : {
        ...chat, messages: chat.messages.some(message => message.runId === run)
          ? chat.messages.map(message => message.runId !== run ? message : { ...message, text, runStatus: snapshot.status, badge: snapshot.reconnecting ? 'Reconnecting' : snapshot.status, activities: snapshot.activities, liveError: snapshot.error || undefined })
          : [...chat.messages, { id: `live-${run}`, role: 'assistant' as const, author, text, runId: run, runStatus: snapshot.status, badge: snapshot.reconnecting ? 'Reconnecting' : snapshot.status, activities: snapshot.activities, liveError: snapshot.error || undefined }],
      }) }));
      if (displayedText !== snapshot.text && !immediate) schedule();
    };
    const refresh = () => { cancelPending(); if (latest) flush(true); };
    livePresentations.current.set(run, refresh);
    systemMotion.addEventListener('change', refresh);
    controller.signal.addEventListener('abort', cancelPending, { once: true });
    void daemonClient.watchRun(session, run, snapshot => {
      latest = snapshot;
      if (['completed', 'failed', 'cancelled', 'interrupted'].includes(snapshot.status)) { cancelPending(); flush(true); if (snapshot.status === 'failed') void refreshCatalogAvailability(); }
      else schedule();
    }, controller.signal).finally(() => {
      cancelPending();
      if (!controller.signal.aborted) flush(true);
      systemMotion.removeEventListener('change', refresh);
      livePresentations.current.delete(run);
      liveConnections.current.delete(run);
    });
  };

  const startNativeConversation = async (instruction: string, harness: string, workspace: string) => {
    const session = await daemonClient.createSession('assisted');
    const task = await daemonClient.createTask({ title: instruction.slice(0, 120), metadata: { kind: 'native_coding_agent', pack_id: 'engineering', harness_id: harness, workspace_id: workspace, conversation_session_id: session.id } });
    await daemonClient.attachSession({ session_id: session.id, task_id: task.id });
    await daemonClient.appendSessionMessage(session.id, 'user', instruction);
    const project = 'Custos Workspace';
    const chat: Session = { nativeHarnessId: harness, workspaceId: workspace, id: session.id, sessionId: session.id, taskId: task.id, source: 'daemon', title: instruction.slice(0, 80), time: 'Live', preview: harness, model: harness, pack: 'engineering', fileName: '', diffHunk: '', diffLinesCount: '', summary: '', diffCode: [], messages: [{ role: 'user', author: 'You', text: instruction }] };
    setProjectData(previous => ({ ...previous, [project]: [...(previous[project] || []), chat] }));
    const run = await daemonClient.startRun({ background: true, task_id: task.id, session_id: session.id, turn_id: `turn_${crypto.randomUUID()}`, preferred_mode: 'native', harness_id: harness, workspace_id: workspace, prompt: instruction });
    setCurrentProject(project); setActiveSessionId(session.id);
    followLiveRun(project, session.id, session.id, run.run_id, harness);
    return { run, sessionId: session.id };
  };

  const handleSendMessage = async (text: string, lens?: 'assistant' | 'coding' | 'research') => {
    if (!activeSession) return;
    if (activeSession.messages.some(message => message.runId && !['completed','failed','cancelled','interrupted'].includes(message.runStatus || ''))) { showToast('Stop or wait for the current run before sending another prompt.'); return; }
    if (pendingTurns.current.has(activeSession.id)) return;
    if (!activeSession.nativeHarnessId && !selectedModelAvailable) { showToast('Select an available model/account and reasoning effort before sending.'); return; }
    pendingTurns.current.add(activeSession.id);
    let userJournalPersisted = false;
    let dispatchAttempted = false;
    const userMsg = {
      id: `pending-${crypto.randomUUID()}`,
      role: 'user' as const,
      author: 'You',
      text,
    };

    // Keep the local draft visible while the request is pending.
    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return {
            ...s,
            messages: [...s.messages, userMsg],
          };
        }
        return s;
      });
      return {
        ...prev,
        [currentProject]: updatedList,
      };
    });

    const reportTurnFailure = (failure: {
      role: 'assistant';
      author: string;
      badge?: string;
      stepName?: string;
      text: string;
    }) => {
      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              messages: s.messages.map((message) => message.id === userMsg.id
                ? { ...message, badge: dispatchAttempted ? 'Run status unknown' : userJournalPersisted ? 'Saved, not run' : 'Not sent', liveError: failure.text }
                : message),
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });
    };

    try {
      if (activeSession.nativeHarnessId && activeSession.workspaceId && activeSession.sessionId) {
        const harness = activeSession.nativeHarnessId;
        const taskId = activeSession.taskId;
        if (!taskId) {
          throw new Error('This coding conversation is not bound to a Task. Reopen it from Tasks before continuing.');
        }
        await daemonClient.appendSessionMessage(activeSession.sessionId, 'user', text);
        userJournalPersisted = true;
        dispatchAttempted = true;
        const run = await daemonClient.startRun({ background: true, task_id: taskId, session_id: activeSession.sessionId, turn_id: `turn_${crypto.randomUUID()}`, preferred_mode: 'native', harness_id: harness, workspace_id: activeSession.workspaceId, prompt: text });
        setProjectData(previous => ({ ...previous, [currentProject]: (previous[currentProject] || []).map(chat => chat.id !== activeSession.id ? chat : { ...chat, messages: [...chat.messages, { id: `live-${run.run_id}`, role: 'assistant' as const, author: harness, runId: run.run_id, runStatus: 'queued', badge: 'queued', text: '' }] }) }));
        followLiveRun(currentProject, activeSession.id, activeSession.sessionId, run.run_id, harness);
        return;
      }
      const chosenModel = (selectedModel || activeSession.model || '').trim();
      const selectedCombo = combos.find((combo) => combo.id.toLowerCase() === chosenModel.toLowerCase());

      if (!chosenModel) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'AI Local Unavailable',
          stepName: 'AI Local',
          text: 'No local model is available yet. Start Ollama and install at least one model, then use Retry models. You can chat without signing in as soon as AI Local discovers it.',
        });
        return;
      }

      if (selectedCombo) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Concrete Model Required',
          stepName: 'Model Gateway',
          text: 'Routing combos are not dispatched as a single model turn yet. Select one concrete live model from a connected account.',
        });
        return;
      }

      const isNativeModel = chosenModel.startsWith('native:');
      let routedAccountId: string | undefined;
      let modelAccount: ConnectedAccount | undefined;
      let effort: string | undefined;

      if (isNativeModel) {
        const modelPath = chosenModel.slice('native:'.length);
        try {
          if (!modelPath) throw new Error('The selected local model has no file path');
          const loaded = await daemonClient.listLoadedInstances();
          if (!loaded.instances.some(instance => instance.model_path === modelPath)) {
            await daemonClient.loadLocalModel(modelPath);
          }
        } catch (error: any) {
          reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Local Model Unavailable', text: error?.message || String(error) });
          return;
        }
      } else {
        const candidates = connectedAccounts.filter(account => account.status === 'active' && account.supportedModels.includes(chosenModel) &&
          (!selectedReasoningEffort || account.catalogModels?.some(model => model.id === chosenModel && model.efforts.includes(selectedReasoningEffort))));
        if (!candidates.length) { showToast('Selected model is unavailable. Refresh models or choose another model.'); return; }
        const transientAccount = candidates.find(isTransientAiLocalAccount);
        const reservation = transientAccount ? undefined : await daemonClient.reserveProviderAccount(candidates[0].providerId, chosenModel, selectedReasoningEffort || undefined);
        modelAccount = transientAccount || candidates.find(account => account.id === reservation?.account_id);
        if (!modelAccount) { showToast('No available account supports this model and reasoning effort. Refresh models.'); return; }
        routedAccountId = isTransientAiLocalAccount(modelAccount) ? undefined : modelAccount.id;
        const metadata = modelAccount.catalogModels?.find(model => model.id === chosenModel);
        effort = selectedReasoningEffort || metadata?.default_effort;
        if (effort && !metadata?.efforts.includes(effort)) { showToast('Reasoning effort is no longer available. Select the model again.'); return; }
      }

      if (chosenModel.startsWith('ollama/')) {
        const account = connectedAccounts.find(a => a.id === routedAccountId && a.status === 'active');
        if (!account) {
          reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Ollama Unavailable', stepName: 'Model Gateway', text: 'Ollama is offline or the selected model is no longer installed. Start Ollama and use Retry models. Your prompt will not be sent to another provider.' });
          return;
        }
        // Recheck before dispatch to catch shutdowns between periodic probes.
        try {
          const probe = await daemonClient.probeModels(account.endpointUrl || 'http://localhost:11434', 'ollama');
          if (!probe.models.some(model => model.id === chosenModel)) {
            reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Model Removed', text: 'The selected Ollama model is no longer installed. Refresh models and choose an installed model.' });
            return;
          }
        } catch {
          setConnectedAccounts(previous => previous.map(a => a.id === account.id ? { ...a, status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: 'cached', catalogError: 'Ollama is unavailable. Start Ollama and retry models.' } : a));
          reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Ollama Offline', text: 'Cannot connect to Ollama. Start Ollama and retry models, then resend your prompt. No other provider was used.' });
          return;
        }
      }

      const selectedAccount = modelAccount;
      const hasActiveAccount = Boolean(selectedAccount);
      if (!isNativeModel && !hasActiveAccount) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Model Not Connected',
          stepName: 'Model Gateway',
          text: `Model "${chosenModel}" cannot be dispatched because no active provider accounts are connected. Please connect a provider in Settings > Providers first.`,
        });
        return;
      }

      // 1. Check if an LLM is currently available / configured
      const llmStatus = await daemonClient.checkLlmStatus();
      if (!llmStatus.configured) {
        const noLlmMsg = {
          role: 'assistant' as const,
          author: 'Custos Engine',
          badge: 'No LLM Available',
          stepName: 'LLM Gateway',
          text: `⚠️ Chưa có LLM nào được cấu hình hiện tại (No LLMs currently available).\n\n${llmStatus.message || 'Hệ thống đang hoạt động ở chế độ Sovereign & Offline Kernel. Vui lòng thêm API Key (Anthropic Claude, OpenAI Codex, Google Gemini...) hoặc chọn Local Model trong Cài đặt > Providers để kích hoạt AI reasoning.'}`,
        };

        reportTurnFailure(noLlmMsg);
        return;
      }

      // 2. Ensure a daemon session exists, but run each chat turn on its own task
      // so active worker claims on the source task do not fence conversation turns.
      let targetSessionId = activeSession.sessionId;
      let runTaskId = '';
      let createTaskError: unknown = null;

      if (!targetSessionId) {
        try {
          const createdSession = await daemonClient.createSession('assisted');
          targetSessionId = createdSession.id;
        } catch (e) {
          console.warn('[DaemonClient] Could not create session on daemon:', e);
        }
      }

      try {
        const rawTitle = (activeSession.title || chosenModel).replace(/^(Chat turn:\s*)+/, '').trim();
        const turnTitle = rawTitle ? `Chat turn: ${rawTitle}` : `Chat turn: ${chosenModel}`;
        const createdTask = await daemonClient.createTask({
          title: turnTitle,
          metadata: {
            kind: 'conversation_chat_turn',
            pack_id: lens || activeSession.pack || 'assistant',
            conversation_session_id: targetSessionId || activeSession.id,
            source_task_id: activeSession.taskId || activeSession.id,
            model: chosenModel,
            ...(!isNativeModel && !isTransientAiLocalAccount(selectedAccount)
              ? { provider_account_id: selectedAccount?.id }
              : {}),
          },
        });
        runTaskId = createdTask.id;
      } catch (e) {
        createTaskError = e;
        console.warn('[DaemonClient] Could not create chat turn task on daemon:', e);
      }

      if (!runTaskId) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Kernel',
          badge: 'Task Required',
          stepName: 'Conversation Dispatch',
          text: `Could not create an isolated chat task for this turn: ${
            createTaskError instanceof Error ? createTaskError.message : String(createTaskError || 'unknown error')
          }`,
        });
        return;
      }

      // Append message to backend session journal
      if (targetSessionId) {
        await daemonClient.appendSessionMessage(targetSessionId, 'user', text);
        userJournalPersisted = true;
      }

      // Register a cancellable background run before consuming its live projection.
      dispatchAttempted = true;
      const run = await daemonClient.startRun({
        background: true,
        turn_id: `turn_${crypto.randomUUID()}`,
        task_id: runTaskId,
        preferred_mode: 'model',
        session_id: targetSessionId,
        prompt: text,
        model: chosenModel,
        provider_account_id: routedAccountId,
        reasoning_effort: effort,
      });

      const runId = run.run_id;
      setProjectData(previous => ({ ...previous, [currentProject]: (previous[currentProject] || []).map(chat => chat.id !== activeSession.id ? chat : { ...chat, messages: [
        ...chat.messages.map(message => message.id === userMsg.id ? { ...message, badge: undefined, liveError: undefined } : message),
        { id: `live-${runId}`, role: 'assistant' as const, author: chosenModel, badge: 'queued', stepName: `Run #${runId.slice(0, 8)}`, text: '', runId, runStatus: 'queued' },
      ] }) }));
      if (targetSessionId) followLiveRun(currentProject, activeSession.id, targetSessionId, runId, chosenModel);
    } catch (err: any) {
      void refreshCatalogAvailability();
      if ((selectedModel || activeSession.model || '').startsWith('ollama/') && String(err?.message || err).includes('Ollama connection failed')) {
        setConnectedAccounts(previous => previous.map(a => a.providerId === 'local' ? { ...a, status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: 'cached', catalogError: 'Ollama stopped during the request. Start Ollama and resend your prompt.' } : a));
      }
      reportTurnFailure({ role: 'assistant', author: 'Custos', text: `${dispatchAttempted ? 'Could not confirm run start; check Activity before retrying' : 'Could not prepare the run'}: ${err?.message || err}` });
    } finally { pendingTurns.current.delete(activeSession.id); }
  };

  const handleClearHistory = () => {
    if (!activeSession) return;
    if (activeSession.source === 'daemon') {
      showToast('Clearing saved conversation history is not available yet.');
      return;
    }
    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return { ...s, messages: [] };
        }
        return s;
      });
      return { ...prev, [currentProject]: updatedList };
    });
    showToast('Conversation cleared in this window');
  };

  const handleDeleteConversation = useCallback(async (sessionId: string) => {
    const list = projectData[currentProject] || [];
    const targetIndex = list.findIndex((session) => session.id === sessionId);
    if (targetIndex < 0) {
      showToast('Conversation not found');
      return;
    }

    const target = list[targetIndex];
    const nextList = list.filter((session) => session.id !== sessionId);
    const fallbackSession = nextList[targetIndex] || nextList[targetIndex - 1] || nextList[0];

    const daemonSessionId = target.sessionId || (target.source === 'daemon' ? target.id : undefined);
    if (daemonSessionId) {
      try {
        await daemonClient.deleteSession(daemonSessionId);
      } catch (error) {
        showToast(`Could not delete session: ${error instanceof Error ? error.message : String(error)}`);
        return;
      }
    }

    setProjectData((prev) => ({
      ...prev,
      [currentProject]: nextList,
    }));

    if (activeSessionId === sessionId) {
      setActiveSessionId(fallbackSession?.id || '');
    }

    showToast(
      target.source === 'daemon'
        ? 'Session deleted from daemon'
        : 'Conversation deleted'
    );
  }, [activeSessionId, currentProject, projectData, showToast]);

  const handleAcceptAndRun = async () => {
    if (!activeSession) return;
    try {
      await daemonClient.advanceTask(activeSession.taskId || activeSession.id, 'queued');
      showToast('Task queued. The daemon will report the run state when it starts.');
    } catch (error) {
      showToast(`Could not advance task: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleRejectDiff = () => {
    showToast('Reject diff is unavailable until a daemon-backed patch proposal is selected.');
  };

  const handleCopyDiff = async () => {
    if (!activeSession) return;
    const diffText = activeSession.diffCode.map((d) => d.text).join('\n');
    if (!diffText) { showToast('No diff is available to copy.'); return; }
    try {
      await navigator.clipboard.writeText(diffText);
      showToast('Code diff copied to clipboard');
    } catch (error) {
      showToast(`Could not copy diff: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleCreateNewSession = async (title: string, pack: 'engineering' | 'research' | 'assistant') => {
    try {
      // 1. Create real Task in Custos Daemon
      const createdTask = await daemonClient.createTask({
        title: title || 'New Supervised Task',
        contract: {
          pack_id: pack,
          name: title || 'New Supervised Task',
          description: `${pack} conversation task`,
          required_capabilities: [],
          evidence_requirements: [],
        },
      });

      // 2. Create Session in Custos Daemon
      const createdSession = await daemonClient.createSession('assisted');
      await daemonClient.attachSession({ session_id: createdSession.id, task_id: createdTask.id });
      await daemonClient.openSessionInLens(createdSession.id, pack === 'engineering' ? 'coding' : pack === 'assistant' ? 'assistant' : 'research');

      const newSessionItem: Session = {
        id: createdSession.id,
        taskId: createdTask.id,
        source: 'daemon',
        taskStatus: createdTask.status,
        sessionId: createdSession.id,
        pack,
        openedLenses: [pack === 'engineering' ? 'code' : pack === 'assistant' ? 'chat' : 'research'],
        title: createdTask.title,
        time: 'Just now',
        preview: `${pack} task · ${createdTask.status}`,
        model: selectedModel || '',
        modelAccountId: selectedModelAccountId,
        reasoningEffort: selectedReasoningEffort,
        fileName: '',
        diffHunk: '',
        diffLinesCount: '',
        summary: `Created task ${createdTask.id}`,
        messages: [],
        diffCode: [],
      };

      setProjectData((prev) => {
        const currentList = prev[currentProject] || [];
        return {
          ...prev,
          [currentProject]: [newSessionItem, ...currentList],
        };
      });

      setActiveSessionId(createdSession.id);
      setTasks((previous) => [createdTask, ...previous]);
      setIsNewSessionOpen(false);
      showToast(`Created Task ${createdTask.id.slice(0, 8)}`);

    } catch (err: any) {
      console.error('[CreateSession] Error:', err);
      showToast(`Error creating session: ${err?.message || err}`);
    }
  };

  const handleCreateChatSession = async (pack: 'engineering' | 'research' | 'assistant') => {
    try {
      const createdSession = await daemonClient.createSession('assisted');
      await daemonClient.openSessionInLens(createdSession.id, pack === 'engineering' ? 'coding' : pack === 'assistant' ? 'assistant' : 'research');
      const conversation: Session = {
        id: createdSession.id,
        sessionId: createdSession.id,
        source: 'daemon',
        pack,
        openedLenses: [pack === 'engineering' ? 'code' : pack === 'assistant' ? 'chat' : 'research'],
        title: `Conversation ${createdSession.id.slice(0, 8)}`,
        time: 'Just now',
        preview: 'Unbound conversation',
        model: selectedModel || '',
        modelAccountId: selectedModelAccountId,
        reasoningEffort: selectedReasoningEffort,
        fileName: '',
        diffHunk: '',
        diffLinesCount: '',
        summary: '',
        messages: [],
        diffCode: [],
      };
      setProjectData((previous) => ({
        ...previous,
        [currentProject]: [conversation, ...(previous[currentProject] || [])],
      }));
      setActiveSessionId(createdSession.id);
      setIsNewSessionOpen(false);
    } catch (error) {
      showToast(`Could not create chat: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleProjectOpened = (project: { name: string; path: string; isGit: boolean; branch?: string }) => {
    setProjectData((prev) => {
      if (prev[project.name]) return prev;
      return {
        ...prev,
        [project.name]: [],
      };
    });
    setCurrentProject(project.name);
    try {
      localStorage.setItem('custos_current_project', project.name);
      localStorage.setItem('custos_current_project_path', project.path);
    } catch {}
    if (project.path) {
      daemonClient.detectWorkspace(project.path).then((det) => {
        setActiveDetection(det);
      }).catch(() => {});
    }
  };

  const handleOpenFolderDirect = async () => {
    try {
      const chosen = await daemonClient.pickFolder();
      if (!chosen) return;
      const detection = await daemonClient.detectWorkspace(chosen);
      setActiveDetection(detection);
      const name = detection.name || chosen.split('/').filter(Boolean).pop() || 'Project';
      const isGit = detection.is_git;
      const branch = detection.branch || 'main';

      await daemonClient.createWorkspace({
        name,
        path: detection.root_path || chosen,
        kind: {
          type: 'folder',
          path: detection.root_path || chosen,
        },
      });

      handleProjectOpened({
        name,
        path: detection.root_path || chosen,
        isGit,
        branch: isGit ? branch : undefined,
      });

      showToast(`Opened project: ${name} ${isGit ? `(${branch})` : ''}`);
    } catch (err: any) {
      console.warn('Direct pick folder fallback to dialog:', err);
      setIsOpenProjectOpen(true);
    }
  };

  const handleNewProjectPrompt = () => {
    handleOpenFolderDirect();
  };

  const handleSaveProvider = async (
    serviceOrParams: string | { service: string; accountName?: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
    maybeApiKey?: string
  ) => {
    try {
      const isObj = typeof serviceOrParams === 'object' && serviceOrParams !== null;
      const service = isObj ? serviceOrParams.service : serviceOrParams;
      const apiKey = isObj ? (serviceOrParams.apiKey || '') : (maybeApiKey || '');
      const endpointUrl = isObj ? serviceOrParams.endpointUrl : undefined;
      const defaultModel = isObj ? serviceOrParams.model : undefined;
      const contextWindow = isObj ? serviceOrParams.contextWindow : undefined;
      const fastMode = isObj ? serviceOrParams.fastMode : undefined;

      const providerId = `${normalizeProviderServiceId(service)}-${crypto.randomUUID()}`;
      const providerType =
        service.toLowerCase().includes('claude') || service.toLowerCase().includes('anthropic') ? 'anthropic' :
        service.toLowerCase().includes('openai') || service.toLowerCase().includes('gpt') ? 'openai' :
        service.toLowerCase().includes('gemini') ? 'gemini' :
        service.toLowerCase().includes('deepseek') ? 'deepseek' : 'local';
      const normalizedService = normalizeProviderServiceId(providerType);
      const resolvedEndpoint = endpointUrl?.trim() || undefined;
      const configured = Boolean(apiKey.trim() || resolvedEndpoint || normalizedService === 'local');

      await daemonClient.saveProvider({
        id: providerId,
        name: (isObj && serviceOrParams.accountName?.trim()) || `${service} Account ${providerId.slice(-6)}`,
        service_type: providerType,
        provider_type: providerType,
        model: defaultModel || '',
        default_model: defaultModel,
        api_key: apiKey,
        endpoint_url: resolvedEndpoint,
        context_window: contextWindow,
        fast_mode: fastMode,
        status: configured ? 'primary' : 'offline',
        status_label: configured ? 'Configured & Active' : 'Offline',
        latency_ms: 28,
        is_active: configured,
      });

      await refreshProviderState();
      setIsAddProviderOpen(false);
      showToast(`Provider ${service} đã được lưu với credential bảo mật!`);
    } catch (err: any) {
      console.error('[SaveProvider] Error:', err);
      showToast(`Lỗi lưu provider: ${err?.message || err}`);
      throw err;
    }
  };

  const handleDeleteProvider = async (id: string) => {
    try {
      await daemonClient.deleteProvider(id);
      await refreshProviderState();
      showToast(`Đã xóa provider ${id} khỏi hệ thống!`);
    } catch (err: any) {
      console.error('[DeleteProvider] Error:', err);
      showToast(`Lỗi xóa provider: ${err?.message || err}`);
    }
  };

  const handleTestConnection = async (providerNameOrId: string) => {
    try {
      const provider = providers.find((p) => p.id === providerNameOrId || p.name === providerNameOrId);
      if (!provider) {
        showToast(`Không tìm thấy provider ${providerNameOrId}`);
        return;
      }
      const probeRes = await daemonClient.probeModels(provider.endpoint || '', undefined, undefined, provider.id);
      await refreshProviderState();
      showToast(`✓ Probe thành công ${provider.name}; daemon tìm thấy ${probeRes.count} models.`);
    } catch (err: any) {
      showToast(`Lỗi kết nối provider: ${err?.message || err}`);
    }
  };

  const handleGenerateClientKey = async () => {
    try {
      const name = window.prompt('Nhập tên Client Token mới (vd: CLI Integration, Microservice):', 'Custom API Key');
      if (!name) return;
      await daemonClient.generateClientKey(name);
      const updated = await daemonClient.listClientKeys();
      if (updated) {
        setClientKeys(updated.map(mapBackendKey));
      }
      showToast('Client Key mới đã được tạo và lưu vào SQLite DB!');
    } catch (err: any) {
      console.error('[GenerateKey] Error:', err);
      showToast(`Lỗi tạo key: ${err?.message || err}`);
    }
  };

  const handleRevokeClientKey = async (id: string) => {
    try {
      await daemonClient.revokeClientKey(id);
      const updated = await daemonClient.listClientKeys();
      if (updated) {
        setClientKeys(updated.map(mapBackendKey));
      }
      showToast(`Đã thu hồi Client Key ${id.slice(0, 8)}`);
    } catch (err: any) {
      console.error('[RevokeKey] Error:', err);
      showToast(`Lỗi thu hồi key: ${err?.message || err}`);
    }
  };

  // Combos & Usage Handlers (OmniRoute / 9Router style)
  const handleCreateCombo = useCallback((comboData: Omit<ModelCombo, 'id' | 'createdAt'>) => {
    const newCombo: ModelCombo = {
      ...comboData,
      id: `combo-${Date.now()}`,
      createdAt: new Date().toISOString(),
    };
    setCombos((prev) => [newCombo, ...prev]);
    showToast(`Routing Combo "${newCombo.name}" created!`);
  }, [showToast]);

  const handleUpdateCombo = useCallback((id: string, updates: Partial<ModelCombo>) => {
    setCombos((prev) =>
      prev.map((c) => (c.id === id ? { ...c, ...updates, updatedAt: new Date().toISOString() } : c))
    );
    showToast('Routing Combo updated');
  }, [showToast]);

  const handleDeleteCombo = useCallback((id: string) => {
    setCombos((prev) => prev.filter((c) => c.id !== id));
    if (selectedModel === id) {
      setSelectedModel('');
      localStorage.setItem('custos_selected_model', '');
    }
    showToast('Routing Combo deleted');
  }, [selectedModel, showToast]);

  const ensureOAuthProviderRecord = useCallback(async (
    providerId: string,
    accountName?: string
  ) => {
    const token = await daemonClient.getOAuthToken(providerId);
    if (!token?.connected) throw new Error('OAuth account credentials were not saved. Retry the connection.');

    const providerRecords = (await daemonClient.listProviders()) as ProviderConfigRecord[];
    const existing = providerRecords.find((provider) => provider.id === providerId);
    const serviceId = normalizeProviderServiceId(existing?.service_type || providerId);

    if (existing && isProviderConnected(existing)) {
      return;
    }

    await daemonClient.saveProvider({
      id: existing?.id || providerId,
      name: existing?.name || accountName || `${PROVIDER_LABELS[serviceId]} (OAuth)`,
      service_type: serviceId,
      provider_type: serviceId,
      api_key_masked: existing?.api_key_masked && existing.api_key_masked !== 'none' ? existing.api_key_masked : 'oauth',
      endpoint_url: existing?.endpoint_url || defaultProbeBaseUrl(serviceId),
      status: 'active',
      status_label: 'OAuth Active',
      fast_mode: existing?.fast_mode ?? false,
      is_active: true,
    });
  }, []);

  const handleConnectOAuth = useCallback(async (providerId: string, accountName: string, _email?: string) => {
    await ensureOAuthProviderRecord(providerId, accountName);
    const refreshed = await refreshProviderState();
    const account = refreshed.accounts.find((item) => item.id === providerId);
    showToast(`Connected ${account?.accountName || accountName || PROVIDER_LABELS[normalizeProviderServiceId(providerId)]} via OAuth`);
  }, [ensureOAuthProviderRecord, refreshProviderState, showToast]);

  const handleDeleteAccount = useCallback(async (accountId: string) => {
    const account = connectedAccounts.find((item) => item.id === accountId);
    if (!account) return;
    try {
      await daemonClient.deleteOAuthToken(account.id);
      await daemonClient.deleteProvider(account.id);
      await refreshProviderState();
      showToast('Account disconnected');
    } catch (err) {
      showToast(`Could not disconnect account: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, [connectedAccounts, refreshProviderState, showToast]);

  return (
    <AppContext.Provider
      value={{
        projectData,
        setProjectData,
        currentProject,
        setCurrentProject,
        activeSessionId,
        setActiveSessionId,
        activeSession,
        currentSessions,
        tasks,
        refreshTasks,
        createTask: handleCreateTask,
        advanceTask: handleAdvanceTask,
        completeTask: handleCompleteTask,
        cancelTask: handleCancelTask,
        workspaces,
        activeWorkspaceId,
        setActiveWorkspaceId,
        activeWorkspace,
        refreshWorkspaces,
        currentTab,
        setCurrentTab,
        activeWorkbench,
        setActiveWorkbench,
        isSessionsCollapsed,
        setIsSessionsCollapsed,
        isRightExplorerOpen,
        setIsRightExplorerOpen,
        toggleRightExplorer,
        providers,
        setProviders,
        clientKeys,
        setClientKeys,
        combos,
        setCombos,
        handleCreateCombo,
        handleUpdateCombo,
        handleDeleteCombo,
        connectedAccounts,
        refreshModels: async () => { await refreshProviderState(); },
        setConnectedAccounts,
        handleConnectOAuth,
        handleDeleteAccount,
        isSettingsOpen,
        setIsSettingsOpen,
        settingsTab,
        setSettingsTab,
        openSettings,
        isAiSetupOpen,
        setIsAiSetupOpen,
        aiSetupTab,
        setAiSetupTab,
        openAiSetup,
        isNewSessionOpen,
        setIsNewSessionOpen,
        isOpenProjectOpen,
        setIsOpenProjectOpen,
        handleProjectOpened,
        handleOpenFolderDirect,
        activeDetection,
        refreshWorkspaceDetection,
        isAddProviderOpen,
        setIsAddProviderOpen,
        isPluginsOpen,
        setIsPluginsOpen,
        openPlugins,
        isSkillsOpen,
        setIsSkillsOpen,
        openSkills,
        isHelpOpen,
        setIsHelpOpen,
        openHelp,
        isGitHubAuthOpen,
        setIsGitHubAuthOpen,
        openGitHubAuth,
        handleGitHubLogin,
        handleGitHubLogout,
        userProfile,
        setUserProfile,
        isSidebarCollapsed,
        setIsSidebarCollapsed,
        toggleSidebar,
        isWebTabOpen,
        setIsWebTabOpen,
        toggleWebTab,
        isWebTabExpanded,
        setIsWebTabExpanded,
        toggleExpandWebTab,
        isResourcePickerOpen,
        setIsResourcePickerOpen,
        resourceTabs,
        handleCloseResourceTab,
        splitPercent,
        setSplitPercent,
        splitTabId,
        setSplitTabId,
        splitRatio,
        setSplitRatio,
        toggleSplitTab,
        closeSplitTab,
        column2Width,
        setColumn2Width,
        isResizingColumn,
        handleStartResizeColumn2,
        handleResetColumn2Width,
        appearance,
        resolvedTheme,
        updateAppearance,
        uiScale,
        handleSetUiScale,
        handleStepUiScale,
        toastMessage,
        showToast,
        savedWorkError,
        retrySavedWork,
        selectedModel,
        selectedModelAvailable,
        selectedModelAccountId,
        selectedReasoningEffort,
        handleSelectModel,
        handleSendMessage,
        handleStopRun,
        startNativeConversation,
        handleClearHistory,
        handleDeleteConversation,
        handleAcceptAndRun,
        handleRejectDiff,
        handleCopyDiff,
        handleCreateNewSession,
        handleCreateChatSession,
        handleNewProjectPrompt,
        handleSaveProvider,
        handleDeleteProvider,
        handleTestConnection,
        handleGenerateClientKey,
        handleRevokeClientKey,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};

export const useAppContext = () => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useAppContext must be used within an AppProvider');
  }
  return context;
};
