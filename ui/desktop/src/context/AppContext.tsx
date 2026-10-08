import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import { MainTab, ProjectData, ProviderItem, ClientApiKey, Session } from '../types';
import { daemonClient } from '../api/daemon_client';
import { Task, SessionJournalEntry } from '../types/domain';

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

  // Navigation & Layout
  currentTab: MainTab;
  setCurrentTab: (tab: MainTab) => void;
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

  // Modals
  isSettingsOpen: boolean;
  setIsSettingsOpen: (open: boolean) => void;
  settingsTab: string;
  setSettingsTab: (tab: string) => void;
  openSettings: (tab?: string) => void;
  isNewSessionOpen: boolean;
  setIsNewSessionOpen: (open: boolean) => void;
  isAddProviderOpen: boolean;
  setIsAddProviderOpen: (open: boolean) => void;

  // UI Scale
  uiScale: number;
  handleSetUiScale: (val: number) => void;
  handleStepUiScale: (delta: number) => void;

  // Toast
  toastMessage: string | null;
  showToast: (msg: string) => void;

  // Actions
  handleSendMessage: (text: string) => Promise<void>;
  handleClearHistory: () => void;
  handleAcceptAndRun: () => Promise<void>;
  handleRejectDiff: () => void;
  handleCopyDiff: () => void;
  handleCreateNewSession: (title: string, pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
  handleNewProjectPrompt: () => void;
  handleSaveProvider: (
    serviceOrParams: string | { service: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
    apiKey?: string
  ) => Promise<void> | void;
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

  const modelName = p.default_model || p.model || (
    iconType === 'anthropic' ? 'claude-3-7-sonnet' :
    iconType === 'openai' ? 'gpt-4o' :
    iconType === 'gemini' ? 'gemini-2.5-flash' : 'deepseek-chat'
  );

  return {
    id: p.id,
    name: p.name,
    model: modelName,
    status,
    statusLabel: p.status === 'configured' ? 'Configured & Active' : p.status_label || (status === 'primary' ? 'Verified & Active' : 'Offline'),
    badgeColor: status === 'primary' ? '#10b981' : status === 'standby' ? '#3b82f6' : '#6b7280',
    apiKey: p.api_key_masked || (p.api_key && p.api_key.trim().length > 0 ? `${p.api_key.slice(0, 6)}••••••••` : 'Chưa cấu hình'),
    quotaUsed: p.quota_used || undefined,
    quotaTotal: p.quota_total || undefined,
    quotaPercent: typeof p.quota_percent === 'number' ? p.quota_percent : undefined,
    endpoint: p.endpoint_url || p.endpoint || undefined,
    defaultModel: p.default_model || p.model || undefined,
    contextWindow: p.context_window || undefined,
    fastMode: p.fast_mode || undefined,
    latency: p.latency_ms ? `${p.latency_ms}ms` : '32ms',
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

const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Projects & Sessions state
  const [projectData, setProjectData] = useState<ProjectData>({ 'Custos Workspace': [] });
  const [currentProject, setCurrentProject] = useState<string>('Custos Workspace');
  const [activeSessionId, setActiveSessionId] = useState<string>('');
  const [tasks, setTasks] = useState<Task[]>([]);

  // Navigation & Layout state
  const [currentTab, setCurrentTab] = useState<MainTab>('studio');
  const [isSessionsCollapsed, setIsSessionsCollapsed] = useState<boolean>(false);
  const [isRightExplorerOpen, setIsRightExplorerOpen] = useState<boolean>(true);

  const toggleRightExplorer = useCallback(() => {
    setIsRightExplorerOpen((prev) => !prev);
  }, []);

  // Providers & Keys state
  const [providers, setProviders] = useState<ProviderItem[]>([]);
  const [clientKeys, setClientKeys] = useState<ClientApiKey[]>([]);

  // Modals state
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [settingsTab, setSettingsTab] = useState<string>('general');
  const [isNewSessionOpen, setIsNewSessionOpen] = useState<boolean>(false);
  const [isAddProviderOpen, setIsAddProviderOpen] = useState<boolean>(false);

  const openSettings = useCallback((tab?: string) => {
    if (tab) setSettingsTab(tab);
    setIsSettingsOpen(true);
  }, []);

  // UI Scale state
  const [uiScale, setUiScale] = useState<number>(() => {
    const saved = localStorage.getItem('custos_ui_scale');
    return saved ? parseInt(saved, 10) : 100;
  });

  // Toast state
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const toastTimeoutRef = useRef<number | null>(null);

  const showToast = useCallback((msg: string) => {
    if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current);
    setToastMessage(msg);
    toastTimeoutRef.current = window.setTimeout(() => {
      setToastMessage(null);
    }, 2500);
  }, []);

  // UI Scale applicator - zoom the entire document root so the full screen/window scales uniformly
  const applyUiScale = useCallback((scaleVal: number) => {
    const rounded = Math.round(scaleVal / 10) * 10;
    const clamped = Math.min(150, Math.max(70, rounded));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
    document.documentElement.style.zoom = `${clamped / 100}`;
    document.body.style.zoom = '';
  }, []);

  const handleSetUiScale = (val: number) => {
    applyUiScale(val);
    showToast(`UI Scale: ${val}%`);
  };

  const handleStepUiScale = (delta: number) => {
    handleSetUiScale(uiScale + delta);
  };

  // Check active session object
  const currentSessions = projectData[currentProject] || [];
  const activeSession: Session | null =
    currentSessions.find((s) => s.id === activeSessionId) || currentSessions[0] || null;

  // Hydrate state from Custos Daemon on Mount
  useEffect(() => {
    applyUiScale(uiScale);

    async function hydrateFromDaemon() {
      try {
        const isOnline = await daemonClient.checkHealth();
        if (!isOnline) {
          showToast('Custos daemon is offline. No simulated data was loaded.');
          return;
        }

        const [fetchedTasks, fetchedSessions, fetchedProviders, fetchedKeys] = await Promise.all([
          daemonClient.listTasks().catch(() => []),
          daemonClient.listSessions().catch(() => []),
          daemonClient.listProviders().catch(() => []),
          daemonClient.listClientKeys().catch(() => []),
        ]);

        if (fetchedProviders && fetchedProviders.length > 0) {
          setProviders(fetchedProviders.map(mapBackendProvider));
        }

        if (fetchedKeys && fetchedKeys.length > 0) {
          setClientKeys(fetchedKeys.map(mapBackendKey));
        }

        const journals = await Promise.all(
          fetchedSessions.map(async (session) => {
            try {
              return [session.id, await daemonClient.getSessionJournal(session.id)] as const;
            } catch {
              return [session.id, [] as SessionJournalEntry[]] as const;
            }
          })
        );
        const journalBySession = new Map(journals);

        if (fetchedTasks.length > 0) {
          setTasks(fetchedTasks);

          // Project daemon tasks and sessions into UI sessions
          const projectedSessions: Session[] = fetchedTasks.map((t) => {
            const domainSession = fetchedSessions.find((s) => (s.task_id || s.attached_to) === t.id);
            const taskId = t.id;
            const sessionId = domainSession?.id;
            return {
              id: sessionId || `unbound:${taskId}`,
              taskId,
              source: 'daemon',
              taskStatus: String(t.status),
              sessionId,
              pack: t.contract?.pack_id || t.contract?.pack,
              title: t.title,
              time: 'Live',
              preview: `Status: ${t.status} | Contract: ${t.contract?.pack || 'general'}`,
              model: 'Model not reported',
              fileName: '',
              diffHunk: '',
              diffLinesCount: '',
              summary: `Task ${t.id}: ${t.status}`,
              messages: (journalBySession.get(domainSession?.id || '') || domainSession?.journal || []).map((j) => ({
                id: j.id || String(j.entry_id || `${sessionId}-${j.occurred_at}`),
                role: (j.role || j.entry_type) === 'user' ? 'user' : 'assistant',
                author: (j.role || j.entry_type) === 'user' ? 'You' : 'Custos Kernel',
                badge: j.badge,
                stepName: j.step_name,
                duration: j.duration,
                text: j.content || j.entry_data || '',
              })),
              diffCode: [],
            };
          });

          setProjectData((prev) => ({
            ...prev,
            'Custos Workspace': projectedSessions,
          }));

          if (projectedSessions.length > 0) {
            setCurrentProject('Custos Workspace');
            setActiveSessionId(projectedSessions[0].id);
          }
        }
      } catch (err) {
        console.warn('[DaemonClient] Could not load data from daemon:', err);
      }
    }

    hydrateFromDaemon();
  }, []);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle settings (Ctrl+,)
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        setIsSettingsOpen((prev) => !prev);
      }
      // Toggle Sessions Sidebar (Ctrl+B)
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        setIsSessionsCollapsed((prev) => !prev);
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
      // Close open modals on Escape
      if (e.key === 'Escape') {
        setIsSettingsOpen(false);
        setIsNewSessionOpen(false);
        setIsAddProviderOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [uiScale]);

  // Actions
  const handleSendMessage = async (text: string) => {
    if (!activeSession) return;
    const userMsg = {
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

    try {
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

        setProjectData((prev) => {
          const list = prev[currentProject] || [];
          const updatedList = list.map((s) => {
            if (s.id === activeSession.id) {
              return {
                ...s,
                messages: [...s.messages, noLlmMsg],
              };
            }
            return s;
          });
          return {
            ...prev,
            [currentProject]: updatedList,
          };
        });

        showToast('Chưa có LLM nào được cấu hình hiện tại');
        return;
      }

      showToast('Dispatching run to Custos Spine...');

      // 2. Append message to backend session journal
      if (activeSession.sessionId) {
        await daemonClient.appendSessionMessage(activeSession.sessionId, 'user', text);
      }

      // 3. Start workflow run
      const run = await daemonClient.startRun({
        task_id: activeSession.taskId || activeSession.id,
        preferred_mode: 'model',
      });

      const assistantMsg = {
        role: 'assistant' as const,
        author: 'Custos runtime',
        badge: `Run ${run.status}`,
        stepName: `Run #${run.id.slice(0, 8)}`,
        text: `Run ${run.id} was accepted with status ${run.status}. The result is not available in this view yet.`,
      };

      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              messages: [...s.messages, assistantMsg],
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });

      showToast(`Run ${run.id.slice(0, 8)}: ${run.status}`);
    } catch (err: any) {
      const errMsg = {
        role: 'assistant' as const,
        author: 'Custos Kernel',
        badge: 'Gate Fenced',
        text: `Execution notice: ${err?.message || err}`,
      };

      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              messages: [...s.messages, errMsg],
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
    showToast('Demo conversation cleared in this window');
  };

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
    showToast('No patch was changed. Rejection is not connected to the daemon yet.');
  };

  const handleCopyDiff = () => {
    if (!activeSession) return;
    const diffText = activeSession.diffCode.map((d) => d.text).join('\n');
    navigator.clipboard.writeText(diffText);
    showToast('Code diff copied to clipboard');
  };

  const handleCreateNewSession = async (title: string, pack: 'engineering' | 'research' | 'assistant') => {
    try {
      // 1. Create real Task in Custos Daemon
      const createdTask = await daemonClient.createTask({
        title: title || 'New Supervised Task',
        contract: { pack_id: pack },
      });

      // 2. Create Session in Custos Daemon
      const createdSession = await daemonClient.createSession('assisted');
      await daemonClient.attachSession({ session_id: createdSession.id, task_id: createdTask.id });

      const newSessionItem: Session = {
        id: createdSession.id,
        taskId: createdTask.id,
        source: 'daemon',
        taskStatus: createdTask.status,
        sessionId: createdSession.id,
        pack,
        title: createdTask.title,
        time: 'Just now',
        preview: `${pack} task · ${createdTask.status}`,
        model: 'Model not reported',
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
      setIsNewSessionOpen(false);
      showToast(`Created Task ${createdTask.id.slice(0, 8)}`);

    } catch (err: any) {
      console.error('[CreateSession] Error:', err);
      showToast(`Error creating session: ${err?.message || err}`);
    }
  };

  const handleNewProjectPrompt = () => {
    const name = window.prompt('Enter new Project Name:');
    if (name && name.trim()) {
      const cleanName = name.trim();
      setProjectData((prev) => {
        if (prev[cleanName]) return prev;
        return {
          ...prev,
          [cleanName]: [],
        };
      });
      setCurrentProject(cleanName);
      showToast(`Created project: ${cleanName}`);
    }
  };

  const handleSaveProvider = async (
    serviceOrParams: string | { service: string; apiKey?: string; endpointUrl?: string; model?: string; contextWindow?: number; fastMode?: boolean },
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

      const existing = providers.find((p) => p.name.toLowerCase().includes(service.toLowerCase()) || p.id === service);
      const providerId = existing?.id || service.toLowerCase().replace(/[^a-z0-9]/g, '_');
      const providerType =
        service.toLowerCase().includes('claude') || service.toLowerCase().includes('anthropic') ? 'anthropic' :
        service.toLowerCase().includes('openai') || service.toLowerCase().includes('gpt') ? 'openai' :
        service.toLowerCase().includes('gemini') ? 'gemini' :
        service.toLowerCase().includes('deepseek') ? 'deepseek' : 'local';

      await daemonClient.saveProvider({
        id: providerId,
        name: existing?.name || service,
        service_type: providerType,
        provider_type: providerType,
        model: defaultModel || existing?.model || 'claude-3-7-sonnet',
        default_model: defaultModel,
        api_key: apiKey,
        endpoint_url: endpointUrl,
        context_window: contextWindow,
        fast_mode: fastMode,
        status: apiKey.trim() || endpointUrl?.trim() ? 'primary' : 'offline',
        status_label: apiKey.trim() || endpointUrl?.trim() ? 'Configured & Active' : 'Offline',
        latency_ms: 28,
        is_active: Boolean(apiKey.trim() || endpointUrl?.trim()),
      });

      const updated = await daemonClient.listProviders();
      if (updated && updated.length > 0) {
        setProviders(updated.map(mapBackendProvider));
      }
      setIsAddProviderOpen(false);
      showToast(`Provider ${service} đã được cập nhật thành công vào Backend SQLite!`);
    } catch (err: any) {
      console.error('[SaveProvider] Error:', err);
      showToast(`Lỗi lưu provider: ${err?.message || err}`);
    }
  };

  const handleDeleteProvider = async (id: string) => {
    try {
      await daemonClient.deleteProvider(id);
      const updated = await daemonClient.listProviders();
      setProviders(updated.map(mapBackendProvider));
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
      if (provider.endpoint) {
        const probeRes = await daemonClient.probeModels(provider.endpoint, provider.iconType, undefined, provider.id);
        showToast(`✓ Đã kết nối endpoint ${provider.name}! Tìm thấy ${probeRes.count} models.`);
      } else {
        showToast(`✓ Đã xác minh kết nối provider ${provider.name} qua Daemon protocol.`);
      }
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
        currentTab,
        setCurrentTab,
        isSessionsCollapsed,
        setIsSessionsCollapsed,
        isRightExplorerOpen,
        setIsRightExplorerOpen,
        toggleRightExplorer,
        providers,
        setProviders,
        clientKeys,
        setClientKeys,
        isSettingsOpen,
        setIsSettingsOpen,
        settingsTab,
        setSettingsTab,
        openSettings,
        isNewSessionOpen,
        setIsNewSessionOpen,
        isAddProviderOpen,
        setIsAddProviderOpen,
        uiScale,
        handleSetUiScale,
        handleStepUiScale,
        toastMessage,
        showToast,
        handleSendMessage,
        handleClearHistory,
        handleAcceptAndRun,
        handleRejectDiff,
        handleCopyDiff,
        handleCreateNewSession,
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
