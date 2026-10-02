import React, { useRef, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useAppContext } from '../../context/AppContext';
import { SessionsSidebar } from '../../components/SessionsSidebar';
import { ChatSection } from '../../components/ChatSection';
import { DiffSection } from '../../components/DiffSection';
import { Splitter } from '../../components/Splitter';

export const StudioPage: React.FC = () => {
  const {
    currentProject,
    currentSessions,
    activeSessionId,
    setActiveSessionId,
    activeSession,
    isSessionsCollapsed,
    setIsSessionsCollapsed,
    viewMode,
    setViewMode,
    splitPercent,
    setSplitPercent,
    isDragging,
    setIsDragging,
    setIsNewSessionOpen,
    handleSendMessage,
    handleClearHistory,
    handleAcceptAndRun,
    handleRejectDiff,
    handleCopyDiff,
    showToast
  } = useAppContext();

  const { sessionId } = useParams<{ sessionId?: string }>();
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement>(null);

  // Sync activeSessionId with route parameter if provided
  useEffect(() => {
    if (sessionId && sessionId !== activeSessionId) {
      const match = currentSessions.find((s) => s.id === sessionId);
      if (match) {
        setActiveSessionId(sessionId);
      }
    }
  }, [sessionId, currentSessions, activeSessionId, setActiveSessionId]);

  const handleSelectSession = (id: string) => {
    setActiveSessionId(id);
    navigate(`/studio/${id}`);
  };

  // Draggable Splitter
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
  }, [isDragging, setIsDragging, setSplitPercent]);

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
  }, [viewMode, setViewMode]);

  return (
    <div className="flex-1 flex overflow-hidden min-w-0 h-full relative">
      {/* Sessions Column */}
      <SessionsSidebar
        currentProject={currentProject}
        sessions={currentSessions}
        activeSessionId={activeSessionId}
        onSelectSession={handleSelectSession}
        onOpenNewSessionModal={() => setIsNewSessionOpen(true)}
        isCollapsed={isSessionsCollapsed}
        onToggleCollapse={() => setIsSessionsCollapsed((prev) => !prev)}
      />

      {/* Main Studio View (Split Chat + Code Diff) */}
      <main
        id="viewStudio"
        ref={containerRef}
        className="flex-1 flex overflow-hidden bg-canvas min-w-0 relative"
      >
        {/* Left Pane: Chat */}
        <div
          style={{
            flex:
              viewMode === 'diff'
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
            flex:
              viewMode === 'chat'
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
    </div>
  );
};

export default StudioPage;
