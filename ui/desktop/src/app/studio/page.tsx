import { useState, useEffect, useRef, type SetStateAction } from 'react';
import { useParams } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { type AppWorkspaceMode, AppWorkspaceShell } from '@/components/shell';
import { type OrcaTab, type ResourceTabsState } from '@/components/views/OrcaTabbedContainer';
import type { Session } from '@/types';
import { daemonClient } from '@/api/daemon_client';

const MODE_STORAGE_KEY = 'custos.workspace.mode.v2';
const LAST_SESSION_KEY = 'custos.workspace.last_session_by_lens.v1';

function homeMode(session: Session): AppWorkspaceMode {
  if (session.pack === 'research') return 'research';
  if (session.pack === 'engineering' || session.pack === 'coding') return 'code';
  return 'chat';
}

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
    handleDeleteConversation,
    handleAcceptAndRun,
    handleRejectDiff,
    handleCopyDiff,
    showToast,
    openSettings,
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

  // Each lens remembers its own conversation; the canonical journal still belongs to the daemon session.
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
  const [tabStates, setTabStates] = useState<Record<string, { tabs: OrcaTab[]; activeTabId: string }>>({});
  const tabScope = `${currentProject}:${activeSessionId || 'no-session'}:${mode}`;

  const getDefaultTabForMode = (m: AppWorkspaceMode): { tabs: OrcaTab[]; activeTabId: string } => {
    if (m === 'code') {
      return {
        tabs: [{ id: 'files', type: 'files', title: 'Files', url: 'custos://resource/files' }],
        activeTabId: 'files',
      };
    }
    if (m === 'research') {
      return {
        tabs: [{ id: 'literature', type: 'literature', title: 'Sources', url: 'custos://resource/literature' }],
        activeTabId: 'literature',
      };
    }
    return {
      tabs: [{ id: 'notes', type: 'notes', title: 'Notes', url: 'custos://resource/notes' }],
      activeTabId: 'notes',
    };
  };

  const currentTabState = tabStates[tabScope] ?? getDefaultTabForMode(mode);
  const setTabs = (update: SetStateAction<OrcaTab[]>) => setTabStates((previous) => {
    const current = previous[tabScope] ?? currentTabState;
    const tabs = typeof update === 'function' ? update(current.tabs) : update;
    return { ...previous, [tabScope]: { ...current, tabs } };
  });
  const setActiveTabId = (update: SetStateAction<string>) => setTabStates((previous) => {
    const current = previous[tabScope] ?? currentTabState;
    const activeTabId = typeof update === 'function' ? update(current.activeTabId) : update;
    return { ...previous, [tabScope]: { ...current, activeTabId } };
  });
  const resourceTabs: ResourceTabsState = { ...currentTabState, setTabs, setActiveTabId };
  const openResources = (expanded: boolean) => {
    const newTabId = `tools-${Date.now()}`;
    setTabs((previous) => [...previous, { id: newTabId, type: 'tools', title: 'New Tab', url: 'custos://resources' }]);
    setActiveTabId(newTabId);
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

  const sessionsForMode = (targetMode: AppWorkspaceMode) => currentSessions.filter((session) =>
    homeMode(session) === targetMode || session.openedLenses?.includes(targetMode) || sessionLenses[session.id]?.includes(targetMode)
  );

  const selectSession = (id: string) => {
    setActiveSessionId(id);
    setLastSessionByLens((previous) => ({ ...previous, [`${currentProject}:${mode}`]: id }));
  };

  const switchMode = (targetMode: AppWorkspaceMode) => {
    const candidates = sessionsForMode(targetMode);
    const remembered = lastSessionByLens[`${currentProject}:${targetMode}`];
    setMode(targetMode);
    setActiveSessionId(candidates.some((session) => session.id === remembered)
      ? remembered
      : candidates[0]?.id || '');
  };

  const openFullChatIn = async (targetMode: AppWorkspaceMode, sourceId = activeSessionId) => {
    const source = currentSessions.find((session) => session.id === sourceId);
    if (!source) return;
    try {
      await daemonClient.openSessionInLens(source.sessionId || source.id,
        targetMode === 'chat' ? 'assistant' : targetMode === 'code' ? 'coding' : 'research');
    } catch (error) {
      showToast(`Could not save workbench opening: ${String(error)}`);
      return;
    }
    setSessionLenses((previous) => ({
      ...previous,
      [source.id]: Array.from(new Set([...(previous[source.id] || []), targetMode])),
    }));
    setLastSessionByLens((previous) => ({ ...previous, [`${currentProject}:${targetMode}`]: source.id }));
    setActiveSessionId(source.id);
    setMode(targetMode);
  };

  // Synchronize session ID from URL if present
  useEffect(() => {
    if (sessionId && routeSessionId.current !== sessionId && currentSessions.some((item) => item.id === sessionId)) {
      routeSessionId.current = sessionId;
      const routed = currentSessions.find((item) => item.id === sessionId);
      if (routed && homeMode(routed) !== mode && !routed.openedLenses?.includes(mode) && !sessionLenses[routed.id]?.includes(mode)) {
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
    if (!currentSessions.some((session) => session.id === activeSessionId)) return;
    if (visible.some((session) => session.id === activeSessionId)) return;
    const active = currentSessions.find((session) => session.id === activeSessionId);
    if (active) setMode(homeMode(active));
  }, [activeSessionId, currentSessions, currentProject, mode, sessionLenses, lastSessionByLens]);

  // Global Keyboard Shortcuts
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.key === '1') {
        e.preventDefault();
        switchMode('chat');
      } else if (e.key === '2') {
        e.preventDefault();
        switchMode('code');
      } else if (e.key === '3') {
        e.preventDefault();
        switchMode('research');
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
  }, [showToast, currentProject, currentSessions, sessionLenses, lastSessionByLens]);

  return (
    <div className="h-full w-full bg-canvas overflow-hidden select-none">
      <AppWorkspaceShell
        mode={mode}
        onSwitchMode={switchMode}
        currentProject={currentProject}
        projectNames={Object.keys(projectData)}
        onSelectProject={(p) => {
          setCurrentProject(p);
          const available = (projectData[p] || []).filter((session) =>
            homeMode(session) === mode || sessionLenses[session.id]?.includes(mode)
          );
          const remembered = lastSessionByLens[`${p}:${mode}`];
          setActiveSessionId(available.find((session) => session.id === remembered)?.id || available[0]?.id || '');
        }}
        sessions={sessionsForMode(mode)}
        allSessions={currentSessions}
        activeSessionId={activeSessionId}
        onSelectSession={selectSession}
        onOpenFullChatIn={openFullChatIn}
        onOpenSessionHere={(id) => void openFullChatIn(mode, id)}
        onNewSession={() => setIsNewSessionOpen(true)}
        onSendMessage={(message) => {
          if (!activeSessionId || !sessionsForMode(mode).some((session) => session.id === activeSessionId)) {
            showToast('Create or select a conversation in this workbench first.');
            return;
          }
          void handleSendMessage(message, mode === 'code' ? 'coding' : mode === 'chat' ? 'assistant' : 'research');
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
