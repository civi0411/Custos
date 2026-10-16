import { useState, useEffect, useRef } from 'react';
import { useParams } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { type ActiveWorkbench, AppWorkspaceShell } from '@/components/shell';
import type { Session } from '@/types';
import { daemonClient } from '@/api/daemon_client';

const MODE_STORAGE_KEY = 'custos.workspace.mode.v2';
const LAST_SESSION_KEY = 'custos.workspace.last_session_by_lens.v1';
const SIDEBAR_VIEW_KEY = 'custos.sidebar.list_mode_by_lens.v1';

function homeMode(session: Session): ActiveWorkbench {
  if (!session.taskId && session.openedLenses?.length) {
    const home = session.openedLenses[0];
    if (home === 'research') return 'research';
    if (home === 'code') return 'engineering';
    return 'copilot';
  }
  if (session.pack === 'research') return 'research';
  if (session.pack === 'engineering' || session.pack === 'coding') return 'engineering';
  return 'copilot';
}

export function StudioPage() {
  const {
    currentProject,
    projectData,
    setCurrentProject,
    currentSessions,
    tasks,
    activeSessionId,
    setActiveSessionId,
    workspaces,
    activeWorkspaceId,
    setActiveWorkspaceId,
    activeWorkspace,
    refreshWorkspaces,
    setIsNewSessionOpen,
    handleSendMessage,
    handleClearHistory,
    handleDeleteConversation,
    handleAcceptAndRun,
    handleRejectDiff,
    handleCopyDiff,
    showToast,
    openSettings,
    activeWorkbench: mode,
    setActiveWorkbench: setMode,
    isSidebarCollapsed,
    setIsSidebarCollapsed,
    isWebTabOpen,
    setIsWebTabOpen,
    resourceTabs,
    isWebTabExpanded,
    setIsWebTabExpanded,
  } = useAppContext();

  const { sessionId } = useParams<{ sessionId?: string }>();
  const routeSessionId = useRef<string | undefined>(undefined);
  const [sessionLenses, setSessionLenses] = useState<Record<string, string[]>>({});
  const [lastSessionByLens, setLastSessionByLens] = useState<Record<string, string>>(() => {
    try {
      const value = JSON.parse(localStorage.getItem(LAST_SESSION_KEY) || '{}');
      return value && typeof value === 'object' && !Array.isArray(value) ? value : {};
    } catch { return {}; }
  });

  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [sidebarViewByLens, setSidebarViewByLens] = useState<Record<string, 'chats' | 'tasks'>>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem(SIDEBAR_VIEW_KEY) || '{}');
      if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return {};
      return Object.fromEntries(['copilot', 'engineering', 'research'].map((lens) => [
        lens,
        saved[lens] === 'tasks' ? 'tasks' : 'chats',
      ]));
    } catch { return {}; }
  });
  const sidebarView = sidebarViewByLens[mode] || 'chats';
  const setSidebarView = (lens: ActiveWorkbench, view: 'chats' | 'tasks') => {
    const next = { ...sidebarViewByLens, [lens]: view };
    setSidebarViewByLens(next);
    try { localStorage.setItem(SIDEBAR_VIEW_KEY, JSON.stringify(next)); } catch { }
  };
  const openResources = (expanded: boolean) => {
    const newTabId = `tools-${Date.now()}`;
    resourceTabs.setTabs((previous) => [...previous, { id: newTabId, type: 'tools', title: 'New Tab', url: 'custos://resources' }]);
    resourceTabs.setActiveTabId(newTabId);
    setIsWebTabExpanded(expanded);
    setIsWebTabOpen(true);
  };

  useEffect(() => {
    try {
      localStorage.setItem(MODE_STORAGE_KEY, mode);
    } catch { }
  }, [mode]);

  useEffect(() => {
    try { localStorage.setItem(LAST_SESSION_KEY, JSON.stringify(lastSessionByLens)); } catch { }
  }, [lastSessionByLens]);

  // Handle sidebar view switching from ActivityRail or other triggers
  useEffect(() => {
    const handleSwitchSidebar = (e: Event) => {
      const customEvent = e as CustomEvent<'chats' | 'tasks'>;
      if (customEvent.detail) {
        setSidebarView(mode, customEvent.detail);
        if (isSidebarCollapsed) {
          setIsSidebarCollapsed(false);
        }
      }
    };
    window.addEventListener('custos:switch-sidebar-view', handleSwitchSidebar);
    return () => window.removeEventListener('custos:switch-sidebar-view', handleSwitchSidebar);
  }, [mode, isSidebarCollapsed, isSidebarCollapsed, setIsSidebarCollapsed]);

  const sessionsForMode = (targetMode: ActiveWorkbench) => currentSessions.filter((session) =>
    homeMode(session) === targetMode || session.openedLenses?.includes(targetMode as any) || sessionLenses[session.id]?.includes(targetMode)
  );

  const selectSession = (id: string) => {
    setActiveTaskId(null);
    setSidebarView(mode, 'chats');
    setActiveSessionId(id);
    setLastSessionByLens((previous) => ({ ...previous, [`${currentProject}:${mode}`]: id }));
  };

  const switchMode = (targetMode: ActiveWorkbench) => {
    setActiveTaskId(null);
    const candidates = sessionsForMode(targetMode);
    const remembered = lastSessionByLens[`${currentProject}:${targetMode}`];
    setMode(targetMode);

    let nextSessionId = '';
    if (candidates.some((session) => session.id === remembered)) {
      nextSessionId = remembered;
    } else if (candidates.length > 0) {
      nextSessionId = candidates[0].id;
    }

    setActiveSessionId(nextSessionId);
    if (nextSessionId) {
      setLastSessionByLens((previous) => ({ ...previous, [`${currentProject}:${targetMode}`]: nextSessionId }));
    }

    // Copilot starts in spacious Chat mode (right pane closed); Engineering & Research open right pane
    if (targetMode === 'copilot') {
      setIsWebTabOpen(false);
    } else {
      setIsWebTabOpen(true);
    }
  };

  const openFullChatIn = async (targetMode: ActiveWorkbench, sourceId = activeSessionId) => {
    const source = currentSessions.find((session) => session.id === sourceId);
    if (!source) return;
    try {
      await daemonClient.openSessionInLens(source.sessionId || source.id,
        targetMode === 'copilot' ? 'assistant' : targetMode === 'engineering' ? 'coding' : 'research');
    } catch (error) {
      showToast(`Could not save workbench opening: ${String(error)}`);
      return;
    }
    setSessionLenses((previous) => ({
      ...previous,
      [source.id]: Array.from(new Set([...(previous[source.id] || []), targetMode])),
    }));
    setLastSessionByLens((previous) => ({ ...previous, [`${currentProject}:${targetMode}`]: source.id }));
    setActiveTaskId(null);
    setSidebarView(targetMode, 'chats');
    setActiveSessionId(source.id);
    setMode(targetMode);
  };

  // Synchronize session ID from URL if present
  useEffect(() => {
    if (sessionId && routeSessionId.current !== sessionId && currentSessions.some((item) => item.id === sessionId)) {
      routeSessionId.current = sessionId;
      const routed = currentSessions.find((item) => item.id === sessionId);
      if (routed && homeMode(routed) !== mode && !routed.openedLenses?.includes(mode as any) && !sessionLenses[routed.id]?.includes(mode)) {
        setMode(homeMode(routed));
      }
      setActiveSessionId(sessionId);
    }
  }, [sessionId, currentSessions, mode, sessionLenses, setActiveSessionId]);

  useEffect(() => {
    const visible = sessionsForMode(mode);
    if (!activeSessionId) {
      const remembered = lastSessionByLens[`${currentProject}:${mode}`];
      const target = visible.find((session) => session.id === remembered) || visible[0];
      if (target) setActiveSessionId(target.id);
      return;
    }
    if (!visible.some((session) => session.id === activeSessionId)) {
      const remembered = lastSessionByLens[`${currentProject}:${mode}`];
      const target = visible.find((session) => session.id === remembered) || visible[0];
      setActiveSessionId(target ? target.id : '');
    }
  }, [activeSessionId, currentSessions, currentProject, mode, sessionLenses, lastSessionByLens]);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // Option/Alt + 1/2/3 to switch workbench mode without colliding with Cmd+1/2/3
      if (e.altKey && !e.metaKey && !e.ctrlKey) {
        if (e.key === '1') {
          e.preventDefault();
          switchMode('copilot');
        } else if (e.key === '2') {
          e.preventDefault();
          switchMode('engineering');
        } else if (e.key === '3') {
          e.preventDefault();
          switchMode('research');
        }
      }
      if (e.key === 'f' && e.shiftKey && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setIsWebTabOpen(true);
        setIsWebTabExpanded(true);
      } else if (e.key === 'b' && e.shiftKey && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setIsWebTabOpen(true);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [showToast, currentProject, currentSessions, sessionLenses, lastSessionByLens, setIsWebTabOpen, setIsWebTabExpanded]);

  const activeModeSessions = sessionsForMode(mode);
  const displaySessions = activeModeSessions;

  return (
    <div className="h-full w-full bg-canvas overflow-hidden select-none">
      <AppWorkspaceShell
        mode={mode}
        onSwitchMode={switchMode}
        currentProject={currentProject}
        projectNames={Object.keys(projectData)}
        onSelectProject={(p) => {
          setActiveTaskId(null);
          setCurrentProject(p);
          const available = (projectData[p] || []).filter((session) =>
            homeMode(session) === mode || sessionLenses[session.id]?.includes(mode)
          );
          const remembered = lastSessionByLens[`${p}:${mode}`];
          setActiveSessionId(available.find((session) => session.id === remembered)?.id || available[0]?.id || '');
        }}
        activeWorkspaceId={activeWorkspaceId}
        onSelectWorkspace={setActiveWorkspaceId}
        activeWorkspace={activeWorkspace}
        workspaces={workspaces}
        refreshWorkspaces={refreshWorkspaces}
        sessions={displaySessions}
        allSessions={currentSessions}
        activeSessionId={activeSessionId}
        onSelectSession={selectSession}
        selectedTaskId={activeTaskId}
        onSelectTask={(id) => { setSidebarView(mode, 'tasks'); setActiveTaskId(id); }}
        onClearTask={() => setActiveTaskId(null)}
        sidebarView={sidebarView}
        onChangeSidebarView={(view) => setSidebarView(mode, view)}
        tasks={tasks}
        onOpenFullChatIn={openFullChatIn}
        onOpenSessionHere={(id) => void openFullChatIn(mode, id)}
        onNewSession={() => { setSidebarView(mode, 'chats'); setActiveTaskId(null); setIsNewSessionOpen(true); }}
        onSendMessage={(message) => {
          if (!activeSessionId) {
            showToast('Select or create a conversation first.');
            return;
          }
          if (activeSessionId) {
            setSessionLenses((previous) => ({
              ...previous,
              [activeSessionId]: Array.from(new Set([...(previous[activeSessionId] || []), mode])),
            }));
          }
          void handleSendMessage(message, mode === 'engineering' ? 'coding' : mode === 'copilot' ? 'assistant' : 'research');
        }}
        onClearHistory={handleClearHistory}
        onDeleteConversation={handleDeleteConversation}
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
