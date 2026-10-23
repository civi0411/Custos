import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Plus } from 'lucide-react';
import { appendConversationDraft, readConversationDraft } from '@/components/chat/ConversationPane';
import { Session } from '@/types';
import { OrcaTabbedContainer, type ResourceTabsState } from '@/components/workbenches/OrcaTabbedContainer';
import { WorkspaceSidebar, type SidebarTool } from '@/components/shell/WorkspaceSidebar';
import { type ActiveWorkbench } from '@/components/shell/AppHeader';
import { AttentionBanner, type AttentionItem } from '@/components/shell/AttentionBanner';
import { RightSidebar } from '@/components/shell/RightSidebar';
import { useAppContext } from '@/context/AppContext';
import { TabQuickLaunchMenu } from '@/components/workbenches/TabQuickLaunchMenu';
import { useScreenContext } from '@/context/ScreenContext';
import { LENS_METADATA } from '@/components/shell/lensConfig';
import { useWorkspaceTabRouter } from '@/components/shell/useWorkspaceTabRouter';
import { ShellModals, type CreateTaskPrefill } from '@/components/shell/ShellModals';
import { useResourcePicker } from '@/components/shell/useResourcePicker';

interface AppWorkspaceShellProps {
  mode: ActiveWorkbench;
  onSwitchMode: (mode: ActiveWorkbench) => void;
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions: Session[];
  allSessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  tasks: import('@/types/domain').Task[];
  selectedTaskId: string | null;
  onSelectTask: (id: string) => void;
  onClearTask: () => void;
  sidebarView: 'chats' | 'tasks';
  onChangeSidebarView: (view: 'chats' | 'tasks') => void;
  onOpenFullChatIn?: (mode: ActiveWorkbench) => void;
  onOpenSessionHere?: (id: string) => void;
  onNewSession: () => void;
  onNewSessionForTask: (taskId: string) => Promise<void>;
  onSendMessage: (text: string) => Promise<boolean>;
  onClearHistory: () => void;
  onDeleteConversation: (sessionId: string) => void | Promise<void>;
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
  onShowToast?: (msg: string) => void;
  onOpenSettings?: () => void;
  isSidebarCollapsed: boolean;
  onToggleSidebar: () => void;
  resourceTabs: ResourceTabsState;
  isWebTabOpen: boolean;
  isWebTabExpanded: boolean;
  onToggleWebTab: () => void;
  onToggleExpandWebTab: () => void;
  openResources: (expanded: boolean) => void;
  workspaces?: import('@/types/domain').ExecutionWorkspace[];
  activeWorkspaceId?: string;
  setActiveWorkspaceId?: (id: string) => void;
  onSelectWorkspace?: (id: string) => void;
  activeWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  refreshWorkspaces?: () => Promise<import('@/types/domain').ExecutionWorkspace[]>;
}

