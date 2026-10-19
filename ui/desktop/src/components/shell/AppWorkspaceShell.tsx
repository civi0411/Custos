import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Plus } from 'lucide-react';
import { appendConversationDraft, readConversationDraft } from '@/components/chat/ConversationPane';
import { Session } from '@/types';
import { OrcaTabbedContainer, resolveResources, type ResourceTabsState, type OrcaTabId, type OrcaTab, getTabType, findScopedResourceTab, workbenchResourceIds } from '@/components/workbenches/OrcaTabbedContainer';
import { WorkspaceSidebar, type SidebarTool } from '@/components/shell/WorkspaceSidebar';
import { type ActiveWorkbench } from '@/components/shell/AppHeader';
import { CommandPalette } from '@/components/shell/CommandPalette';
import { AttentionBanner, type AttentionItem } from '@/components/shell/AttentionBanner';
import { RightSidebar } from '@/components/shell/RightSidebar';

import { useAppContext } from '@/context/AppContext';
import { CreateTaskModal } from '@/components/tasks/CreateTaskModal';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor, HarnessDescriptor } from '@/types/domain';
import { TabQuickLaunchMenu } from '@/components/workbenches/TabQuickLaunchMenu';
import { useScreenContext } from '@/context/ScreenContext';


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
    if (activeScreen.activeSessionId && activeScreen.activeSessionId !== activeSessionId) {
      onSelectSession(activeScreen.activeSessionId);
    }
    if (activeScreen.selectedTaskId !== undefined && activeScreen.selectedTaskId !== selectedTaskId) {
      if (activeScreen.selectedTaskId) onSelectTask(activeScreen.selectedTaskId);
      else onClearTask();
    }
    if (activeScreen.sidebarView && activeScreen.sidebarView !== sidebarView) {
      onChangeSidebarView(activeScreen.sidebarView);
    }
  }, [activeScreenId]);
  useEffect(() => {
    const query = window.matchMedia('(max-width: 1100px)');
    const update = () => { setCompactWorkspace(query.matches); if (query.matches) setIsInspectorOpen(false); };
    update(); query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  const [isGlobalCreateTaskOpen, setIsGlobalCreateTaskOpen] = useState(false);
  const [createTaskPrefill, setCreateTaskPrefill] = useState<{
    title?: string;
    description?: string;
    workspaceId?: string;
    sessionId?: string;
    packId?: 'code' | 'research' | 'copilot';
  }>({});
  const [pickerCapabilities, setPickerCapabilities] = useState<CapabilityDescriptor[]>([]);
  const [pickerCapabilityState, setPickerCapabilityState] = useState<'loading' | 'ready' | 'failed'>('loading');

  useEffect(() => {
    if (!isResourcePickerOpen) return;
    let mounted = true;
    setPickerCapabilityState('loading');
    daemonClient.listCapabilities()
      .then((capabilities) => {
        if (!mounted) return;
        setPickerCapabilities(capabilities);
        setPickerCapabilityState('ready');
      })
      .catch(() => { if (mounted) setPickerCapabilityState('failed'); });
    return () => { mounted = false; };
  }, [isResourcePickerOpen]);

  useEffect(() => {
    if (!isResourcePickerOpen) return;
    const dismiss = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsResourcePickerOpen(false);
    };
    window.addEventListener('keydown', dismiss);
    return () => window.removeEventListener('keydown', dismiss);
  }, [isResourcePickerOpen]);

  const [pickerTargetPane, setPickerTargetPane] = useState<'primary' | 'split'>('primary');
  const [pickerAnchor, setPickerAnchor] = useState<{ top: number; left: number } | null>(null);

  const handleOpenResourcePicker = useCallback((targetPane: 'primary' | 'split' = 'primary', anchor?: { top: number; left: number }) => {
    setPickerTargetPane(targetPane);
    setPickerAnchor(anchor || null);
    if (!isWebTabOpen) onToggleWebTab();
    setIsResourcePickerOpen(true);
  }, [setIsResourcePickerOpen, isWebTabOpen, onToggleWebTab]);

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
  const openTab = useCallback((
    id: OrcaTabId,
    title: string,
    url: string,
    agentId?: string,
    workspaceId?: string,
    targetPane: 'primary' | 'split' = 'primary',
    targetSessionId?: string,
  ) => {
    if (!isWebTabOpen) {
      onToggleWebTab();
    }
    const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);
    const newTabId = `${id}-${crypto.randomUUID()}`;
    const newTab: OrcaTab = {
      id: newTabId,
      type: id,
      title,
      url,
      agentId,
      workspaceId: workspaceId || activeWorkspaceId || undefined,
      sessionId: targetSessionId || activeSession.id || undefined,
      taskId: selectedTaskId || activeSession.taskId,
    };

    if (activeTab && getTabType(activeTab) === 'tools') {
      resourceTabs.setTabs((prev) =>
        prev.map((t) => (t.id === activeTab.id ? newTab : t))
      );
    } else {
      const isMultiInstance = ['terminal', 'agents', 'browser', 'emulator', 'conversation', 'chat', 'tasks', 'task'].includes(id);
      const isSpecificFile = id === 'files' && Boolean(url && (url.startsWith('custos://file?') || url.startsWith('custos://diff?')));

      if (!isMultiInstance && !isSpecificFile) {
        const existing = findScopedResourceTab(resourceTabs.tabs, id, {
          agentId,
          workspaceId: workspaceId || activeWorkspaceId || undefined,
          sessionId: targetSessionId || activeSession.id || undefined,
          taskId: selectedTaskId || activeSession.taskId,
        });
        if (existing) {
          if (targetPane === 'split') {
            resourceTabs.setSplitTabId?.(existing.id);
          } else {
            resourceTabs.setActiveTabId(existing.id);
          }
          return;
        }
      } else if (isSpecificFile) {
        const existingFile = resourceTabs.tabs.find((t) => t.type === 'files' && t.url === url);
        if (existingFile) {
          if (targetPane === 'split') {
            resourceTabs.setSplitTabId?.(existingFile.id);
          } else {
            resourceTabs.setActiveTabId(existingFile.id);
          }
          return;
        }
      }

      resourceTabs.setTabs((prev) => [...prev, newTab]);
    }

    if (targetPane === 'split') {
      resourceTabs.setSplitTabId?.(newTabId);
    } else {
      resourceTabs.setActiveTabId(newTabId);
    }
  }, [isWebTabOpen, onToggleWebTab, resourceTabs, activeWorkspaceId, activeSession.id, activeSession.taskId, selectedTaskId]);

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

  const handleOpenTab = useCallback((
    id: OrcaTabId,
    customUrl?: string,
    customTitle?: string,
    targetPane: 'primary' | 'split' = 'primary',
    targetWorkspaceId?: string,
  ) => {
    const effectiveWsId = targetWorkspaceId || activeWorkspaceId;

    // 1. Files handling:
    // If opening 'files' without a specific file URL, reveal the Files Explorer in the Right Sidebar (Orca pattern)
    if (id === 'files' && (!customUrl || !customUrl.startsWith('custos://'))) {
      setIsInspectorOpen(true);
      window.dispatchEvent(new CustomEvent('custos:switch-inspector-tab', { detail: { tabId: 'files' } }));
      return;
    }

    // Specific file editor or diff tab in center canvas
    if (id === 'files' && customUrl && (customUrl.startsWith('custos://file?') || customUrl.startsWith('custos://diff?'))) {
      const existingFile = resourceTabs.tabs.find(t => t.type === 'files' && t.url === customUrl);
      if (existingFile) {
        if (!isWebTabOpen) onToggleWebTab();
        if (targetPane === 'split') resourceTabs.setSplitTabId?.(existingFile.id);
        else resourceTabs.setActiveTabId(existingFile.id);
        return;
      }
      const parsedTitle = customTitle || decodeURIComponent(customUrl.replace(/^custos:\/\/(file|diff)\?path=/, '')).split('/').pop() || 'File';
      openTab('files', parsedTitle, customUrl, undefined, effectiveWsId, targetPane);
      return;
    }

    // 2. Parallel multi-instance runtimes (Terminal, Agents, Browser, Phone Simulator, Chat, Tasks)
    if (['terminal', 'browser', 'emulator', 'agents', 'conversation', 'chat', 'tasks', 'task'].includes(id)) {
      if (id === 'conversation' || id === 'chat') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'conversation' || t.type === 'chat' || t.id.startsWith('conversation')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Chat #${count}` : (activeSession?.title || 'Chat'));
        const url = customUrl || `custos://chat/${count}`;
        openTab('conversation', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'tasks' || id === 'task') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'tasks' || t.type === 'task' || t.id.startsWith('tasks')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Tasks #${count}` : 'Tasks');
        const url = customUrl || `custos://tasks/${count}`;
        openTab('tasks', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'terminal') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'terminal' || t.id.startsWith('terminal')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Terminal ${count}` : 'Terminal');
        const url = customUrl || `custos://terminal/${count}`;
        openTab('terminal', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'browser') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'browser' || t.id.startsWith('browser')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Browser ${count}` : 'Web Preview');
        const url = customUrl || `custos://browser/${count}`;
        openTab('browser', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'emulator') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'emulator' || t.id.startsWith('emulator')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Phone Simulator ${count}` : 'Phone Simulator');
        const url = customUrl || `custos://emulator/${count}`;
        openTab('emulator', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'agents') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'agents' || t.id.startsWith('agents')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Coding Agents #${count}` : 'Coding Agents');
        const url = customUrl || `custos://agent/session-${count}`;
        openTab('agents', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
    }

    // 3. Singleton scoped lenses (worktrees, activity, lab, literature, findings, drafts, outbox, etc.)
    const existing = findScopedResourceTab(resourceTabs.tabs, id, {
      workspaceId: effectiveWsId || undefined, sessionId: activeSession.id || undefined,
      taskId: selectedTaskId || activeSession.taskId,
    });
    if (existing) {
      if (id === 'synthesis' && customUrl) {
        resourceTabs.setTabs(previous => previous.map(tab => tab.id === existing.id ? { ...tab, url: customUrl } : tab));
      }
      if (!isWebTabOpen) {
        onToggleWebTab();
      }
      if (targetPane === 'split') {
        resourceTabs.setSplitTabId?.(existing.id);
      } else {
        resourceTabs.setActiveTabId(existing.id);
      }
    } else {
      openTab(id, customTitle || (id.charAt(0).toUpperCase() + id.slice(1)), customUrl || `custos://resource/${id}`, undefined, effectiveWsId, targetPane);
    }
  }, [resourceTabs, isWebTabOpen, onToggleWebTab, openTab, activeWorkspaceId, activeSession.id, activeSession.taskId, selectedTaskId]);


  const handleLaunchAgentTab = useCallback((harness: HarnessDescriptor, targetPane: 'primary' | 'split' = 'primary') => {
    const existingAgentTabs = resourceTabs.tabs.filter((t) => t.type === 'agents' || t.agentId);
    const count = existingAgentTabs.length + 1;
    const title = count > 1 ? `${harness.name} #${count}` : harness.name;
    openTab('agents', title, `custos://agent/${harness.id}`, harness.id, activeWorkspaceId, targetPane);
    setIsResourcePickerOpen(false);
    onShowToast?.(`Opened ${harness.name}. Choose a Task and start the agent when ready.`);
  }, [openTab, resourceTabs.tabs, activeWorkspaceId, onShowToast, setIsResourcePickerOpen]);

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

  useEffect(() => {
    const handleOpenChatEvent = () => {
      const existing = resourceTabs.tabs.find((t) => t.type === 'conversation' || t.type === 'chat');
      if (existing) {
        resourceTabs.setActiveTabId(existing.id);
      } else {
        handleOpenTab('conversation');
      }
    };
    const handleOpenTabEvent = (e: Event) => {
      const custom = e as CustomEvent<{ id: OrcaTabId; url?: string; title?: string }>;
      if (custom.detail?.id) {
        handleOpenTab(custom.detail.id, custom.detail.url, custom.detail.title);
      }
    };
    const handleOpenFilesSidebar = () => {
      setIsInspectorOpen(true);
      window.dispatchEvent(new CustomEvent('custos:switch-inspector-tab', { detail: { tabId: 'files' } }));
    };
    window.addEventListener('custos:open-chat-tab', handleOpenChatEvent);
    window.addEventListener('custos:open-tab', handleOpenTabEvent);
    window.addEventListener('custos:open-files-sidebar', handleOpenFilesSidebar);
    window.addEventListener('custos:open-file-explorer', handleOpenFilesSidebar);
    return () => {
      window.removeEventListener('custos:open-chat-tab', handleOpenChatEvent);
      window.removeEventListener('custos:open-tab', handleOpenTabEvent);
      window.removeEventListener('custos:open-files-sidebar', handleOpenFilesSidebar);
      window.removeEventListener('custos:open-file-explorer', handleOpenFilesSidebar);
    };
  }, [resourceTabs, handleOpenTab, setIsInspectorOpen]);

  const pickerResources = resolveResources(
    pickerCapabilities,
    pickerCapabilityState !== 'loading',
    pickerCapabilityState === 'failed',
    typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  ).filter(resource => workbenchResourceIds(mode).includes(resource.id));

  // Lens metadata & personality
  const lensMeta = {
    copilot: {
      title: 'Copilot Sessions',
      taskLabel: 'New Conversation',
      iconColor: 'workbench-accent',
      activeColor: 'workbench-accent-bg',
      avatarText: 'CL',
      avatarBg: 'workbench-accent-bg',
      avatarColor: 'text-fg-on-emphasis',
      badge: 'Copilot',
      modelName: 'Custos mediated · executor visible per run',
      placeholder: 'Ask, plan, or continue the current task…'
    },
    engineering: {
      title: 'Engineering Sessions',
      taskLabel: 'New Engineering Task',
      iconColor: 'workbench-accent',
      activeColor: 'workbench-accent-bg',
      avatarText: 'CV',
      avatarBg: 'workbench-accent-bg',
      avatarColor: 'text-fg-on-emphasis',
      badge: 'Coding',
      modelName: 'Repository · worktree · verification',
      placeholder: 'Ask about the repository, propose a change, or continue coding…'
    },
    research: {
      title: 'Research Sessions',
      taskLabel: 'New Research Hypothesis',
      iconColor: 'workbench-accent',
      activeColor: 'workbench-accent-bg',
      avatarText: 'RL',
      avatarBg: 'workbench-accent-bg',
      avatarColor: 'text-fg-on-emphasis',
      badge: 'Research',
      modelName: 'Sources · claims · experiments',
      placeholder: 'Ask a question, inspect evidence, or design an experiment…'
    }
  }[mode];

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

  // Sidebar Tools: Keep sidebar clean (resources accessed via '+' picker in workbenches)
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
          onOpenSettings={() => openSettings('general')}
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
          activeWorkspaceId={activeWorkspaceId}
          activeSession={inspectedSession}
          activeTask={inspectedTask}
          activeTab={inspectedTab}
          onOpenTab={handleOpenTab}
          mode={mode}
          sessions={sessions}
          allSessions={allSessions}
          activeSessionId={activeSessionId}
          onSelectSession={handleSelectSessionWithScreen}
          onNewSession={handleNewSession}
          onDeleteSession={onDeleteConversation}
          tasks={workbenchTasks}
          selectedTaskId={selectedTaskId}
          onSelectTask={handleSelectTaskWithScreen}
          onRequestCreateTask={() => {
            setCreateTaskPrefill({
              workspaceId: activeWorkspaceId,
              packId: mode === 'engineering' ? 'code' : mode === 'research' ? 'research' : 'copilot',
            });
            setIsGlobalCreateTaskOpen(true);
          }}
        />
      </div>


      {/* ── 5. MODALS & POPUPS ── */}
      {/* Command & Jump Palette (Cmd+K / Cmd+P) */}
      <CommandPalette
        isOpen={isCommandPaletteOpen}
        onClose={() => setIsCommandPaletteOpen(false)}
        onSwitchMode={onSwitchMode}
        onOpenTab={openTab}
        onNewSession={onNewSession}
        onOpenSettings={() => openSettings('general')}
        onOpenWorktrees={() => openTab('worktrees', 'Worktrees', 'custos://worktrees')}
        currentMode={mode}
      />

      {/* Global Create Task Modal */}
      <CreateTaskModal
        isOpen={isGlobalCreateTaskOpen}
        onClose={() => setIsGlobalCreateTaskOpen(false)}
        onSubmit={async (params) => {
          const created = await createTask(params);
          if (createTaskPrefill.sessionId) {
            setProjectData((prev) => {
              const next = { ...prev };
              for (const p of Object.keys(next)) {
                next[p] = next[p].map((s) =>
                  s.id === createTaskPrefill.sessionId
                    ? { ...s, taskId: created.id, task_id: created.id }
                    : s
                );
              }
              return next;
            });
            onShowToast?.(`Task "${created.title}" created from conversation`);
          }
          onSelectTask(created.id);
          onChangeSidebarView('tasks');
          setIsGlobalCreateTaskOpen(false);
        }}
        workspaces={workspaces}
        currentWorkspaceId={createTaskPrefill.workspaceId || activeWorkspaceId}
        initialTitle={createTaskPrefill.title}
        initialDescription={createTaskPrefill.description}
        initialWorkspaceId={createTaskPrefill.workspaceId || activeWorkspaceId}
        initialSessionId={createTaskPrefill.sessionId}
        initialPackId={createTaskPrefill.packId}
      />
    </div>
  );
};
