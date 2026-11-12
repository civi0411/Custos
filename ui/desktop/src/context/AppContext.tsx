import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import type { MainTab, ProjectData, Session } from '../types';
import { daemonClient } from '../api/daemon_client';

import { useAppearanceSlice } from './app/appearance';
import { useTasksSlice } from './app/tasks';
import { useWorkspacesSlice } from './app/workspaces';
import { useCanvasLayoutSlice, type ActiveWorkbench, getDefaultTabForMode } from './app/canvas_layout';
import { useModalsSlice } from './app/modals';
import { useProvidersSlice } from './app/providers';
import { useSessionsSlice } from './app/sessions';

import type { AppContextType } from './app/types';
import { defaultProbeBaseUrl } from './app/providers_utils';

export type { ActiveWorkbench, AppContextType };
export { getDefaultTabForMode, defaultProbeBaseUrl };



const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Projects & Sessions state
  const [projectData, setProjectData] = useState<ProjectData>({ 'AgentHub': [] });
  const [currentProject, setCurrentProject] = useState<string>('AgentHub');
  const [activeSessionId, setActiveSessionId] = useState<string>('');

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

  // Navigation state
  const [currentTab, setCurrentTab] = useState<MainTab>('studio');
  const [activeWorkbench, setActiveWorkbenchState] = useState<ActiveWorkbench>(() => {
    try {
      const saved = localStorage.getItem('custos.workspace.mode.v2');
      if (saved === 'copilot' || saved === 'chat') return 'copilot';
      if (saved === 'engineering' || saved === 'code') return 'engineering';
      if (saved === 'research') return 'research';
      if (saved === 'quant') return 'quant';
    } catch { }
    return 'copilot';
  });

  const setActiveWorkbench = useCallback((wb: ActiveWorkbench) => {
    setActiveWorkbenchState(wb);
    try {
      localStorage.setItem('custos.workspace.mode.v2', wb);
    } catch { }
  }, []);

  const [installedWorkbenches, setInstalledWorkbenches] = useState<string[]>([
    'assistant',
    'copilot',
    'coding',
    'engineering',
  ]);
  const [isWorkbenchLibraryOpen, setIsWorkbenchLibraryOpen] = useState<boolean>(false);

  const refreshInstalledWorkbenches = useCallback(async () => {
    try {
      const inv = await daemonClient.listPackageInventory();
      if (inv && Array.isArray(inv.items)) {
        const enabled = inv.items
          .filter((i) => {
            if (i.state === 'disabled' || i.enabled_in_profile === '0' || i.is_enabled === false) {
              return false;
            }
            return i.state === 'builtin' || i.state === 'installed';
          })
          .map((i) => i.component_id);
        const allAliases = new Set<string>();
        for (const id of enabled) {
          allAliases.add(id);
          if (id === 'assistant') allAliases.add('copilot');
          if (id === 'copilot') allAliases.add('assistant');
          if (id === 'coding') allAliases.add('engineering');
          if (id === 'engineering') allAliases.add('coding');
        }
        setInstalledWorkbenches(Array.from(allAliases));
      }
    } catch {
      setInstalledWorkbenches((prev) =>
        prev.length === 0
          ? ['assistant', 'copilot', 'coding', 'engineering']
          : prev
      );
    }
  }, []);

  useEffect(() => {
    refreshInstalledWorkbenches();
  }, [refreshInstalledWorkbenches]);

  const [isRightExplorerOpen, setIsRightExplorerOpen] = useState<boolean>(true);
  const toggleRightExplorer = useCallback(() => {
    setIsRightExplorerOpen((prev) => !prev);
  }, []);

  // Modals slice
  const modalsSlice = useModalsSlice({ showToast });

  // Appearance slice
  const {
    appearance,
    resolvedTheme,
    updateAppearance,
    uiScale,
    handleSetUiScale,
    handleStepUiScale,
  } = useAppearanceSlice(showToast);

  // Tasks slice
  const {
    tasks,
    setTasks,
    refreshTasks,
    createTask,
    advanceTask,
    completeTask,
    cancelTask,
  } = useTasksSlice(showToast);

  // Workspaces slice
  const {
    workspaces,
    activeWorkspaceId,
    setActiveWorkspaceId,
    activeWorkspace,
    refreshWorkspaces,
    activeDetection,
    refreshWorkspaceDetection,
    handleProjectOpened,
    handleOpenFolderDirect,
  } = useWorkspacesSlice({
    currentProject,
    setCurrentProject,
    onProjectRegistered: useCallback((projectName: string) => {
      setProjectData((prev) => {
        if (prev[projectName]) return prev;
        return {
          ...prev,
          [projectName]: [],
        };
      });
    }, []),
    onModelSelect: useCallback((model: string) => {
      providersSlice.setSelectedModel(model);
    }, []),
    onOpenProjectFallback: useCallback(() => {
      modalsSlice.setIsOpenProjectOpen(true);
    }, [modalsSlice.setIsOpenProjectOpen]),
    showToast,
  });

  // Canvas Layout slice
  const canvasSlice = useCanvasLayoutSlice({
    currentProject,
    activeWorkbench,
    activeWorkspaceId,
    activeSessionId,
    isSidebarCollapsed: modalsSlice.isSidebarCollapsed,
  });

  // Active session object
  const currentSessions = projectData[currentProject] || [];
  const activeSession: Session | null =
    currentSessions.find((s) => s.id === activeSessionId) || null;

  // Providers & Keys & Models slice
  const providersSlice = useProvidersSlice({
    currentProject,
    activeSessionId,
    activeSession,
    setProjectData,
    showToast,
    setIsAddProviderOpen: modalsSlice.setIsAddProviderOpen,
  });

  // Sessions action slice
  const sessionsSlice = useSessionsSlice({
    currentProject,
    setCurrentProject,
    projectData,
    setProjectData,
    activeSessionId,
    setActiveSessionId,
    activeSession,
    selectedModel: providersSlice.selectedModel,
    selectedModelAccountId: providersSlice.selectedModelAccountId,
    selectedReasoningEffort: providersSlice.selectedReasoningEffort,
    selectedModelAvailable: providersSlice.selectedModelAvailable,
    combos: providersSlice.combos,
    connectedAccounts: providersSlice.connectedAccounts,
    setConnectedAccounts: providersSlice.setConnectedAccounts,
    workspaces,
    setTasks,
    setIsNewSessionOpen: modalsSlice.setIsNewSessionOpen,
    showToast,
    refreshCatalogAvailability: providersSlice.refreshCatalogAvailability,
    appearance,
    handleOpenFolderDirect,
  });

  // Sync projects with daemon ExecutionWorkspaces on mount
  useEffect(() => {
    daemonClient.listWorkspaces().then((wspaces) => {
      const active = (wspaces || []).filter((w) => {
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

  // Hydrate state from Custos Daemon on Mount
  useEffect(() => {
    handleSetUiScale(uiScale);

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

        void providersSlice.refreshProviderState().catch((error) => {
          showToast(`Could not load provider settings: ${error instanceof Error ? error.message : String(error)}`);
        });

        setTasks(fetchedTasks);

        const journalResults = await Promise.allSettled(
          fetchedSessions.map(async (session) => {
            try {
              const journal = await daemonClient.getSessionJournal(session.id);
              return [session.id, journal] as const;
            } catch (err) {
              console.warn(`[Hydration] Failed to fetch journal for session ${session.id}:`, err);
              return [session.id, []] as const;
            }
          })
        );
        const journalBySession = new Map<string, any>();
        for (const res of journalResults) {
          if (res.status === 'fulfilled') {
            journalBySession.set(res.value[0], res.value[1]);
          }
        }

        if (fetchedSessions.length > 0) {
          const projectedSessions: Session[] = fetchedSessions.map((domainSession) => {
            const attachedTaskId = domainSession.task_id || domainSession.attached_to
              || (typeof domainSession.mode === 'object' ? domainSession.mode.attached?.task_id : undefined);
            const t = fetchedTasks.find((task) => task.id === attachedTaskId);
            const taskId = t?.id;
            const sessionId = domainSession.id;
            let selection: { model?: string; accountId?: string; effort?: string } = {};
            try {
              const saved = JSON.parse(localStorage.getItem(`custos.conversation_model.${sessionId}`) || '{}');
              if (saved && typeof saved.model === 'string') {
                selection = {
                  model: saved.model,
                  accountId: typeof saved.accountId === 'string' ? saved.accountId : undefined,
                  effort: typeof saved.effort === 'string' ? saved.effort : undefined,
                };
              }
            } catch {}
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
              model: t?.metadata?.kind === 'native_coding_agent' ? t.metadata.harness_id : selection.model ?? (t?.contract as any)?.model ?? providersSlice.selectedModel ?? '',
              modelAccountId: selection.accountId || providersSlice.selectedModelAccountId,
              reasoningEffort: selection.effort || providersSlice.selectedReasoningEffort,
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

          const allProjected = projectedSessions;
          const targetProject = currentProject || 'AgentHub';
          setProjectData((prev) => ({
            ...prev,
            [targetProject]: allProjected,
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
                setProjectData(previous => ({
                  ...previous,
                  [targetProject]: (previous[targetProject] || []).map(item => item.id !== chat.id ? item : {
                    ...item,
                    messages: item.messages.map(message => message.recordedAt === run.journal_occurred_at ? {
                      ...message,
                      runId: run.run_id,
                      runStatus: run.status,
                      badge: run.status,
                      author: run.model || message.author,
                    } : message),
                  }),
                }));
              }
              sessionsSlice.followLiveRun(targetProject, chat.id, chat.sessionId || chat.id, run.run_id, run.model || 'Custos');
            }
          }

          setCurrentProject(targetProject);
          setActiveSessionId((previous) => allProjected.some((session) => session.id === previous)
            ? previous : (allProjected[0]?.id || ''));
        } else {
          const targetProject = currentProject || 'AgentHub';
          setProjectData((previous) => ({ ...previous, [targetProject]: [] }));
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
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        modalsSlice.setIsSettingsOpen((prev) => !prev);
      }
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        modalsSlice.setIsSidebarCollapsed((prev) => !prev);
      }
      if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handleStepUiScale(10);
      }
      if ((e.metaKey || e.ctrlKey) && (e.key === '-' || e.key === '_')) {
        e.preventDefault();
        handleStepUiScale(-10);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '0') {
        e.preventDefault();
        handleSetUiScale(100);
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '3') {
        e.preventDefault();
        modalsSlice.openAiSetup('cloud');
      }
      if (e.key === 'Escape') {
        modalsSlice.setIsSettingsOpen(false);
        modalsSlice.setIsAiSetupOpen(false);
        modalsSlice.setIsNewSessionOpen(false);
        modalsSlice.setIsAddProviderOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [uiScale, modalsSlice, handleSetUiScale, handleStepUiScale]);

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
        createTask,
        advanceTask,
        completeTask,
        cancelTask,
        workspaces,
        activeWorkspaceId,
        setActiveWorkspaceId,
        activeWorkspace,
        refreshWorkspaces,
        currentTab,
        setCurrentTab,
        activeWorkbench,
        setActiveWorkbench,
        installedWorkbenches,
        refreshInstalledWorkbenches,
        isWorkbenchLibraryOpen,
        setIsWorkbenchLibraryOpen,
        isRightExplorerOpen,
        setIsRightExplorerOpen,
        toggleRightExplorer,
        ...modalsSlice,
        ...canvasSlice,
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
        activeDetection,
        refreshWorkspaceDetection,
        handleProjectOpened,
        handleOpenFolderDirect,
        ...providersSlice,
        ...sessionsSlice,
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
