import { useState, useEffect, type SetStateAction } from 'react';
import { useParams } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { AppHeader, type AppWorkspaceMode } from '@/components/shell/AppHeader';
import { ClaudeChatView } from '@/components/views/ClaudeChatView';
import { CodexOrcaView } from '@/components/views/CodexOrcaView';
import { ResearchView } from '@/components/views/ResearchView';
import { type OrcaTab, type OrcaTabId, type ResourceTabsState } from '@/components/views/OrcaTabbedContainer';

const MODE_STORAGE_KEY = 'custos.workspace.mode.v2';

export function StudioPage() {
  const {
    currentProject,
    projectData,
    setCurrentProject,
    currentSessions,
    activeSessionId,
    setActiveSessionId,
    setIsNewSessionOpen,
    handleSendMessage,
    handleClearHistory,
    handleAcceptAndRun,
    handleRejectDiff,
    handleCopyDiff,
    showToast,
    openSettings,
  } = useAppContext();

  const { sessionId } = useParams<{ sessionId?: string }>();

  // Workbench lens state is presentation-only; it does not change the active session.
  const [mode, setMode] = useState<AppWorkspaceMode>(() => {
    try {
      const saved = localStorage.getItem(MODE_STORAGE_KEY);
      if (saved === 'chat' || saved === 'code' || saved === 'research') return saved;
    } catch { }
    return 'code'; // Default to Code mode as showcased in screenshot 2
  });

  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [isWebTabOpen, setIsWebTabOpen] = useState(false);
  const [isWebTabExpanded, setIsWebTabExpanded] = useState(false);
  const [tabStates, setTabStates] = useState<Record<string, { tabs: OrcaTab[]; activeTabId: OrcaTabId }>>({});
  const tabScope = `${currentProject}:${activeSessionId || 'no-session'}`;
  const currentTabState = tabStates[tabScope] ?? {
    tabs: [{ id: 'tools' as const, title: 'Resources', url: 'custos://resources' }],
    activeTabId: 'tools' as const,
  };
  const setTabs = (update: SetStateAction<OrcaTab[]>) => setTabStates((previous) => {
    const current = previous[tabScope] ?? currentTabState;
    const tabs = typeof update === 'function' ? update(current.tabs) : update;
    return { ...previous, [tabScope]: { ...current, tabs } };
  });
  const setActiveTabId = (update: SetStateAction<OrcaTabId>) => setTabStates((previous) => {
    const current = previous[tabScope] ?? currentTabState;
    const activeTabId = typeof update === 'function' ? update(current.activeTabId) : update;
    return { ...previous, [tabScope]: { ...current, activeTabId } };
  });
  const resourceTabs: ResourceTabsState = { ...currentTabState, setTabs, setActiveTabId };
  const openResources = (expanded: boolean) => {
    setTabs((previous) => previous.some((tab) => tab.id === 'tools') ? previous : [...previous, { id: 'tools', title: 'Resources', url: 'custos://resources' }]);
    setActiveTabId('tools');
    setIsWebTabExpanded(expanded);
    setIsWebTabOpen(true);
  };

  useEffect(() => {
    try {
      localStorage.setItem(MODE_STORAGE_KEY, mode);
    } catch { }
  }, [mode]);

  // Synchronize session ID from URL if present
  useEffect(() => {
    if (sessionId && sessionId !== activeSessionId && currentSessions.some((item) => item.id === sessionId)) {
      setActiveSessionId(sessionId);
    }
  }, [sessionId, activeSessionId, currentSessions, setActiveSessionId]);

  // Global Keyboard Shortcuts (⌘1 for Chat, ⌘2 for Code)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.key === '1') {
        e.preventDefault();
        setMode('chat');
        showToast('Opened Copilot view (⌘1)');
      } else if (e.key === '2') {
        e.preventDefault();
        setMode('code');
        showToast('Opened Coding view (⌘2)');
      } else if (e.key === '3') {
        e.preventDefault();
        setMode('research');
        showToast('Opened Research view (⌘3)');
      } else if (e.key === 'f' && e.shiftKey && e.metaKey) {
        e.preventDefault();
        setIsWebTabOpen(true);
        setIsWebTabExpanded(true);
      } else if (e.key === 'b' && e.shiftKey && e.metaKey) {
        e.preventDefault();
        setIsWebTabOpen(true);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [showToast]);

  return (
    <div className="flex flex-col h-full w-full bg-[#0d1117] overflow-hidden select-none">
      {/* ─────────────────────────────────────────────────────────────
          UNIFIED TOP BAR WITH THE 2 MODE ICONS: [ 💬 Chat ] & [ </> Code ]
      ───────────────────────────────────────────────────────────── */}
      <AppHeader
        mode={mode}
        workspaceTitle={currentSessions.find((item) => item.id === activeSessionId)?.title}
        onSwitchMode={(newMode) => setMode(newMode)}
        isSidebarCollapsed={isSidebarCollapsed}
        onToggleSidebar={() => setIsSidebarCollapsed((prev) => !prev)}
        onShowToast={showToast}
        onNewTab={() => openResources(false)}
        onNewTabFullView={() => openResources(true)}
        isWebTabOpen={isWebTabOpen}
        webTabCount={currentTabState.tabs.length}

        onToggleWebTab={() => setIsWebTabOpen((prev) => !prev)}
      />

      {/* ─────────────────────────────────────────────────────────────
          MAIN WORKSPACE BODY
      ───────────────────────────────────────────────────────────── */}
      <div className="flex-1 overflow-hidden min-h-0 relative">
        {mode === 'chat' && (
          /* SCREENSHOT 1: CLAUDE DESKTOP CHAT & COWORK */
          <ClaudeChatView
            currentProject={currentProject}
            projectNames={Object.keys(projectData)}
            onSelectProject={(p) => {
              setCurrentProject(p);
              const first = (projectData[p] || [])[0];
              if (first) setActiveSessionId(first.id);
            }}
            sessions={currentSessions}
            activeSessionId={activeSessionId}
            onSelectSession={(id) => setActiveSessionId(id)}
            onNewSession={() => setIsNewSessionOpen(true)}
            onSendMessage={handleSendMessage}
            onClearHistory={handleClearHistory}
            onShowToast={showToast}
            onOpenSettings={() => openSettings('general')}
            onSwitchMode={setMode}
            isSidebarCollapsed={isSidebarCollapsed}
            isWebTabOpen={isWebTabOpen}


            isWebTabExpanded={isWebTabExpanded}
            onToggleExpandWebTab={() => setIsWebTabExpanded(!isWebTabExpanded)}
            onToggleWebTab={() => setIsWebTabOpen((prev) => !prev)}
            resourceTabs={resourceTabs}
          />
        )}

        {mode === 'code' && (
          /* SCREENSHOT 2: CODEX / ORCA ADE 3-COLUMN WORKSPACE */
          <CodexOrcaView
            currentProject={currentProject}
            projectNames={Object.keys(projectData)}
            onSelectProject={(p) => {
              setCurrentProject(p);
              const first = (projectData[p] || [])[0];
              if (first) setActiveSessionId(first.id);
            }}
            sessions={currentSessions}
            activeSessionId={activeSessionId}
            onSelectSession={(id) => setActiveSessionId(id)}
            onNewSession={() => setIsNewSessionOpen(true)}
            onSendMessage={handleSendMessage}
            onClearHistory={handleClearHistory}
            onAcceptAndRun={handleAcceptAndRun}
            onRejectDiff={handleRejectDiff}
            onCopyDiff={handleCopyDiff}
            onShowToast={showToast}
            onOpenSettings={() => openSettings('general')}
            onSwitchMode={setMode}
            isSidebarCollapsed={isSidebarCollapsed}
            isWebTabOpen={isWebTabOpen}
            isWebTabExpanded={isWebTabExpanded}
            onToggleExpandWebTab={() => setIsWebTabExpanded(!isWebTabExpanded)}
            onToggleWebTab={() => setIsWebTabOpen((prev) => !prev)}
            resourceTabs={resourceTabs}
          />
        )}

        {mode === 'research' && (
          /* RESEARCH: CLAUDE SCIENCE LAB */
          <ResearchView
            onOpenResourcePane={() => setIsWebTabOpen(true)}
            currentProject={currentProject}
            projectNames={Object.keys(projectData)}
            onSelectProject={(p) => {
              setCurrentProject(p);
              const first = (projectData[p] || [])[0];
              if (first) setActiveSessionId(first.id);
            }}
            sessions={currentSessions}
            activeSessionId={activeSessionId}
            onSelectSession={(id) => setActiveSessionId(id)}
            onNewSession={() => setIsNewSessionOpen(true)}
            onSendMessage={handleSendMessage}
            onClearHistory={handleClearHistory}
            onShowToast={showToast}
            onOpenSettings={() => openSettings('general')}
            onSwitchMode={setMode}
            isSidebarCollapsed={isSidebarCollapsed}
            isWebTabOpen={isWebTabOpen}
            isWebTabExpanded={isWebTabExpanded}
            onToggleExpandWebTab={() => setIsWebTabExpanded(!isWebTabExpanded)}
            onToggleWebTab={() => setIsWebTabOpen((prev) => !prev)}
            resourceTabs={resourceTabs}
          />
        )}
      </div>
    </div>
  );
}

export default StudioPage;
