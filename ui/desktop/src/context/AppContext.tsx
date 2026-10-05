import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import { MainTab, ProjectData, ProviderItem, ClientApiKey, Session } from '../types';
import { initialProjectData, initialProviders, initialClientKeys } from '../data/mockData';
import { daemonClient } from '../api/daemon_client';
import { Task } from '../types/domain';

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
  handleSaveProvider: (service: string, apiKey: string) => void;
  handleGenerateClientKey: () => void;
  handleRevokeClientKey: (id: string) => void;
}

const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Projects & Sessions state
  const [projectData, setProjectData] = useState<ProjectData>(() => daemonClient.isDemoMode
    ? Object.fromEntries(Object.entries(initialProjectData).map(([name, sessions]) =>
      [name, sessions.map((session) => ({ ...session, source: 'demo' as const }))]))
    : { 'Custos OS': [] });
  const [currentProject, setCurrentProject] = useState<string>('Custos OS');
  const [activeSessionId, setActiveSessionId] = useState<string>('auth');
  const [tasks, setTasks] = useState<Task[]>([]);

  // Navigation & Layout state
  const [currentTab, setCurrentTab] = useState<MainTab>('studio');
  const [isSessionsCollapsed, setIsSessionsCollapsed] = useState<boolean>(false);
  const [isRightExplorerOpen, setIsRightExplorerOpen] = useState<boolean>(true);

  const toggleRightExplorer = useCallback(() => {
    setIsRightExplorerOpen((prev) => !prev);
  }, []);

  // Providers & Keys state
  const [providers, setProviders] = useState<ProviderItem[]>(initialProviders);
  const [clientKeys, setClientKeys] = useState<ClientApiKey[]>(initialClientKeys);

  // Modals state
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [isNewSessionOpen, setIsNewSessionOpen] = useState<boolean>(false);
  const [isAddProviderOpen, setIsAddProviderOpen] = useState<boolean>(false);

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
    const clamped = Math.min(150, Math.max(75, scaleVal));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
    document.documentElement.style.zoom = (clamped / 100).toFixed(2);
    document.body.style.zoom = '1';
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
      if (daemonClient.isDemoMode) return;
      try {
        const [fetchedTasks, fetchedSessions] = await Promise.all([
          daemonClient.listTasks(),
          daemonClient.listSessions(),
        ]);

        if (fetchedTasks.length > 0) {
          setTasks(fetchedTasks);

          // Project daemon tasks and sessions into UI sessions
          const projectedSessions: Session[] = fetchedTasks.map((t) => {
            const domainSession = fetchedSessions.find((s) => s.task_id === t.id);
            return {
              id: t.id,
              source: daemonClient.isDemoMode ? 'demo' : 'daemon',
              taskStatus: t.status,
              sessionId: domainSession?.id,
              pack: t.contract?.pack,
              title: t.title,
              time: 'Live',
              preview: `Status: ${t.status} | Contract: ${t.contract?.pack || 'general'}`,
              model: 'Model not reported',
              fileName: '',
              diffHunk: '',
              diffLinesCount: '',
              summary: `Task ${t.id}: ${t.status}`,
              messages: (domainSession?.journal || []).map((j) => ({
                id: j.id,
                role: j.role === 'user' ? 'user' : 'assistant',
                author: j.role === 'user' ? 'You' : 'Custos Kernel',
                badge: j.badge,
                stepName: j.step_name,
                duration: j.duration,
                text: j.content,
              })),
              diffCode: [],
            };
          });

          setProjectData((prev) => ({
            ...prev,
            'Custos OS': projectedSessions,
          }));

          if (projectedSessions.length > 0) {
            setActiveSessionId(projectedSessions[0].id);
          }
        }
      } catch (err) {
        console.warn('[DaemonClient] Could not load tasks:', err);
      }
    }

    hydrateFromDaemon();
  }, []);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle settings (⌘,)
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        setIsSettingsOpen((prev) => !prev);
      }
      // Toggle Sessions Sidebar (⌘B)
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        setIsSessionsCollapsed((prev) => !prev);
      }
      // UI Zoom In (⌘+ or ⌘=)
      if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handleStepUiScale(10);
      }
      // UI Zoom Out (⌘-)
      if ((e.metaKey || e.ctrlKey) && (e.key === '-' || e.key === '_')) {
        e.preventDefault();
        handleStepUiScale(-10);
      }
      // Reset UI Scale (⌘0)
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

    showToast('Dispatching run to Custos Spine...');

    try {
      // 1. Append message to backend session journal
      if (activeSession.sessionId) {
        await daemonClient.appendSessionMessage(activeSession.sessionId, 'user', text);
      }

      // 2. Start workflow run
      const run = await daemonClient.startRun({
        task_id: activeSession.id,
        instruction: text,
        preferred_mode: 'model',
      });

      const assistantMsg = {
        role: 'assistant' as const,
        author: daemonClient.isDemoMode ? 'Demo simulator' : 'Custos runtime',
        badge: `${daemonClient.isDemoMode ? 'Demo run' : 'Run'} ${run.status}`,
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
    if (activeSession.source === 'demo') {
      showToast('Demo patch only. No code was applied.');
      return;
    }
    try {
      await daemonClient.advanceTask(activeSession.id, 'verification', 'Active');
      showToast('Task advanced to verification. Check the task status for the result.');
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
        contract: { pack },
      });

      // 2. Create Session in Custos Daemon
      const createdSession = await daemonClient.createSession('supervised', createdTask.id);

      const newSessionItem: Session = {
        id: createdTask.id,
        source: daemonClient.isDemoMode ? 'demo' : 'daemon',
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

      setActiveSessionId(createdTask.id);
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

  const handleSaveProvider = (service: string, apiKey: string) => {
    setProviders((prev) =>
      prev.map((p) => {
        if (p.name.toLowerCase().includes(service.toLowerCase())) {
          return {
            ...p,
            apiKey: apiKey.slice(0, 7) + '••••••••',
            status: 'primary',
            statusLabel: 'Configured',
          };
        }
        return p;
      })
    );
    setIsAddProviderOpen(false);
    showToast(`Demo provider setting updated for ${service}; no credential was stored.`);
  };

  const handleGenerateClientKey = () => {
    const newKey: ClientApiKey = {
      id: `key-${Date.now()}`,
      name: 'External Client API Token',
      token: `custos_live_sec_••••${Math.floor(1000 + Math.random() * 9000)}`,
      created: new Date().toISOString().split('T')[0],
      icon: 'terminal',
    };
    setClientKeys((prev) => [newKey, ...prev]);
    showToast('Demo token added locally; it cannot authenticate clients.');
  };

  const handleRevokeClientKey = (id: string) => {
    setClientKeys((prev) => prev.filter((k) => k.id !== id));
    showToast('Demo token removed locally.');
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
