import { useEffect, useRef, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { RightFileExplorer } from '@/components/sidebar';
import { ChatSection } from '@/components/chat';
import { DiffSection } from '@/components/diff';
import { Splitter, WorkspaceTabBar, type WorkspacePane, type LayoutMode } from '@/components/shell';
import { 
  EngineeringWorkspace, 
  ResearchWorkspace, 
  AssistantWorkspace,
  BrowserWorkspace,
  MarkdownWorkspace
} from '@/components/workspaces';
import { DelegateTaskModal, TaskDetailsModal } from '@/components/modals';
import type { WorkspacePaneType, WorkbenchLens } from '@/types';

type WorkspaceLayout = {
  panes: WorkspacePane[];
  selected: WorkspacePaneType;
  split: boolean;
  layoutMode: LayoutMode;
  splitPercent: number;
};

const storageKey = 'custos.workspace.layout.v7';
const paneTitles: Record<WorkspacePaneType, string> = {
  chat: 'Conversation',
  engineering: '..entHub/Custos',
  research: 'Research Lab',
  diff: 'Diff Review',
  assistant: 'Fleet Assistant',
  browser: 'Local Browser',
  markdown: 'Notes.md'
};

const defaultLensConfigs: Record<WorkbenchLens, WorkspaceLayout> = {
  copilot: {
    panes: [
      { id: 'chat', title: 'Conversation' },
      { id: 'assistant', title: 'Fleet Assistant' }
    ],
    selected: 'chat',
    split: true,
    layoutMode: 'split-2',
    splitPercent: 42
  },
  coding: {
    panes: [
      { id: 'engineering', title: '..entHub/Custos' },
      { id: 'chat', title: 'Conversation' },
      { id: 'browser', title: 'Local Browser' },
      { id: 'diff', title: 'Diff Review' }
    ],
    selected: 'engineering',
    split: true,
    layoutMode: 'split-2',
    splitPercent: 42
  },
  research: {
    panes: [
      { id: 'research', title: 'Research Lab' },
      { id: 'chat', title: 'Conversation' },
      { id: 'markdown', title: 'Notes.md' }
    ],
    selected: 'research',
    split: true,
    layoutMode: 'split-2',
    splitPercent: 42
  }
};

export function StudioPage() {
  const {
    currentSessions, activeSessionId, setActiveSessionId, activeSession,
    isRightExplorerOpen, toggleRightExplorer,
    setIsSettingsOpen, handleSendMessage, handleClearHistory, handleAcceptAndRun,
    handleRejectDiff, handleCopyDiff, showToast,
  } = useAppContext();
  const { sessionId } = useParams<{ sessionId?: string }>();
  const [activeLens, setActiveLens] = useState<WorkbenchLens>('coding');
  const [isDelegateTaskOpen, setIsDelegateTaskOpen] = useState(false);
  const [isTaskDetailsOpen, setIsTaskDetailsOpen] = useState(false);
  const [sessionTaskOverride, setSessionTaskOverride] = useState<any>(null);

  const activeTask = sessionTaskOverride || activeSession;

  const handleDelegateTask = (taskData: any) => {
    setSessionTaskOverride({
      ...(activeSession || {}),
      id: `task-${Date.now().toString().slice(-4)}`,
      title: taskData.title,
      taskStatus: 'Running',
      model: taskData.model,
      pack: taskData.pack,
      budget: `$${taskData.budgetLimit.toFixed(2)}`,
      strategy: taskData.strategy,
      autonomous: taskData.autonomous
    });
    setIsDelegateTaskOpen(false);
    showToast(`Task "${taskData.title}" delegated to ${taskData.model}!`);
  };

  const [lensConfigs, setLensConfigs] = useState<Record<WorkbenchLens, WorkspaceLayout>>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem(storageKey) || 'null');
      if (saved && saved.coding && saved.research && saved.copilot) return saved;
    } catch {}
    return defaultLensConfigs;
  });

  const layout = lensConfigs[activeLens];
  const setLayout = (updater: React.SetStateAction<WorkspaceLayout>) => {
    setLensConfigs(prev => {
      const nextLayout = typeof updater === 'function' ? updater(prev[activeLens]) : updater;
      return { ...prev, [activeLens]: nextLayout };
    });
  };

  const [isNarrow, setIsNarrow] = useState(() => window.innerWidth < 768);
  const dragging = useRef(false);
  const canvasRef = useRef<HTMLDivElement>(null);

  useEffect(() => { 
    localStorage.setItem(storageKey, JSON.stringify(lensConfigs)); 
  }, [lensConfigs]);

  const handleSwitchLens = (lens: WorkbenchLens) => {
    setActiveLens(lens);
    showToast(`Switched to ${lens.toUpperCase()} lens`);
  };

  useEffect(() => {
    const resize = () => setIsNarrow(window.innerWidth < 768);
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  }, []);

  useEffect(() => {
    if (sessionId && sessionId !== activeSessionId && currentSessions.some((item) => item.id === sessionId)) {
      setActiveSessionId(sessionId);
    }
  }, [sessionId, activeSessionId, currentSessions, setActiveSessionId]);

  const selectPane = (kind: WorkspacePaneType) => {
    setLayout((previous) => ({
      ...previous,
      selected: kind,
      layoutMode: previous.layoutMode === 'single' ? 'single' : previous.layoutMode
    }));
  };

  const openPane = (kind: WorkspacePaneType) => {
    setLayout((previous) => ({
      ...previous,
      panes: previous.panes.some((pane) => pane.id === kind) ? previous.panes : [...previous.panes, { id: kind, title: paneTitles[kind] || kind }],
      selected: kind,
    }));
  };

  const closePane = (kind: WorkspacePaneType) => {
    if (kind === 'chat' && layout.panes.length === 1) return;
    setLayout((previous) => {
      const panes = previous.panes.filter((pane) => pane.id !== kind);
      const selected = previous.selected === kind 
        ? (panes[0]?.id || 'engineering') 
        : previous.selected;
      return { ...previous, panes, selected };
    });
  };

  const changeLayoutMode = (mode: LayoutMode) => {
    setLayout((previous) => ({
      ...previous,
      layoutMode: mode,
      split: mode !== 'single'
    }));
    showToast(`Switched layout: ${mode.toUpperCase()}`);
  };

  const toggleSplit = () => {
    setLayout((previous) => {
      const nextMode: LayoutMode = previous.layoutMode === 'single' ? 'split-2' : 'single';
      return {
        ...previous,
        layoutMode: nextMode,
        split: nextMode !== 'single'
      };
    });
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey)) return;
      const key = event.key.toLowerCase();
      
      // Layout Switching Shortcuts
      if (key === '0') { event.preventDefault(); changeLayoutMode('single'); }
      if (key === '\\') { event.preventDefault(); toggleSplit(); }
      if (key === '3' && event.altKey) { event.preventDefault(); changeLayoutMode('split-3'); }
      if (key === '4' && event.altKey) { event.preventDefault(); changeLayoutMode('split-4'); }
      if (key === '5' && event.altKey) { event.preventDefault(); changeLayoutMode('split-5'); }

      // Tabs Opening Shortcuts
      if (key === '1' && !event.altKey) { event.preventDefault(); openPane('engineering'); }
      if (key === '2' && !event.altKey) { event.preventDefault(); openPane('research'); }
      if (key === '3' && !event.altKey) { event.preventDefault(); selectPane('chat'); }
      if (key === '4' && !event.altKey) { event.preventDefault(); openPane('diff'); }
      if (key === 'b' && event.shiftKey) { event.preventDefault(); openPane('browser'); }
      if (key === 'm' && event.shiftKey) { event.preventDefault(); openPane('markdown'); }
      if (key === 'w' && layout.panes.length > 1) { event.preventDefault(); closePane(layout.selected); }
      if (key === 'e') { event.preventDefault(); toggleRightExplorer(); }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [layout, toggleRightExplorer]);

  useEffect(() => {
    const move = (event: MouseEvent) => {
      if (!dragging.current || !canvasRef.current) return;
      const rect = canvasRef.current.getBoundingClientRect();
      setLayout((previous) => ({ ...previous, splitPercent: Math.min(75, Math.max(25, ((event.clientX - rect.left) / rect.width) * 100)) }));
    };
    const end = () => { dragging.current = false; document.body.classList.remove('resizer-active'); };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', end);
    return () => { window.removeEventListener('mousemove', move); window.removeEventListener('mouseup', end); };
  }, []);

  // Render a specific workspace tab
  const renderWorkspaceTab = (tab: WorkspacePaneType) => {
    switch (tab) {
      case 'engineering':
        return (
          <EngineeringWorkspace 
            session={activeSession} 
            onAcceptAndRun={handleAcceptAndRun} 
            onRejectDiff={handleRejectDiff} 
            onCopyDiff={handleCopyDiff} 
          />
        );
      case 'research':
        return <ResearchWorkspace />;
      case 'browser':
        return <BrowserWorkspace />;
      case 'markdown':
        return <MarkdownWorkspace />;
      case 'assistant':
        return <AssistantWorkspace />;
      case 'diff':
        return (
          <DiffSection 
            session={activeSession} 
            onAcceptAndRun={handleAcceptAndRun} 
            onRejectDiff={handleRejectDiff} 
            onCopyDiff={handleCopyDiff} 
          />
        );
      case 'chat':
      default:
        return (
          <ChatSection 
            session={activeSession} 
            onSendMessage={handleSendMessage} 
            onClearHistory={handleClearHistory} 
            onShowToast={showToast} 
          />
        );
    }
  };

  const chat = renderWorkspaceTab('chat');
  const activeMain = renderWorkspaceTab(layout.selected);

  // -------------------------------------------------------------
  // RENDER DYNAMIC MULTI-PANE CANVAS (1, 2, 3, 4, 5 Panes)
  // -------------------------------------------------------------
  const renderCanvasLayout = () => {
    if (isNarrow || layout.layoutMode === 'single') {
      return (
        <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
          {activeMain}
        </div>
      );
    }

    // MODE 2: Split 2 (Side by Side Resizable)
    if (layout.layoutMode === 'split-2') {
      const rightTab = layout.selected === 'chat' ? 'engineering' : layout.selected;
      return (
        <>
          <div className="flex min-w-0 flex-col overflow-hidden" style={{ flexBasis: `${layout.splitPercent}%` }}>
            {chat}
          </div>
          <Splitter 
            onDragStart={() => { dragging.current = true; document.body.classList.add('resizer-active'); }} 
            onDoubleClick={() => setLayout((previous) => ({ ...previous, splitPercent: 42 }))} 
          />
          <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
            {renderWorkspaceTab(rightTab)}
          </div>
        </>
      );
    }

    // MODE 3: Split 3 (Triple Columns: Chat + Coding + Companion)
    if (layout.layoutMode === 'split-3') {
      const thirdTab = layout.selected !== 'chat' && layout.selected !== 'engineering' ? layout.selected : 'browser';
      return (
        <div className="grid grid-cols-3 h-full w-full divide-x divide-[#1c2130]">
          <div className="flex flex-col min-w-0 overflow-hidden">{chat}</div>
          <div className="flex flex-col min-w-0 overflow-hidden">{renderWorkspaceTab('engineering')}</div>
          <div className="flex flex-col min-w-0 overflow-hidden">{renderWorkspaceTab(thirdTab)}</div>
        </div>
      );
    }

    // MODE 4: Split 4 (2x2 Grid)
    if (layout.layoutMode === 'split-4') {
      return (
        <div className="grid grid-cols-2 grid-rows-2 h-full w-full divide-x divide-y divide-[#1c2130]">
          {/* Cell 1: Chat */}
          <div className="flex flex-col min-w-0 min-h-0 overflow-hidden relative">
            <div className="h-6 px-2 bg-[#090b10] border-b border-[#1c2130] flex items-center justify-between text-[10.5px] font-mono text-neutral-400">
              <span className="text-white font-medium">1 • AI Co-Pilot</span>
              <span className="text-neutral-500">Claude/Codex</span>
            </div>
            <div className="flex-1 min-h-0">{chat}</div>
          </div>

          {/* Cell 2: Coding IDE */}
          <div className="flex flex-col min-w-0 min-h-0 overflow-hidden relative">
            <div className="h-6 px-2 bg-[#090b10] border-b border-[#1c2130] flex items-center justify-between text-[10.5px] font-mono text-neutral-400">
              <span className="text-emerald-400 font-medium">2 • Codex IDE</span>
              <span className="text-neutral-500">Monaco & Terminal</span>
            </div>
            <div className="flex-1 min-h-0">{renderWorkspaceTab('engineering')}</div>
          </div>

          {/* Cell 3: Browser */}
          <div className="flex flex-col min-w-0 min-h-0 overflow-hidden relative">
            <div className="h-6 px-2 bg-[#090b10] border-b border-[#1c2130] flex items-center justify-between text-[10.5px] font-mono text-neutral-400">
              <span className="text-cyan-400 font-medium">3 • Local Browser</span>
              <span className="text-neutral-500">Preview & HMR</span>
            </div>
            <div className="flex-1 min-h-0">{renderWorkspaceTab('browser')}</div>
          </div>

          {/* Cell 4: Markdown Notes or Diff */}
          <div className="flex flex-col min-w-0 min-h-0 overflow-hidden relative">
            <div className="h-6 px-2 bg-[#090b10] border-b border-[#1c2130] flex items-center justify-between text-[10.5px] font-mono text-neutral-400">
              <span className="text-amber-400 font-medium">4 • Notes & Scratchpad</span>
              <span className="text-neutral-500">Live Markdown</span>
            </div>
            <div className="flex-1 min-h-0">{renderWorkspaceTab('markdown')}</div>
          </div>
        </div>
      );
    }

    // MODE 5: Split 5 (Master 50% + 4 Tiles 50%)
    if (layout.layoutMode === 'split-5') {
      return (
        <div className="flex h-full w-full divide-x divide-[#1c2130] overflow-hidden">
          {/* Master Pane on Left (50% Width) */}
          <div className="w-1/2 flex flex-col min-w-0 overflow-hidden">
            <div className="h-6 px-2 bg-[#090b10] border-b border-[#1c2130] flex items-center justify-between text-[10.5px] font-mono text-neutral-400">
              <span className="text-emerald-400 font-semibold">MASTER • Codex IDE & Terminal</span>
              <span className="text-neutral-500">Active Worktree</span>
            </div>
            <div className="flex-1 min-h-0">{renderWorkspaceTab('engineering')}</div>
          </div>

          {/* 4 Quadrants on Right (50% Width in 2x2 Grid) */}
          <div className="w-1/2 grid grid-cols-2 grid-rows-2 h-full divide-x divide-y divide-[#1c2130] overflow-hidden">
            {/* Tile 1: AI Chat */}
            <div className="flex flex-col min-w-0 min-h-0 overflow-hidden">
              <div className="h-5 px-2 bg-[#090b10] border-b border-[#1c2130] text-[10px] font-mono text-white flex items-center justify-between">
                <span>AI Assistant</span>
                <span className="text-[9px] text-neutral-500">Claude/Codex</span>
              </div>
              <div className="flex-1 min-h-0">{chat}</div>
            </div>

            {/* Tile 2: Research */}
            <div className="flex flex-col min-w-0 min-h-0 overflow-hidden">
              <div className="h-5 px-2 bg-[#090b10] border-b border-[#1c2130] text-[10px] font-mono text-purple-300 flex items-center justify-between">
                <span>Research Lab</span>
                <span className="text-[9px] text-neutral-500">ArXiv Papers</span>
              </div>
              <div className="flex-1 min-h-0">{renderWorkspaceTab('research')}</div>
            </div>

            {/* Tile 3: Browser */}
            <div className="flex flex-col min-w-0 min-h-0 overflow-hidden">
              <div className="h-5 px-2 bg-[#090b10] border-b border-[#1c2130] text-[10px] font-mono text-cyan-300 flex items-center justify-between">
                <span>Local Browser</span>
                <span className="text-[9px] text-neutral-500">Port 1420</span>
              </div>
              <div className="flex-1 min-h-0">{renderWorkspaceTab('browser')}</div>
            </div>

            {/* Tile 4: Diff Review */}
            <div className="flex flex-col min-w-0 min-h-0 overflow-hidden">
              <div className="h-5 px-2 bg-[#090b10] border-b border-[#1c2130] text-[10px] font-mono text-blue-300 flex items-center justify-between">
                <span>Diff Review</span>
                <span className="text-[9px] text-neutral-500">Git (+34 -2)</span>
              </div>
              <div className="flex-1 min-h-0">{renderWorkspaceTab('diff')}</div>
            </div>
          </div>
        </div>
      );
    }

    return null;
  };

  return (
    <div className="flex h-full min-w-0 flex-1 overflow-hidden">
      <main className="flex min-w-0 flex-1 flex-col bg-canvas">
        <WorkspaceTabBar 
          activeLens={activeLens}
          activeTask={activeTask}
          onSwitchLens={handleSwitchLens}
          onOpenDelegateModal={() => setIsDelegateTaskOpen(true)}
          onOpenTaskDetailsModal={() => setIsTaskDetailsOpen(true)}
          panes={layout.panes} 
          selected={layout.selected} 
          split={layout.split} 
          layoutMode={layout.layoutMode}
          onChangeLayoutMode={changeLayoutMode}
          inspectorOpen={isRightExplorerOpen}
          onSelect={selectPane} 
          onClose={closePane} 
          onAdd={openPane} 
          onToggleSplit={toggleSplit} 
          onToggleInspector={toggleRightExplorer} 
          onOpenSettings={() => setIsSettingsOpen(true)}
          onSelectAgentModel={(model) => showToast(`Selected AI Assistant: ${model}`)}
        />
        {activeSession?.source === 'demo' && (
          <div className="shrink-0 border-b border-brand-amber/30 bg-brand-amber/10 px-3 py-1 text-[11px] text-brand-amber" role="status">
            Demo conversation. Example messages and changes are not live results.
          </div>
        )}
        <div ref={canvasRef} className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
          {renderCanvasLayout()}
          <RightFileExplorer 
            isOpen={isRightExplorerOpen} 
            onClose={toggleRightExplorer} 
          />
        </div>
      </main>

      {/* Delegate Task Modal (Assist Mode -> Task Transition) */}
      <DelegateTaskModal
        isOpen={isDelegateTaskOpen}
        onClose={() => setIsDelegateTaskOpen(false)}
        onSubmit={handleDelegateTask}
        activeSessionName={activeSession?.title || 'Active Session'}
      />

      {/* Autonomous Task Details Modal */}
      <TaskDetailsModal
        isOpen={isTaskDetailsOpen}
        onClose={() => setIsTaskDetailsOpen(false)}
        task={activeTask}
      />
    </div>
  );
}

export default StudioPage;