export const AppWorkspaceShell: React.FC<AppWorkspaceShellProps> = ({
  mode,
  onSwitchMode,
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  allSessions,
  activeSessionId,
  onSelectSession,
  tasks,
  selectedTaskId,
  onSelectTask,
  onClearTask,
  sidebarView,
  onChangeSidebarView,
  onOpenFullChatIn,
  onOpenSessionHere,
  onNewSession,
  onNewSessionForTask,
  onSendMessage,
  onClearHistory: _onClearHistory,
  onDeleteConversation,
  onAcceptAndRun,
  onRejectDiff,
  onCopyDiff,
  onShowToast,
  onOpenSettings: _onOpenSettings,
  isSidebarCollapsed,
  onToggleSidebar: _onToggleSidebar,
  resourceTabs,
  isWebTabOpen,
  isWebTabExpanded,
  onToggleWebTab,
  onToggleExpandWebTab,
  openResources: _openResources,
  workspaces: propWorkspaces,
  activeWorkspaceId: propActiveWorkspaceId,
  setActiveWorkspaceId: propSetActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  activeWorkspace: propActiveWorkspace,
  refreshWorkspaces: propRefreshWorkspaces,
}) => {
  const {
    openSettings,
    workspaces: ctxWorkspaces,
    activeWorkspaceId: ctxActiveWorkspaceId,
    setActiveWorkspaceId: ctxSetActiveWorkspaceId,
    activeWorkspace: ctxActiveWorkspace,
    refreshWorkspaces: ctxRefreshWorkspaces,
    createTask,
    advanceTask,
    completeTask,
    cancelTask,
    refreshTasks,
    setProjectData,
    isResourcePickerOpen,
    setIsResourcePickerOpen,
    splitPercent,
    savedWorkError,
    retrySavedWork,
    isInspectorOpen,
    setIsInspectorOpen,
  } = useAppContext();

  const workspaces = propWorkspaces ?? ctxWorkspaces;
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const setActiveWorkspaceId = propOnSelectWorkspace ?? propSetActiveWorkspaceId ?? ctxSetActiveWorkspaceId;
  const activeWorkspace = propActiveWorkspace ?? ctxActiveWorkspace;
  const refreshWorkspaces = propRefreshWorkspaces ?? ctxRefreshWorkspaces;
  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false);
  const [compactWorkspace, setCompactWorkspace] = useState(false);

  const {
    activeScreenId,
    activeScreen,
    updateActiveScreen,
  } = useScreenContext();

  // Sync active screen's session, task and view state when screen switches
  useEffect(() => {
    if (!activeScreen) return;
    if (activeScreen.activeSessionId !== undefined && activeScreen.activeSessionId !== activeSessionId) {
      onSelectSession(activeScreen.activeSessionId || '');
    }
    const screenTaskId = activeScreen.selectedTaskId ?? null;
    if (screenTaskId !== selectedTaskId) {
      if (screenTaskId) onSelectTask(screenTaskId);
      else onClearTask();
    }
    if (activeScreen.sidebarView && activeScreen.sidebarView !== sidebarView) {
      onChangeSidebarView(activeScreen.sidebarView);
    }
  }, [activeScreenId]);

  // Keep activeScreen.tabs in sync with resourceTabs on the active screen, including runId and session bindings
  useEffect(() => {
    if (!activeScreen) return;
    const currentTabs = activeScreen.tabs || [];
    const tabsDiffer = currentTabs.length !== resourceTabs.tabs.length ||
      currentTabs.some((t, i) => {
        const next = resourceTabs.tabs[i];
        if (!next) return true;
        return (
          t.id !== next.id ||
          t.url !== next.url ||
          t.title !== next.title ||
          t.type !== next.type ||
          t.sessionId !== next.sessionId ||
          t.taskId !== next.taskId ||
          t.runId !== next.runId ||
          t.agentId !== next.agentId ||
          t.workspaceId !== next.workspaceId ||
          t.sourceId !== next.sourceId ||
          t.artifactId !== next.artifactId ||
          t.revision !== next.revision
        );
      });
    const activeDiffer = activeScreen.activeTabId !== resourceTabs.activeTabId;

    if (tabsDiffer || activeDiffer) {
      updateActiveScreen({
        tabs: resourceTabs.tabs,
        activeTabId: resourceTabs.activeTabId,
      });
    }
  }, [resourceTabs.tabs, resourceTabs.activeTabId, activeScreen, updateActiveScreen]);

  useEffect(() => {
    const query = window.matchMedia('(max-width: 1100px)');
    const update = () => { setCompactWorkspace(query.matches); if (query.matches) setIsInspectorOpen(false); };
    update(); query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);

  const [isGlobalCreateTaskOpen, setIsGlobalCreateTaskOpen] = useState(false);
  const [createTaskPrefill, setCreateTaskPrefill] = useState<CreateTaskPrefill>({});

  const {
    pickerTargetPane,
    pickerAnchor,
    setPickerAnchor,
    handleOpenResourcePicker,
    pickerResources,
  } = useResourcePicker({
    isResourcePickerOpen,
    setIsResourcePickerOpen,
    isWebTabOpen,
    onToggleWebTab,
    mode,
  });

  // Populated only from daemon approval/effect events. Never fabricate authority requests.
  const [attentionItems, setAttentionItems] = useState<AttentionItem[]>([]);

  const containerRef = useRef<HTMLDivElement>(null);
  const openedCanvasScopes = useRef(new Set<string>());
  const canvasScope = resourceTabs.scopeKey || mode;
  if (isWebTabOpen) openedCanvasScopes.current.add(canvasScope);

  // Active session lookup
  const activeSession =
    sessions.find((s) => s.id === activeSessionId) ||
    allSessions.find((s) => s.id === activeSessionId) || {
      id: '',
      nativeHarnessId: undefined as string | undefined,
      title: 'No conversation',
      time: '',
      preview: '',
      model: '',
      fileName: '',
      diffHunk: '',
      diffLinesCount: '',
      summary: '',
      messages: [],
      diffCode: [],
    };

  const boundTask = activeSession.taskId ? (tasks.find((task) => task.id === activeSession.taskId) ?? null) : null;
  const boundWorkspace = workspaces.find((w) => w.id === (boundTask?.metadata?.workspace_id || activeWorkspaceId));
  const taskAttentionCount = attentionItems.filter(
    (item) => item.worktreeId === boundWorkspace?.id || Boolean(activeSession.taskId && item.targetResource.includes(activeSession.taskId))
  ).length;

  const { openTab, handleOpenTab, handleLaunchAgentTab } = useWorkspaceTabRouter({
    isWebTabOpen,
    onToggleWebTab,
    resourceTabs,
    activeWorkspaceId,
    activeSession,
    selectedTaskId,
    onShowToast,
    setIsResourcePickerOpen,
  });

  // Global Keyboard Shortcuts (Cmd+K for Jump Palette, Cmd+Opt+K for Kanban)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        if (e.altKey) {
          e.preventDefault();
          openTab('kanban', 'Fleet Kanban', 'orca://kanban');
        } else {
          e.preventDefault();
          setIsCommandPaletteOpen((prev) => !prev);
        }
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'p') {
        e.preventDefault();
        setIsCommandPaletteOpen((prev) => !prev);
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'l') {
        e.preventDefault();
        onToggleExpandWebTab();
      }
    };
    const onOpenPalette = () => {
      setIsCommandPaletteOpen(true);
    };
    const onToggleChat = () => {
      onToggleExpandWebTab();
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('custos:open-command-palette', onOpenPalette);
    window.addEventListener('custos:toggle-chat', onToggleChat);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('custos:open-command-palette', onOpenPalette);
      window.removeEventListener('custos:toggle-chat', onToggleChat);
    };
  }, [openTab, onToggleExpandWebTab]);

  const handleResourceAskAgent = useCallback((draft: string, sessionId = activeSession.id) => {
    if (!sessionId || !allSessions.some(session => session.id === sessionId)) {
      onShowToast?.('Open the resource conversation before adding context.'); return;
    }
    if (readConversationDraft(sessionId) !== draft) appendConversationDraft(sessionId, draft);
    if (sessionId !== activeSession.id) onSelectSession(sessionId);
    onChangeSidebarView('chats');
    if (isWebTabExpanded) onToggleExpandWebTab();
    if (compactWorkspace && isWebTabOpen) onToggleWebTab();
  }, [activeSession.id, allSessions, onSelectSession, onChangeSidebarView, onShowToast, isWebTabExpanded, onToggleExpandWebTab, compactWorkspace, isWebTabOpen, onToggleWebTab]);

  const handleSelectSessionWithScreen = useCallback((id: string) => {
    onSelectSession(id);
    updateActiveScreen({ activeSessionId: id });
    const targetSession = allSessions.find((s) => s.id === id) || sessions.find((s) => s.id === id);
    const title = targetSession?.title || 'Chat';
    const existingTab = resourceTabs.tabs.find((t) => (t.type === 'conversation' || t.type === 'chat') && t.sessionId === id);
    if (existingTab) {
      resourceTabs.setActiveTabId(existingTab.id);
    } else {
      const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);
      if (activeTab && (activeTab.type === 'conversation' || activeTab.type === 'chat')) {
        resourceTabs.setTabs((prev) => prev.map((t) => (t.id === activeTab.id ? { ...t, sessionId: id, title } : t)));
      } else {
        openTab('conversation', title, `custos://chat/${id}`, undefined, activeWorkspaceId, 'primary', id);
      }
    }
  }, [onSelectSession, updateActiveScreen, allSessions, sessions, resourceTabs, openTab, activeWorkspaceId]);

  const handleSelectTaskWithScreen = useCallback((id: string) => {
    onSelectTask(id);
    updateActiveScreen({ selectedTaskId: id });
    const existing = resourceTabs.tabs.find((t) => t.type === 'tasks' || t.type === 'task');
    if (existing) {
      resourceTabs.setActiveTabId(existing.id);
    } else {
      openTab('tasks', 'Tasks & Timeline', 'custos://resource/tasks', undefined, activeWorkspaceId);
    }
  }, [onSelectTask, updateActiveScreen, resourceTabs, openTab, activeWorkspaceId]);

  const handleChangeSidebarViewWithScreen = useCallback((view: 'chats' | 'tasks') => {
    onChangeSidebarView(view);
    updateActiveScreen({ sidebarView: view });
    if (view === 'tasks') {
      const existing = resourceTabs.tabs.find((t) => t.type === 'tasks' || t.type === 'task');
      if (existing) {
        resourceTabs.setActiveTabId(existing.id);
      } else {
        openTab('tasks', 'Tasks & Timeline', 'custos://resource/tasks', undefined, activeWorkspaceId);
      }
    } else {
      const existing = resourceTabs.tabs.find((t) => t.type === 'conversation' || t.type === 'chat');
      if (existing) {
        resourceTabs.setActiveTabId(existing.id);
      }
    }
  }, [onChangeSidebarView, updateActiveScreen, resourceTabs, openTab, activeWorkspaceId]);

  const handleNewSession = useCallback(() => {
    onNewSession();
    const existingCount = resourceTabs.tabs.filter((t) => t.type === 'conversation' || t.type === 'chat').length;
    const count = existingCount + 1;
    const title = count > 1 ? `Chat #${count}` : 'AI Assistant';
    openTab('conversation', title, `custos://chat/new-${Date.now()}`, undefined, activeWorkspaceId);
  }, [onNewSession, resourceTabs.tabs, openTab, activeWorkspaceId]);

  const lensMeta = LENS_METADATA[mode];

  const workbenchTasks = tasks.filter(task => {
    if (task.metadata?.kind === 'conversation_chat_turn') return false;
    const pack = String(task.contract?.pack_id || task.metadata?.pack_id || '').toLowerCase();
    if (!pack) return true;
    return mode === 'engineering' ? ['engineering', 'coding', 'code'].includes(pack)
      : mode === 'research' ? pack === 'research' : ['assistant', 'copilot'].includes(pack);
  });
  const inspectedTab = resourceTabs.tabs.find(tab => tab.id === resourceTabs.activeTabId);
  const inspectedWorkspace = isWebTabOpen && inspectedTab?.workspaceId ? workspaces.find(workspace => workspace.id === inspectedTab.workspaceId) : activeWorkspace;
  const inspectedSession = isWebTabOpen && inspectedTab?.sessionId ? allSessions.find(session => session.id === inspectedTab.sessionId) : activeSession;
  const inspectedTask = isWebTabOpen && inspectedTab?.taskId ? tasks.find(task => task.id === inspectedTab.taskId) : tasks.find(task => task.id === selectedTaskId) || boundTask;

  const sidebarTools: SidebarTool[] = [];

  return (
    <div className="custos-workspace flex flex-col h-full w-full font-sans overflow-hidden select-none" data-workbench={mode}>
      {/* ── 1. SUPERVISORY ATTENTION STRIP (OrCa Attention + SADE Authority Gate) ── */}
      <AttentionBanner
        items={attentionItems}
        onDismiss={(id) => setAttentionItems((prev) => prev.filter((i) => i.id !== id))}
      />
      {savedWorkError && (
        <div role="alert" className="flex items-center justify-between gap-3 border-b border-rose-500/30 bg-rose-500/10 px-4 py-2 text-xs text-rose-300">
          <span>{savedWorkError}</span>
          <button type="button" onClick={retrySavedWork} className="shrink-0 rounded border border-rose-500/30 px-2 py-1 hover:bg-rose-500/10">Retry</button>
        </div>
      )}

      {/* ── 2. ADAPTIVE WORKSPACE BODY ── */}
      <div className="flex-1 flex overflow-hidden min-h-0 relative">
        {/* Shared Sidebar */}
        <WorkspaceSidebar
          activeWorkbench={mode}
          onSwitchWorkbench={onSwitchMode}
          currentProject={currentProject}
          projectNames={projectNames}
          onSelectProject={onSelectProject}
          sessions={sessions}
          allSessions={allSessions}
          activeSessionId={activeSessionId}
          onSelectSession={handleSelectSessionWithScreen}
          tasks={workbenchTasks}
          selectedTaskId={selectedTaskId}
          onSelectTask={handleSelectTaskWithScreen}
          onClearTask={() => {
            onClearTask();
            updateActiveScreen({ selectedTaskId: null });
          }}
          listMode={sidebarView}
          onChangeListMode={handleChangeSidebarViewWithScreen}
          onOpenFullChatIn={onOpenFullChatIn}
          onOpenSessionHere={onOpenSessionHere}
          onDeleteSession={onDeleteConversation}
          newTaskLabel={lensMeta.taskLabel}
          newTaskIcon={Plus}
          newTaskIconColor={lensMeta.iconColor}
          onNewSession={handleNewSession}
          onRequestCreateTask={() => {
            setCreateTaskPrefill({
              workspaceId: activeWorkspaceId,
              packId: mode === 'engineering' ? 'code' : mode === 'research' ? 'research' : 'copilot',
            });
            setIsGlobalCreateTaskOpen(true);
          }}
          tools={sidebarTools}
          sessionsTitle={lensMeta.title}
          emptySessionsMessage="No sessions yet"
          projectActiveColor={lensMeta.activeColor}
          avatarColor={lensMeta.avatarBg}
          avatarText={lensMeta.avatarText}
          avatarTextColor={lensMeta.avatarColor}
          onShowToast={onShowToast}
          onOpenSettings={(tab?: string) => openSettings(tab || 'general')}
          onOpenTab={handleOpenTab}
          onSwitchMode={onSwitchMode}
          isSidebarCollapsed={isSidebarCollapsed}
          workspaces={workspaces}
          activeWorkspaceId={activeWorkspaceId}
          onSelectWorkspace={setActiveWorkspaceId}
          refreshWorkspaces={refreshWorkspaces}
        />

        {/* Central Workspace Canvas: Tiling Canvas containing all Panes (Chat, Tasks, Terminal, Editor, Research) */}
        <div ref={containerRef} className="flex-1 flex flex-col overflow-hidden min-w-0 relative bg-canvas">
          <div className="flex-1 min-h-0 overflow-hidden relative">
            <OrcaTabbedContainer
              resourceTabs={resourceTabs}
              isPaneOpen={true}
              session={activeSession}
              onSendMessage={onSendMessage}
              boundTask={boundTask}
              boundWorkspace={boundWorkspace}
              taskAttentionCount={taskAttentionCount}
              onAcceptAndRun={onAcceptAndRun}
              onRejectDiff={onRejectDiff}
              onCopyDiff={onCopyDiff}
              onShowToast={onShowToast}
              isExpanded={isWebTabExpanded}
              onToggleExpand={onToggleExpandWebTab}
              mode={mode}
              splitPercent={splitPercent}
              onOpenTab={handleOpenTab}
              onOpenResourcePicker={handleOpenResourcePicker}
              onNewConversation={handleNewSession}
              onAskAgent={handleResourceAskAgent}
              onHandoffToCoding={async (_claims, result) => {
                const targetId = result.task_ids[0];
                if (!targetId) { onShowToast?.('The handoff returned no Coding Task.'); return; }
                try {
                  await refreshTasks();
                  if (result.workspace_id) setActiveWorkspaceId(result.workspace_id);
                  onSwitchMode('engineering');
                  onSelectTask(targetId);
                } catch (error) { onShowToast?.(`Handoff created Task ${targetId}; could not open it: ${String(error)}`); }
              }}
              activeWorkspaceId={activeWorkspaceId}
              onSelectWorkspace={setActiveWorkspaceId}
              activeWorkspace={activeWorkspace}
              workspaces={workspaces}
              tasks={workbenchTasks}
              allSessions={allSessions}
              selectedTaskId={selectedTaskId}
              onSelectTask={(id) => {
                if (id) {
                  handleSelectTaskWithScreen(id);
                } else {
                  onClearTask();
                }
              }}
              onOpenSession={(sessionId) => {
                handleSelectSessionWithScreen(sessionId);
              }}
              onNewSessionForTask={taskId => { void onNewSessionForTask(taskId); }}
              onCreateTask={createTask}
              onAdvanceStatus={async (taskId, status) => {
                await advanceTask(taskId, status);
              }}
              onCompleteTask={async (taskId, summary) => {
                await completeTask(taskId, summary);
              }}
              onCancelTask={async (taskId, reason) => {
                await cancelTask(taskId, reason);
              }}
              onRefreshTasks={refreshTasks}
            />
          </div>
          <TabQuickLaunchMenu
            isOpen={isResourcePickerOpen}
            onClose={() => {
              setIsResourcePickerOpen(false);
              setPickerAnchor(null);
            }}
            anchor={pickerAnchor}
            pickerResources={pickerResources}
            onOpenResource={(resId) => {
              handleOpenTab(resId, undefined, undefined, pickerTargetPane);
              setIsResourcePickerOpen(false);
              setPickerAnchor(null);
            }}
            onLaunchAgent={(harness) => {
              handleLaunchAgentTab(harness, pickerTargetPane);
              setPickerAnchor(null);
            }}
            activeWorkspaceTitle={activeWorkspace?.name || activeWorkspace?.id}
            mode={mode}
          />
        </div>
        <RightSidebar
          isOpen={isInspectorOpen}
          onClose={() => setIsInspectorOpen(false)}
          activeWorkspace={activeWorkspace || inspectedWorkspace}
          activeSession={inspectedSession}
          activeTask={inspectedTask}
          activeTab={inspectedTab}
          onOpenTab={handleOpenTab}
          mode={mode}
        />
      </div>

      {/* ── 5. MODALS & POPUPS ── */}
      <ShellModals
        isCommandPaletteOpen={isCommandPaletteOpen}
        onCloseCommandPalette={() => setIsCommandPaletteOpen(false)}
        onSwitchMode={onSwitchMode}
        openTab={openTab}
        onNewSession={handleNewSession}
        onOpenSettings={() => openSettings('general')}
        mode={mode}
        isGlobalCreateTaskOpen={isGlobalCreateTaskOpen}
        onCloseCreateTask={() => setIsGlobalCreateTaskOpen(false)}
        createTaskPrefill={createTaskPrefill}
        createTask={createTask}
        setProjectData={setProjectData}
        onShowToast={onShowToast}
        onSelectTask={onSelectTask}
        onChangeSidebarView={onChangeSidebarView}
        workspaces={workspaces}
        activeWorkspaceId={activeWorkspaceId}
      />
    </div>
  );
};
