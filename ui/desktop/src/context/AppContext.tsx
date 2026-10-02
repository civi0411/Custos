import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ViewMode, MainTab, ProjectData, ProviderItem, ClientApiKey, Session } from '../types';
import { initialProjectData, initialProviders, initialClientKeys } from '../data/mockData';

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

  // Navigation & Layout
  currentTab: MainTab;
  setCurrentTab: (tab: MainTab) => void;
  viewMode: ViewMode;
  setViewMode: (mode: ViewMode) => void;
  isSessionsCollapsed: boolean;
  setIsSessionsCollapsed: React.Dispatch<React.SetStateAction<boolean>>;
  splitPercent: number;
  setSplitPercent: (percent: number) => void;
  isDragging: boolean;
  setIsDragging: (dragging: boolean) => void;

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
  handleSendMessage: (text: string) => void;
  handleClearHistory: () => void;
  handleAcceptAndRun: () => void;
  handleRejectDiff: () => void;
  handleCopyDiff: () => void;
  handleCreateNewSession: (title: string, task: string) => void;
  handleNewProjectPrompt: () => void;
  handleSaveProvider: (service: string, apiKey: string) => void;
  handleGenerateClientKey: () => void;
  handleRevokeClientKey: (id: string) => void;
}

const AppContext = createContext<AppContextType | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Projects & Sessions state
  const [projectData, setProjectData] = useState<ProjectData>(initialProjectData);
  const [currentProject, setCurrentProject] = useState<string>('Default project');
  const [activeSessionId, setActiveSessionId] = useState<string>('auth');

  // Navigation & Layout state
  const [currentTab, setCurrentTab] = useState<MainTab>('studio');
  const [viewMode, setViewMode] = useState<ViewMode>('split');
  const [isSessionsCollapsed, setIsSessionsCollapsed] = useState<boolean>(false);
  const [splitPercent, setSplitPercent] = useState<number>(50);
  const [isDragging, setIsDragging] = useState<boolean>(false);

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

  // Initial Tauri verification & UI Scale
  useEffect(() => {
    applyUiScale(uiScale);

    invoke<string>('greet', { name: 'Custos UI' })
      .then((greeting) => {
        console.log('[Tauri IPC Bridge]', greeting);
      })
      .catch((err) => {
        console.warn('[Tauri IPC]', err);
      });
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
      // Toggle Split View / Full view (⌘\)
      if ((e.metaKey || e.ctrlKey) && e.key === '\\') {
        e.preventDefault();
        setViewMode((prev) => {
          if (prev === 'split') return 'chat';
          if (prev === 'chat') return 'diff';
          return 'split';
        });
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
  const handleSendMessage = (text: string) => {
    if (!activeSession) return;
    const userMsg = {
      role: 'user' as const,
      author: 'You',
      text
    };

    const assistantMsg = {
      role: 'assistant' as const,
      author: 'Custos Agent',
      badge: 'Autonomous Kernel Engine',
      stepName: 'Analyzing AST tree & compiling patch',
      duration: '48ms',
      text: `Received prompt: "${text}". Synthesizing change set and verifying invariants against Task Kernel...`
    };

    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return {
            ...s,
            messages: [...s.messages, userMsg, assistantMsg]
          };
        }
        return s;
      });
      return {
        ...prev,
        [currentProject]: updatedList
      };
    });

    showToast('Task submitted to Custos Engine');
  };

  const handleClearHistory = () => {
    if (!activeSession) return;
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
    showToast('Conversation cleared');
  };

  const handleAcceptAndRun = () => {
    showToast('Code diff accepted and running in sandbox');
  };

  const handleRejectDiff = () => {
    showToast('Diff rejected and reverted');
  };

  const handleCopyDiff = () => {
    if (!activeSession) return;
    const diffText = activeSession.diffCode.map((l) => l.text).join('\n');
    navigator.clipboard.writeText(diffText);
    showToast('Diff copied to clipboard');
  };

  const handleCreateNewSession = (title: string, task: string) => {
    const newId = `session-${Date.now()}`;
    const newSession: Session = {
      id: newId,
      title,
      time: 'Just now',
      preview: task || 'New session created...',
      model: 'Claude 3.7 Sonnet',
      fileName: 'src/main.rs',
      diffHunk: '@@ -1,5 +1,9 @@ main()',
      diffLinesCount: '+4 -0 lines',
      summary: 'Initial draft patch generated by Custos autonomous runtime.',
      messages: task
        ? [
            { role: 'user', author: 'You', text: task },
            {
              role: 'assistant',
              author: 'Custos Agent',
              badge: 'Session Initialized',
              text: `Started session "${title}". Preparing plan and context window...`
            }
          ]
        : [],
      diffCode: [
        { type: 'context', text: ' fn main() {' },
        { type: 'add', text: '+    println!("Hello from Custos!");' },
        { type: 'context', text: ' }' }
      ]
    };

    setProjectData((prev) => ({
      ...prev,
      [currentProject]: [newSession, ...(prev[currentProject] || [])]
    }));

    setActiveSessionId(newId);
    setIsNewSessionOpen(false);
    showToast(`Started session: ${title}`);
  };

  const handleNewProjectPrompt = () => {
    const name = window.prompt('Enter new project name:');
    if (name && name.trim()) {
      const trimmed = name.trim();
      if (!projectData[trimmed]) {
        setProjectData((prev) => ({
          ...prev,
          [trimmed]: []
        }));
      }
      setCurrentProject(trimmed);
      showToast(`Switched to project: ${trimmed}`);
    }
  };

  const handleSaveProvider = (service: string, apiKey: string) => {
    const masked = apiKey.length > 8 ? `${apiKey.slice(0, 7)}••••••••${apiKey.slice(-4)}` : '••••••••';
    const newProvider: ProviderItem = {
      id: `p-${Date.now()}`,
      name: service === 'anthropic' ? 'Anthropic' : service === 'openai' ? 'OpenAI' : 'Google Cloud',
      model: service === 'anthropic' ? 'claude-3-7-sonnet' : 'gpt-4o',
      status: 'standby',
      statusLabel: 'Standby Route',
      badgeColor: 'text-neutral-400 bg-surface-elevated',
      apiKey: masked,
      latency: '~200ms',
      iconType: service as any
    };
    setProviders((prev) => [...prev, newProvider]);
    setIsAddProviderOpen(false);
    showToast(`Added provider: ${newProvider.name}`);
  };

  const handleGenerateClientKey = () => {
    const token = `custos_live_${Math.random().toString(36).substring(2, 6)}••••••••${Math.random().toString(36).substring(2, 6)}`;
    const newKey: ClientApiKey = {
      id: `k-${Date.now()}`,
      name: 'Custom Desktop Client',
      token,
      created: 'Just now',
      icon: 'laptop'
    };
    setClientKeys((prev) => [newKey, ...prev]);
    showToast('Generated new client API key');
  };

  const handleRevokeClientKey = (id: string) => {
    setClientKeys((prev) => prev.filter((k) => k.id !== id));
    showToast('Client API key revoked');
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
        currentTab,
        setCurrentTab,
        viewMode,
        setViewMode,
        isSessionsCollapsed,
        setIsSessionsCollapsed,
        splitPercent,
        setSplitPercent,
        isDragging,
        setIsDragging,
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
        handleRevokeClientKey
      }}
    >
      {children}
    </AppContext.Provider>
  );
};

export const useAppContext = (): AppContextType => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useAppContext must be used within an AppProvider');
  }
  return context;
};
