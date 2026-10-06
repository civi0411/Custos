import { useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { AppHeader, type AppWorkspaceMode } from '@/components/shell/AppHeader';
import { ClaudeChatView } from '@/components/views/ClaudeChatView';
import { CodexOrcaView } from '@/components/views/CodexOrcaView';
import { ResearchView } from '@/components/views/ResearchView';

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

  // Mode switcher: 'chat' (Claude Desktop) vs 'code' (Codex / Orca ADE 3-Column)
  const [mode, setMode] = useState<AppWorkspaceMode>(() => {
    try {
      const saved = localStorage.getItem(MODE_STORAGE_KEY);
      if (saved === 'chat' || saved === 'code') return saved;
    } catch {}
    return 'code'; // Default to Code mode as showcased in screenshot 2
  });

  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [isWebTabOpen, setIsWebTabOpen] = useState(false);
  const [isWebTabExpanded, setIsWebTabExpanded] = useState(false);
  const [webTabCount, setWebTabCount] = useState(0);

  useEffect(() => {
    try {
      localStorage.setItem(MODE_STORAGE_KEY, mode);
    } catch {}
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
        showToast('Switched to Claude Chat & Cowork mode (⌘1)');
      } else if (e.key === '2') {
        e.preventDefault();
        setMode('code');
        showToast('Switched to Codex & Orca ADE mode (⌘2)');
      } else if (e.key === '3') {
        e.preventDefault();
        setMode('research');
        showToast('Switched to Claude Science Lab (⌘3)');
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
        onSwitchMode={(newMode) => setMode(newMode)}
        isSidebarCollapsed={isSidebarCollapsed}
        onToggleSidebar={() => setIsSidebarCollapsed((prev) => !prev)}
        onShowToast={showToast}
        onNewTab={() => {
          setIsWebTabOpen(true);
          setIsWebTabExpanded(false);
        }}
        onNewTabFullView={() => {
          setIsWebTabOpen(true);
          setIsWebTabExpanded(true);
        }}
        isWebTabOpen={isWebTabOpen}
        webTabCount={webTabCount}
        
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
            onWebTabCountChange={setWebTabCount}
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
            onWebTabCountChange={setWebTabCount}
          />
        )}

        {mode === 'research' && (
          /* RESEARCH: CLAUDE SCIENCE LAB */
          <ResearchView
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
            onWebTabCountChange={setWebTabCount}
          />
        )}
      </div>
    </div>
  );
}

export default StudioPage;
