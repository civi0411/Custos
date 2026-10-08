import { useState, useEffect, type SetStateAction } from 'react';
import { useParams } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { type AppWorkspaceMode, AppWorkspaceShell } from '@/components/shell';
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
    <div className="h-full w-full bg-canvas overflow-hidden select-none">
      <AppWorkspaceShell
        mode={mode}
        onSwitchMode={(newMode) => setMode(newMode)}
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
        isSidebarCollapsed={isSidebarCollapsed}
        onToggleSidebar={() => setIsSidebarCollapsed((prev) => !prev)}
        resourceTabs={resourceTabs}
        isWebTabOpen={isWebTabOpen}
        isWebTabExpanded={isWebTabExpanded}
        onToggleWebTab={() => setIsWebTabOpen((prev) => !prev)}
        onToggleExpandWebTab={() => setIsWebTabExpanded(!isWebTabExpanded)}
        openResources={openResources}
      />
    </div>
  );
}

export default StudioPage;
