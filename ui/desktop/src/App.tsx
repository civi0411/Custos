import React, { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ViewMode, MainTab, ProjectData, ProviderItem, ClientApiKey, Session } from './types';
import { initialProjectData, initialProviders, initialClientKeys } from './data/mockData';
import { Header } from './components/Header';
import { IconSidebar } from './components/IconSidebar';
import { SessionsSidebar } from './components/SessionsSidebar';
import { ChatSection } from './components/ChatSection';
import { DiffSection } from './components/DiffSection';
import { Splitter } from './components/Splitter';
import { ProvidersView } from './components/ProvidersView';
import { StatusBar } from './components/StatusBar';
import { SettingsModal } from './components/SettingsModal';
import { NewSessionModal } from './components/NewSessionModal';
import { AddProviderModal } from './components/AddProviderModal';
import { Toast } from './components/Toast';

export const App: React.FC = () => {
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

  // UI Scale applicator
  const applyUiScale = useCallback((scaleVal: number) => {
    const clamped = Math.min(150, Math.max(75, scaleVal));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
    // Apply zoom
    document.body.style.zoom = (clamped / 100).toFixed(2);
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

  // Initial Tauri verification
  useEffect(() => {
    applyUiScale(uiScale);

    // Test Tauri IPC bridge in background
    invoke<string>('greet', { name: 'Custos UI' })
      .then((greeting) => {
        console.log('[Tauri IPC Bridge]', greeting);
      })
      .catch((err) => {
        console.warn('[Tauri IPC]', err);
      });
  }, []);

  // Draggable Splitter
  const containerRef = useRef<HTMLDivElement>(null);

  const handleSplitterDragStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    document.body.classList.add('resizer-active');
  };

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const offsetX = e.clientX - rect.left;
      let percent = (offsetX / rect.width) * 100;
      if (percent < 20) percent = 20;
      if (percent > 80) percent = 80;
      setSplitPercent(percent);
    };

    const handleMouseUp = () => {
      if (isDragging) {
        setIsDragging(false);
        document.body.classList.remove('resizer-active');
      }
    };

    if (isDragging) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    }

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging]);

  // Window resize handler for responsiveness
  useEffect(() => {
    let lastW = window.innerWidth;
    const handleResize = () => {
      const w = window.innerWidth;
      if (w < 768 && viewMode === 'split') {
        setViewMode('chat');
      } else if (w >= 1024 && lastW < 768 && viewMode === 'chat') {
        setViewMode('split');
      }
      lastW = w;
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [viewMode]);

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
    <div className="bg-black text-[#e2e4ea] font-sans antialiased h-screen overflow-hidden flex flex-col select-none">
      {/* Top Header */}
      <Header
        currentProject={currentProject}
        projectNames={Object.keys(projectData)}
        onSelectProject={(p) => {
          setCurrentProject(p);
          const first = (projectData[p] || [])[0];
          if (first) setActiveSessionId(first.id);
        }}
        onNewProjectPrompt={handleNewProjectPrompt}
        isSessionsCollapsed={isSessionsCollapsed}
        onToggleSessions={() => setIsSessionsCollapsed((prev) => !prev)}
        viewMode={viewMode}
        onSetViewMode={setViewMode}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onShowToast={showToast}
      />

      {/* Main Body */}
      <div className="flex-1 flex overflow-hidden min-w-0 relative">
        {/* Left Compact Icon Sidebar */}
        <IconSidebar
          currentTab={currentTab}
          onSwitchTab={setCurrentTab}
          onOpenSettings={() => setIsSettingsOpen(true)}
          onShowToast={showToast}
        />

        {/* Tab Content: Studio vs Providers */}
        {currentTab === 'studio' ? (
          <>
            {/* Sessions Column */}
            <SessionsSidebar
              currentProject={currentProject}
              sessions={currentSessions}
              activeSessionId={activeSessionId}
              onSelectSession={setActiveSessionId}
              onOpenNewSessionModal={() => setIsNewSessionOpen(true)}
              isCollapsed={isSessionsCollapsed}
              onToggleCollapse={() => setIsSessionsCollapsed((prev) => !prev)}
            />

            {/* Main Studio View (Split Chat + Code Diff) */}
            <main 
              id="viewStudio" 
              ref={containerRef}
              className="flex-1 flex overflow-hidden bg-black min-w-0 relative"
            >
              {/* Left Pane: Chat */}
              <div 
                style={{ 
                  flex: viewMode === 'diff' 
                    ? '0 0 0%' 
                    : viewMode === 'chat' 
                    ? '1 1 100%' 
                    : `0 0 ${splitPercent}%`,
                  display: viewMode === 'diff' ? 'none' : 'flex'
                }} 
                className="flex-col min-w-0 overflow-hidden"
              >
                <ChatSection
                  session={activeSession}
                  onSendMessage={handleSendMessage}
                  onClearHistory={handleClearHistory}
                  onShowToast={showToast}
                />
              </div>

              {/* Draggable Resizer */}
              {viewMode === 'split' && (
                <Splitter
                  onDragStart={handleSplitterDragStart}
                  onDoubleClick={() => {
                    setSplitPercent(50);
                    showToast('Reset split layout to 50/50');
                  }}
                />
              )}

              {/* Right Pane: Code Diff */}
              <div 
                style={{ 
                  flex: viewMode === 'chat' 
                    ? '0 0 0%' 
                    : viewMode === 'diff' 
                    ? '1 1 100%' 
                    : `0 0 ${100 - splitPercent}%`,
                  display: viewMode === 'chat' ? 'none' : 'flex'
                }} 
                className="flex-col min-w-0 overflow-hidden"
              >
                <DiffSection
                  session={activeSession}
                  onAcceptAndRun={handleAcceptAndRun}
                  onRejectDiff={handleRejectDiff}
                  onCopyDiff={handleCopyDiff}
                />
              </div>
            </main>
          </>
        ) : (
          /* Tab: Providers & API Keys */
          <ProvidersView
            providers={providers}
            clientKeys={clientKeys}
            onOpenAddProviderModal={() => setIsAddProviderOpen(true)}
            onGenerateClientKey={handleGenerateClientKey}
            onRevokeClientKey={handleRevokeClientKey}
            onTestConnection={(name) => showToast(`Key test: ${name} 200 OK`)}
          />
        )}
      </div>

      {/* Status Bar */}
      <StatusBar
        activeSessionTitle={activeSession?.title || 'No active session'}
        activeModel={activeSession?.model || 'Claude 3.7 Sonnet'}
        uiScale={uiScale}
        onSetUiScale={handleSetUiScale}
        onStepUiScale={handleStepUiScale}
      />

      {/* Modals */}
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        onOpenProviders={() => setCurrentTab('providers')}
        onShowToast={showToast}
      />

      <NewSessionModal
        isOpen={isNewSessionOpen}
        onClose={() => setIsNewSessionOpen(false)}
        onCreateSession={handleCreateNewSession}
      />

      <AddProviderModal
        isOpen={isAddProviderOpen}
        onClose={() => setIsAddProviderOpen(false)}
        onSaveProvider={handleSaveProvider}
      />

      {/* Floating Toast */}
      <Toast message={toastMessage} />
    </div>
  );
};

export default App;
