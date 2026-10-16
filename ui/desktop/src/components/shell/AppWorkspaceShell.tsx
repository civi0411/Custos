import StopIcon from '~icons/lucide/square';
import React, { useState, useRef, useEffect, useCallback } from 'react';
import {
  Plus,
  FlaskConical,
  MessageSquare,
  Compass,
  ArrowUp,
  Paperclip,
  X,
  Mic,
  Monitor,
} from 'lucide-react';
import { ComposerPlusMenu } from '@/components/chat/ComposerPlusMenu';
import { TaskAnchorBanner } from '@/components/shell/TaskAnchorBanner';
import { AgentActionCard, AgentWorkingStatus } from '@/components/chat';
import { Session } from '@/types';
import { OrcaTabbedContainer, resolveResources, type ResourceTabsState, type OrcaTabId, type OrcaTab, getTabType } from '@/components/workbenches/OrcaTabbedContainer';
import { WorkspaceSidebar, type SidebarTool } from '@/components/shell/WorkspaceSidebar';
import { type ActiveWorkbench } from '@/components/shell/AppHeader';
import { CommandPalette } from '@/components/shell/CommandPalette';
import { AttentionBanner, type AttentionItem } from '@/components/shell/AttentionBanner';
import { JobActivityDock } from '@/components/shell/JobActivityDock';
import { useJobRegistry } from '@/hooks/useJobRegistry';
import { ModelSelector } from '@/components/chat/ModelSelector';

import { useAppContext } from '@/context/AppContext';
import { TasksPane } from '@/components/tasks/TasksPane';
import { CreateTaskModal } from '@/components/tasks/CreateTaskModal';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import 'katex/dist/katex.min.css';
import { remarkNormalizeChatMath } from '@/components/chat/markdownMath';
import { HighlightedCodeBlock } from '@/components/chat/HighlightedCodeBlock';
import { SlashCommandMenu, SLASH_COMMANDS, type SlashCommandDef } from '@/components/chat/SlashCommandMenu';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor, HarnessDescriptor } from '@/types/domain';
import { TabQuickLaunchMenu } from '@/components/workbenches/TabQuickLaunchMenu';




// Width changes are handled by CSS; only new message text needs parsing.
const ConversationMarkdown = React.memo(function ConversationMarkdown({ text }: { text: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm, remarkMath, remarkNormalizeChatMath]}
      rehypePlugins={[[rehypeKatex, { trust: false, throwOnError: false, strict: false, maxExpand: 1000, maxSize: 20 }]]}
      components={{
        pre({ children }) {
          const child = React.Children.toArray(children)[0];
          if (!React.isValidElement<{ className?: string; children?: React.ReactNode }>(child)) {
            return <pre>{children}</pre>;
          }
          const language = /(?:^|\s)language-([^\s]+)/.exec(child.props.className || '')?.[1];
          return <HighlightedCodeBlock code={String(child.props.children ?? '').replace(/\n$/, '')} language={language} />;
        },
        code({ children, className }) {
          return <code className={className || 'px-1.5 py-0.5 rounded bg-surface-1 text-fg-editor border border-border-muted font-mono text-[12px]'}>{children}</code>;
        }
      }}
    >
      {text}
    </ReactMarkdown>
  );
});

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
  onSendMessage: (text: string) => void;
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
    selectedModel,
    selectedModelAvailable,
    handleSelectModel,
    handleStopRun,
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
    column2Width,
    isResizingColumn,
    handleStartResizeColumn2,
    handleResetColumn2Width,
    savedWorkError,
    retrySavedWork,
  } = useAppContext();

  const workspaces = propWorkspaces ?? ctxWorkspaces;
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const setActiveWorkspaceId = propOnSelectWorkspace ?? propSetActiveWorkspaceId ?? ctxSetActiveWorkspaceId;
  const activeWorkspace = propActiveWorkspace ?? ctxActiveWorkspace;
  const refreshWorkspaces = propRefreshWorkspaces ?? ctxRefreshWorkspaces;
  const [inputText, setInputText] = useState('');
  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false);
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

  const handleOpenResourcePicker = useCallback((targetPane: 'primary' | 'split' = 'primary') => {
    setPickerTargetPane(targetPane);
    setIsResourcePickerOpen(true);
  }, [setIsResourcePickerOpen]);

  // Populated only from daemon approval/effect events. Never fabricate authority requests.
  const [attentionItems, setAttentionItems] = useState<AttentionItem[]>([]);

  const containerRef = useRef<HTMLDivElement>(null);

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
  const [composerMode, setComposerMode] = useState<'chat' | 'task'>('task');
  const [attachedFiles, setAttachedFiles] = useState<File[]>([]);
  const [isPlusMenuOpen, setIsPlusMenuOpen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleTriggerFileInput = () => {
    fileInputRef.current?.click();
  };

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      const newFiles = Array.from(e.target.files);
      setAttachedFiles((prev) => [...prev, ...newFiles]);
      onShowToast?.(`Đã thêm ${newFiles.length} tệp đính kèm`);
      e.target.value = '';
    }
  };

  const handleRemoveFile = (index: number) => {
    setAttachedFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const hasMessages = activeSession.messages && activeSession.messages.length > 0;

  // Global Keyboard Shortcuts (Cmd+K for Jump Palette, Cmd+Opt+K for Kanban, Cmd+U for File Attachment)
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'u') {
        e.preventDefault();
        fileInputRef.current?.click();
      } else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
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
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onShowToast]);

  const liveMessage = activeSession.messages.find(message => message.runId && !['completed', 'failed', 'cancelled', 'interrupted'].includes(message.runStatus || ''));
  const isPackCommand = SLASH_COMMANDS.some(command => command.command.toLowerCase() === inputText.trim().split(/\s+/)[0].toLowerCase());
  const dialogueRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const followBottom = useRef(true);

  const [scrollEdges, setScrollEdges] = useState({ top: false, bottom: false });
  const updateScrollEdges = () => {
    const el = dialogueRef.current;
    if (!el) return;
    const canScroll = el.scrollHeight > el.clientHeight;
    const top = canScroll && el.scrollTop > 12;
    const bottom = canScroll && el.scrollTop < el.scrollHeight - el.clientHeight - 12;
    setScrollEdges(previous => previous.top === top && previous.bottom === bottom ? previous : { top, bottom });
  };

  useEffect(() => {
    const el = dialogueRef.current;
    if (!el) return;
    const observer = new ResizeObserver(() => updateScrollEdges());
    observer.observe(el);
    updateScrollEdges();
    return () => observer.disconnect();
  }, [activeSessionId, hasMessages]);

  useEffect(() => { followBottom.current = true; }, [activeSessionId]);
  useEffect(() => {
    if (followBottom.current && dialogueRef.current) dialogueRef.current.scrollTop = dialogueRef.current.scrollHeight;
    updateScrollEdges();
  }, [activeSession.messages]);

  const { dispatchJob, updateJob } = useJobRegistry();

  const executeSlashCommand = (cmd: SlashCommandDef, args: string) => {
    setInputText('');

    if (cmd.command === '/task') {
      setCreateTaskPrefill({
        title: args.trim(),
        workspaceId: activeSession.workspaceId || activeWorkspaceId,
        sessionId: activeSession.id,
        packId: mode === 'engineering' ? 'code' : mode === 'research' ? 'research' : 'copilot',
      });
      setIsGlobalCreateTaskOpen(true);
      return;
    }

    // 1. Dispatch in JobRegistry so active tabs & sidebar show running status immediately
    const entryId = dispatchJob({
      jobId: cmd.jobId,
      jobName: cmd.label,
      sourceTabId: mode === 'engineering' ? 'changes' : mode === 'research' ? 'literature' : undefined,
    });

    // 2. Trigger daemon packs job directly (single accepted command path; avoid double-dispatch into chat LLM)
    daemonClient
      .runPacksJob({
        jobId: cmd.jobId,
        sessionId: activeSession.id,
        inputs: cmd.getInputs(args, { sessionId: activeSession.id }),
      })
      .then((res) => {
        const taskId = res.taskId || res.task_id || 'unknown';
        updateJob(entryId, { taskId, status: 'started', endedAt: Date.now() });
        onShowToast?.(`Started ${cmd.command} (Task ${taskId})`);
      })
      .catch((err: any) => {
        updateJob(entryId, { status: 'failed', error: err.message || String(err), endedAt: Date.now() });
        onShowToast?.(`Failed to run ${cmd.command}: ${err.message || String(err)}`);
      });
  };

  const handleSelectSlashCommand = (cmd: SlashCommandDef) => {
    if (cmd.hasArgs) {
      setInputText(`${cmd.command} `);
    } else {
      executeSlashCommand(cmd, '');
    }
  };

  const handleSteer = async () => {
    const trimmed = inputText.trim();
    if (!trimmed || !liveMessage?.runId) return;
    setInputText('');
    try {
      const harnessId = activeSession.nativeHarnessId || 'claude-code';
      await daemonClient.steerHarness(harnessId, liveMessage.runId, trimmed);
      onShowToast?.(`Guidance queued for next turn: "${trimmed}"`);
    } catch (err: any) {
      onShowToast?.(`Live steering unsupported for headless CLI harness: ${err?.message || err}`);
    }
  };

  const handleSend = () => {
    const trimmed = inputText.trim();
    if ((!trimmed && attachedFiles.length === 0) || liveMessage) return;

    if (trimmed.startsWith('/')) {
      const parts = trimmed.split(' ');
      const cmdName = parts[0].toLowerCase();
      const args = parts.slice(1).join(' ');
      const match = SLASH_COMMANDS.find((c) => c.command.toLowerCase() === cmdName);
      if (match) {
        executeSlashCommand(match, args);
        return;
      }
    }

    if (!activeSession.nativeHarnessId && !selectedModelAvailable && !isPackCommand) {
      onShowToast?.('Select an available model/account and reasoning effort');
      return;
    }

    let messageToSend = trimmed;
    if (attachedFiles.length > 0) {
      const fileDescriptions = attachedFiles
        .map((f) => `📎 [Attachment: ${f.name} (${(f.size / 1024).toFixed(1)} KB)]`)
        .join('\n');
      messageToSend = messageToSend ? `${messageToSend}\n\n${fileDescriptions}` : fileDescriptions;
      setAttachedFiles([]);
    }

    onSendMessage(messageToSend);
    setInputText('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Escape' && liveMessage?.runId) {
      e.preventDefault();
      void handleStopRun(liveMessage.runId);
      return;
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (liveMessage) {
        handleSteer();
      } else {
        handleSend();
      }
    }
  };

  const openTab = useCallback((
    id: OrcaTabId,
    title: string,
    url: string,
    agentId?: string,
    workspaceId?: string,
    targetPane: 'primary' | 'split' = 'primary'
  ) => {
    if (!isWebTabOpen) {
      onToggleWebTab();
    }
    const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);
    const newTabId = `${id}-${agentId ? `${agentId}-` : ''}${Date.now()}`;
    const newTab: OrcaTab = { id: newTabId, type: id, title, url, agentId, workspaceId };

    if (activeTab && getTabType(activeTab) === 'tools') {
      resourceTabs.setTabs((prev) =>
        prev.map((t) => (t.id === activeTab.id ? newTab : t))
      );
    } else {
      if (id !== 'agents' || !agentId) {
        const existing = resourceTabs.tabs.find((t) => getTabType(t) === id && (!agentId || t.agentId === agentId));
        if (existing) {
          if ((id === 'browser' || id === 'emulator') && url && !url.startsWith('custos:')) {
            resourceTabs.setTabs((prev) =>
              prev.map((t) => (t.id === existing.id ? { ...t, url, title: title || t.title } : t))
            );
          }
          if (targetPane === 'split') {
            resourceTabs.setSplitTabId?.(existing.id);
          } else {
            resourceTabs.setActiveTabId(existing.id);
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
  }, [isWebTabOpen, onToggleWebTab, resourceTabs]);

  const handleOpenTab = useCallback((id: OrcaTabId, customUrl?: string, customTitle?: string, targetPane: 'primary' | 'split' = 'primary') => {
    const existing = resourceTabs.tabs.find((t) => getTabType(t) === id);
    if (existing) {
      if (!isWebTabOpen) {
        onToggleWebTab();
      }
      if (targetPane === 'split') {
        resourceTabs.setSplitTabId?.(existing.id);
      } else {
        resourceTabs.setActiveTabId(existing.id);
      }
    } else {
      openTab(id, customTitle || (id.charAt(0).toUpperCase() + id.slice(1)), customUrl || `custos://resource/${id}`, undefined, undefined, targetPane);
    }
  }, [resourceTabs, isWebTabOpen, onToggleWebTab, openTab]);


  const handleLaunchAgentTab = useCallback((harness: HarnessDescriptor, targetPane: 'primary' | 'split' = 'primary') => {
    const existingAgentTabs = resourceTabs.tabs.filter((t) => t.type === 'agents' || t.agentId);
    const count = existingAgentTabs.length + 1;
    const title = count > 1 ? `${harness.name} #${count}` : harness.name;
    openTab('agents', title, `custos://agent/${harness.id}`, harness.id, activeWorkspaceId, targetPane);
    setIsResourcePickerOpen(false);
    onShowToast?.(`Launched ${harness.name} in parallel tab`);
  }, [openTab, resourceTabs.tabs, activeWorkspaceId, onShowToast, setIsResourcePickerOpen]);

  const handleResourceAskAgent = useCallback((draft: string) => {
    setInputText(previous => previous ? `${previous}\n\n${draft}` : draft);
  }, []);

  const pickerResources = resolveResources(
    pickerCapabilities,
    pickerCapabilityState !== 'loading',
    pickerCapabilityState === 'failed',
    typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  ).filter((resource) => {
    // Shared across all workbenches (Browser Preview & Phone Simulator & Notes)
    if (['browser', 'emulator', 'notes'].includes(resource.id)) return true;
    if (mode === 'engineering') {
      return ['files', 'activity', 'terminal', 'worktrees', 'agents'].includes(resource.id);
    }
    if (mode === 'research') {
      return ['literature', 'findings', 'lab', 'knowledge', 'artifacts', 'synthesis'].includes(resource.id);
    }
    return ['drafts', 'outbox', 'contacts', 'calendar'].includes(resource.id);
  });

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

      {/* ── 3. ADAPTIVE WORKSPACE BODY ── */}
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
          onSelectSession={onSelectSession}
          tasks={tasks}
          selectedTaskId={selectedTaskId}
          onSelectTask={onSelectTask}
          onClearTask={onClearTask}
          listMode={sidebarView}
          onChangeListMode={onChangeSidebarView}
          onOpenFullChatIn={onOpenFullChatIn}
          onOpenSessionHere={onOpenSessionHere}
          onDeleteSession={onDeleteConversation}
          newTaskLabel={lensMeta.taskLabel}
          newTaskIcon={Plus}
          newTaskIconColor={lensMeta.iconColor}
          onNewSession={onNewSession}
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

        {/* Central Workspace Canvas */}
        <div ref={containerRef} className="flex-1 flex overflow-hidden min-w-0 relative">
          {/* COLUMN 1: CONVERSATION PANE */}
          <div
            className="flex-1 flex flex-col h-full overflow-hidden bg-canvas relative select-text min-w-0"
            style={{
              display: isWebTabExpanded ? 'none' : 'flex'
            }}
          >
            {/* Dialogue Messages Canvas */}
            <div className="relative flex-1 min-h-0">
              {sidebarView === 'tasks' || selectedTaskId ? (
                <TasksPane
                  tasks={tasks}
                  sessions={allSessions}
                  workspaces={workspaces}
                  selectedTaskId={selectedTaskId}
                  onSelectTask={(id) => {
                    if (id) {
                      onSelectTask(id);
                    } else {
                      onClearTask();
                    }
                  }}
                  onOpenSession={(sessionId) => {
                    onChangeSidebarView('chats');
                    onSelectSession(sessionId);
                    onClearTask();
                  }}
                  onNewSessionForTask={(_taskId) => {
                    onChangeSidebarView('chats');
                    onNewSession();
                    onClearTask();
                  }}
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
              ) : (
                <div className="flex flex-col h-full overflow-hidden">
                  {/* Task Anchor Banner (Decoupled Task & Session Anchor) */}
                  <TaskAnchorBanner
                    session={activeSession}
                    task={boundTask}
                    boundWorkspace={boundWorkspace}
                    attentionCount={taskAttentionCount}
                    hasMessages={Boolean(hasMessages)}
                    onViewTask={() => {
                      if (activeSession.taskId) {
                        onSelectTask(activeSession.taskId);
                        onChangeSidebarView('tasks');
                      }
                    }}
                    onOpenSplitCanvas={() => {
                      if (!isWebTabOpen) {
                        onToggleWebTab();
                      }
                    }}
                    onPromoteToTask={() => {
                      const firstUserMsg = activeSession.messages.find((m) => m.role === 'user')?.text || '';
                      setCreateTaskPrefill({
                        title: activeSession.title,
                        description: firstUserMsg || activeSession.preview || '',
                        workspaceId: activeSession.workspaceId || activeWorkspaceId,
                        sessionId: activeSession.id,
                        packId: mode === 'engineering' ? 'code' : mode === 'research' ? 'research' : 'copilot',
                      });
                      setIsGlobalCreateTaskOpen(true);
                    }}
                    onDetachTask={() => {
                      activeSession.taskId = undefined;
                      onShowToast?.('Đã tách phiên trò chuyện khỏi Task (Về Chat Thường)');
                    }}
                  />

                  <div
                    ref={dialogueRef}
                    onScroll={event => {
                      const element = event.currentTarget;
                      followBottom.current = element.scrollHeight - element.scrollTop - element.clientHeight < 80;
                      updateScrollEdges();
                    }}
                    className="flex-1 overflow-y-auto px-6 py-6 space-y-6"
                  >
                  {!hasMessages ? (
                    <div className="h-full flex flex-col items-center justify-center text-center my-auto pb-10 px-4 w-full max-w-2xl mx-auto">
                      <div className="w-12 h-12 rounded-2xl bg-surface-1 border border-border-default flex items-center justify-center text-fg-muted mb-3 shadow-xs">
                        {mode === 'engineering' ? (
                          <span className="font-mono text-xl workbench-accent select-none">&#123;_&#125;</span>
                        ) : mode === 'research' ? (
                          <FlaskConical className="w-6 h-6 workbench-accent" />
                        ) : (
                          <MessageSquare className="w-6 h-6 workbench-accent" />
                        )}
                      </div>
                      <div className="workbench-kicker mb-1.5 text-xs text-white/50">{lensMeta.badge} workbench</div>
                      <h1 className="text-xl md:text-2xl font-semibold text-fg-editor tracking-[-0.02em] max-w-lg mb-6">
                        {currentProject === 'Custos Workspace' ? (
                          mode === 'engineering' ? 'What should we build today?' : mode === 'research' ? 'What should we investigate today?' : 'How can Custos help you today?'
                        ) : (
                          <>
                            {mode === 'engineering' ? 'What should we build in ' : mode === 'research' ? 'What should we investigate in ' : 'How can Custos help with '}
                            <span className="underline decoration-[var(--color-border-emphasis)] underline-offset-4">{currentProject}</span>?
                          </>
                        )}
                      </h1>

                      {/* CENTER HERO CHAT BOX (Hình 1) */}
                      <div className="w-full text-left rounded-3xl bg-[#1c1c20] border border-white/12 shadow-2xl p-4 relative transition-all focus-within:border-white/20 focus-within:ring-1 focus-within:ring-white/10 backdrop-blur-md">
                        <SlashCommandMenu
                          query={inputText}
                          mode={mode}
                          isOpen={inputText.startsWith('/') && (!inputText.includes(' ') || inputText.length <= 15)}
                          onSelectCommand={handleSelectSlashCommand}
                          onClose={() => { }}
                        />

                        {/* Attached Files Chips */}
                        {attachedFiles.length > 0 && (
                          <div className="mb-2.5 flex flex-wrap gap-1.5">
                            {attachedFiles.map((file, idx) => (
                              <div
                                key={`${file.name}-${idx}`}
                                className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-white/8 border border-white/10 text-xs text-white/90"
                              >
                                <Paperclip className="size-3 text-white/60" />
                                <span className="truncate max-w-[150px]">{file.name}</span>
                                <button
                                  type="button"
                                  onClick={() => handleRemoveFile(idx)}
                                  className="text-white/40 hover:text-white/90 transition-colors ml-0.5 cursor-pointer"
                                >
                                  <X className="size-3" />
                                </button>
                              </div>
                            ))}
                          </div>
                        )}

                        {/* Textarea */}
                        <textarea
                          ref={textareaRef}
                          rows={3}
                          value={inputText}
                          onChange={(e) => setInputText(e.target.value)}
                          onKeyDown={handleKeyDown}
                          placeholder={
                            composerMode === 'chat' && !activeSession.taskId
                              ? 'Ask Custos anything (Hỏi đáp, tra cứu, giải thích)...'
                              : lensMeta.placeholder || 'Ask Custos or assign task: Plan, analyze code, execute actions...'
                          }
                          className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-white placeholder-white/35 text-[14px] resize-none leading-relaxed min-h-[64px]"
                          autoFocus
                        />

                        {/* Bottom Toolbar inside the Box */}
                        <div className="flex items-center justify-between pt-2 mt-1">
                          <div className="flex items-center gap-2">
                            {/* + Button */}
                            <div className="relative">
                              <button
                                type="button"
                                onClick={() => setIsPlusMenuOpen((prev) => !prev)}
                                className="size-8 rounded-full hover:bg-white/10 text-white/70 hover:text-white flex items-center justify-center transition cursor-pointer"
                                title="Add files, skills, plugins (⌘U)"
                              >
                                <Plus className="size-4" />
                              </button>
                              <ComposerPlusMenu
                                isOpen={isPlusMenuOpen}
                                onClose={() => setIsPlusMenuOpen(false)}
                                onAddFilesOrPhotos={handleTriggerFileInput}
                                onOpenSkills={() => {
                                  onShowToast?.('Mở danh mục Skills');
                                  handleOpenTab('tools');
                                }}
                                onAddConnector={() => {
                                  onShowToast?.('Mở MCP Connectors');
                                  handleOpenTab('tools');
                                }}
                                onAddPlugins={() => {
                                  onShowToast?.('Mở Plugins');
                                  handleOpenTab('tools');
                                }}
                                position="top"
                              />
                            </div>

                            {/* Share screen button (as seen in Image 1) */}
                            <button
                              type="button"
                              onClick={() => {
                                onShowToast?.('Chia sẻ màn hình / context workspace');
                              }}
                              className="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs text-white/60 hover:text-white hover:bg-white/8 transition cursor-pointer"
                            >
                              <Monitor className="size-3.5" />
                              <span>Share screen</span>
                            </button>
                          </div>

                          <div className="flex items-center gap-2">
                            {/* Model selector */}
                            {activeSession.nativeHarnessId ? (
                              <span className="rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-xs text-white/80">
                                {activeSession.nativeHarnessId}
                              </span>
                            ) : (
                              <ModelSelector
                                currentModel={activeSession?.model || selectedModel}
                                onSelectModel={handleSelectModel}
                              />
                            )}

                            {/* Mic */}
                            <button
                              type="button"
                              className="size-8 rounded-full hover:bg-white/10 text-white/60 hover:text-white flex items-center justify-center transition cursor-pointer"
                              title="Voice input"
                            >
                              <Mic className="size-4" />
                            </button>

                            {/* Submit button (Image 1 style circular button) */}
                            <button
                              type="button"
                              onClick={handleSend}
                              disabled={!inputText.trim() && attachedFiles.length === 0}
                              className={`size-8 rounded-full flex items-center justify-center transition shadow-md cursor-pointer ${
                                inputText.trim() || attachedFiles.length > 0
                                  ? 'bg-[#2563eb] hover:bg-[#1d4ed8] text-white'
                                  : 'bg-white/10 text-white/30 cursor-not-allowed'
                              }`}
                              title="Send message (Enter)"
                            >
                              <ArrowUp className="size-4" />
                            </button>
                          </div>
                        </div>
                      </div>

                      {/* FLOATING CAPSULE TOGGLE: [ Chat | Task ] BELOW THE HERO BOX (Hình 1) */}
                      <div className="mt-4 flex justify-center select-none">
                        <div className="flex items-center rounded-full bg-[#18181b] border border-white/10 p-1 shadow-lg">
                          <button
                            type="button"
                            onClick={() => setComposerMode('chat')}
                            className={`px-4 py-1.5 rounded-full text-xs font-medium transition cursor-pointer ${
                              composerMode === 'chat' && !activeSession.taskId
                                ? 'bg-zinc-700/90 text-white shadow-xs font-semibold'
                                : 'text-white/50 hover:text-white/80'
                            }`}
                            title="Chat Thường: Tra cứu, hỏi đáp, an toàn code"
                          >
                            Chat
                          </button>
                          <button
                            type="button"
                            onClick={() => setComposerMode('task')}
                            className={`px-4 py-1.5 rounded-full text-xs font-medium transition cursor-pointer ${
                              composerMode === 'task' || Boolean(activeSession.taskId)
                                ? 'bg-zinc-700/90 text-white shadow-xs font-semibold'
                                : 'text-white/50 hover:text-white/80'
                            }`}
                            title="Chế độ Task: Giao việc cho Agent phân tích và sửa code"
                          >
                            Task
                          </button>
                        </div>
                      </div>
                    </div>
                  ) : (
                  <div className="space-y-6 max-w-3xl mx-auto w-full">
                    {activeSession.messages.map((msg, i) => {
                      const isUser = msg.role === 'user';
                      return (
                        <div
                          key={msg.id || i}
                          aria-label={isUser ? "User message" : "Agent message"}
                          className={isUser ? "conversation-user-message space-y-2 mb-4" : "space-y-2.5 pb-6 border-b border-border-muted/40 last:border-b-0 min-w-0"}
                        >
                          {/* Message Content */}
                          <div className="conversation-markdown text-[13px] leading-relaxed text-fg-editor space-y-3 min-w-0 break-words [overflow-wrap:anywhere]">
                            <ConversationMarkdown text={msg.text} />
                            {!msg.text && msg.runId && !['completed', 'failed', 'cancelled', 'interrupted'].includes(msg.runStatus || '') && <p role="status" className="text-xs text-fg-muted">{msg.runStatus === 'thinking' ? 'Thinking...' : 'Running...'}</p>}
                            {msg.liveError && <p role="alert" className="mt-2 text-xs text-amber-400">{msg.liveError}</p>}
                            {msg.activities?.map((activity, index) => (
                              <AgentActionCard
                                key={`${msg.runId || i}-activity-${index}`}
                                activity={activity}
                                onOpenTab={handleOpenTab}
                                onApproveAction={(actionId) =>
                                  daemonClient.approveOutbox(actionId).then(() => onShowToast?.('Action approved')).catch(err => onShowToast?.(String(err)))
                                }
                                onRejectAction={(actionId) =>
                                  onShowToast?.(`Action ${actionId} rejected`)
                                }
                                onHandoffToCoding={(claimId, statement) =>
                                  daemonClient.handoffClaimToCoding(claimId, statement).then(() => onShowToast?.('Claim handed off to Coding')).catch(err => onShowToast?.(String(err)))
                                }
                                onSendAnswer={(answer) => {
                                  if (liveMessage?.runId) {
                                    const harnessId = activeSession.nativeHarnessId || 'claude-code';
                                    daemonClient.steerHarness(harnessId, liveMessage.runId, answer)
                                      .then(() => onShowToast?.('Answer delivered to agent'))
                                      .catch(() => onSendMessage(answer));
                                  } else {
                                    onSendMessage(answer);
                                  }
                                }}
                              />
                            ))}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
              </div>
              )}

              {sidebarView !== 'tasks' && !selectedTaskId && (
                <>
                  <div aria-hidden="true" className={`conversation-scroll-blur conversation-scroll-blur-top ${scrollEdges.top ? 'is-visible' : ''}`} />
                  <div aria-hidden="true" className={`conversation-scroll-blur conversation-scroll-blur-bottom ${scrollEdges.bottom ? 'is-visible' : ''}`} />
                </>
              )}
            </div>

            {/* DOCKED CHAT COMPOSER (Natural Flex Child) - Only when hasMessages */}
            {sidebarView !== 'tasks' && !selectedTaskId && hasMessages && (
              <div className="shrink-0 px-4 py-3 bg-canvas">
                {liveMessage && (
                  <div className="max-w-3xl mx-auto mb-2.5">
                    <AgentWorkingStatus
                      status={liveMessage.runStatus}
                      runId={liveMessage.runId}
                      harnessName={
                        mode === 'research'
                          ? 'Research Copilot'
                          : mode === 'copilot'
                          ? 'Personal Copilot'
                          : activeSession.nativeHarnessId
                          ? `Coding Agent (${activeSession.nativeHarnessId})`
                          : 'Coding Assistant'
                      }
                      currentActivity={liveMessage.activities?.[liveMessage.activities.length - 1]}
                      onStop={() => liveMessage.runId && void handleStopRun(liveMessage.runId)}
                      onFocusSteer={() => textareaRef.current?.focus()}
                    />
                  </div>
                )}

                {/* Attached Files Chips */}
                {attachedFiles.length > 0 && (
                  <div className="max-w-3xl mx-auto mb-2 flex flex-wrap gap-1.5">
                    {attachedFiles.map((file, idx) => (
                      <div
                        key={`${file.name}-${idx}`}
                        className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-[#27272a] border border-white/10 text-xs text-white/90"
                      >
                        <Paperclip className="size-3 text-white/60" />
                        <span className="truncate max-w-[150px]">{file.name}</span>
                        <button
                          type="button"
                          onClick={() => handleRemoveFile(idx)}
                          className="text-white/40 hover:text-white/90 transition-colors ml-0.5 cursor-pointer"
                        >
                          <X className="size-3" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}

                {/* DOCKED PILL COMPOSER (Hình 2: Compact rounded-full pill) */}
                <div className="max-w-3xl mx-auto rounded-full bg-[#242428] border border-white/10 shadow-xl px-2.5 py-1.5 flex items-center gap-2 relative transition-all focus-within:border-white/20 focus-within:ring-1 focus-within:ring-white/15">
                  <SlashCommandMenu
                    query={inputText}
                    mode={mode}
                    isOpen={inputText.startsWith('/') && (!inputText.includes(' ') || inputText.length <= 15)}
                    onSelectCommand={handleSelectSlashCommand}
                    onClose={() => { }}
                  />

                  {/* Plus button with popover */}
                  <div className="relative shrink-0">
                    <button
                      type="button"
                      onClick={() => setIsPlusMenuOpen((prev) => !prev)}
                      className="size-8 rounded-full hover:bg-white/10 text-white/70 hover:text-white flex items-center justify-center transition cursor-pointer"
                      title="Add files, skills, plugins (⌘U)"
                    >
                      <Plus className="size-4" />
                    </button>

                    <ComposerPlusMenu
                      isOpen={isPlusMenuOpen}
                      onClose={() => setIsPlusMenuOpen(false)}
                      onAddFilesOrPhotos={handleTriggerFileInput}
                      onOpenSkills={() => {
                        onShowToast?.('Mở danh mục Skills');
                        handleOpenTab('tools');
                      }}
                      onAddConnector={() => {
                        onShowToast?.('Mở MCP Connectors');
                        handleOpenTab('tools');
                      }}
                      onAddPlugins={() => {
                        onShowToast?.('Mở Plugins');
                        handleOpenTab('tools');
                      }}
                      position="top"
                    />
                  </div>

                  {/* Input / Textarea */}
                  <textarea
                    ref={textareaRef}
                    rows={1}
                    value={inputText}
                    onChange={(e) => setInputText(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder={
                      liveMessage
                        ? 'Agent is working… Type guidance here to steer agent in-flight (or press Stop/Esc)...'
                        : composerMode === 'chat' && !activeSession.taskId
                        ? 'Ask Custos...'
                        : 'Ask Custos or assign task...'
                    }
                    className="flex-1 bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-white placeholder-white/40 text-[13.5px] resize-none leading-relaxed py-1.5 max-h-32 min-h-[28px]"
                  />

                  {/* Right Controls: Model Selector, Mic, and Blue Up-Arrow Send Button */}
                  <div className="flex items-center gap-1.5 shrink-0">
                    {activeSession.nativeHarnessId ? (
                      <span className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[11px] text-white/80">
                        {activeSession.nativeHarnessId}
                      </span>
                    ) : (
                      <ModelSelector
                        currentModel={activeSession?.model || selectedModel}
                        onSelectModel={handleSelectModel}
                      />
                    )}

                    <button
                      type="button"
                      className="size-8 rounded-full hover:bg-white/10 text-white/60 hover:text-white flex items-center justify-center transition cursor-pointer"
                      title="Voice input"
                    >
                      <Mic className="size-4" />
                    </button>

                    <button
                      type="button"
                      onClick={() => {
                        if (liveMessage) {
                          if (inputText.trim()) {
                            handleSteer();
                          } else if (liveMessage.runId) {
                            void handleStopRun(liveMessage.runId);
                          }
                        } else {
                          handleSend();
                        }
                      }}
                      disabled={!liveMessage && (!inputText.trim() && attachedFiles.length === 0)}
                      aria-label={
                        liveMessage
                          ? inputText.trim()
                            ? 'Queue guidance for next turn'
                            : 'Stop response'
                          : 'Send message'
                      }
                      title={
                        liveMessage
                          ? inputText.trim()
                            ? 'Queue guidance for next turn'
                            : 'Stop response'
                          : 'Send message (Enter)'
                      }
                      className={`size-8 rounded-full flex items-center justify-center transition shadow-sm cursor-pointer ${
                        liveMessage
                          ? inputText.trim()
                            ? 'bg-amber-500 text-black hover:bg-amber-400'
                            : 'bg-rose-500 text-white hover:bg-rose-600'
                          : inputText.trim() || attachedFiles.length > 0
                          ? 'bg-[#2563eb] hover:bg-[#1d4ed8] text-white'
                          : 'bg-white/10 text-white/30 cursor-not-allowed'
                      }`}
                    >
                      {liveMessage ? (
                        inputText.trim() ? (
                          <Compass className="size-4 animate-spin-slow" aria-hidden="true" />
                        ) : (
                          <StopIcon className="size-3.5 fill-current" aria-hidden="true" />
                        )
                      ) : (
                        <ArrowUp className="size-4" aria-hidden="true" />
                      )}
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* EDGE REVEAL DRAG HANDLE (When pane is collapsed, drag left to reveal) */}
          {!isWebTabOpen && (
            <div
              className="absolute right-0 inset-y-0 w-3.5 z-30 cursor-col-resize group flex items-center justify-end select-none"
              onMouseDown={handleStartResizeColumn2}
              title="Kéo sang trái hoặc click vào tab trên menu để mở lại khung tài nguyên"
            >
              <div className="w-1 h-16 bg-white/20 rounded-l group-hover:bg-workbench-accent group-hover:w-1.5 transition-all shadow-sm" />
            </div>
          )}

          {/* COLUMN 2: RIGHT RESOURCE CANVAS & TABBED CONTAINER */}
          {isWebTabOpen && (
            <div
              className="h-full shrink-0 bg-surface-0 relative z-20 flex flex-col overflow-hidden"
              style={{
                width: isWebTabExpanded ? '100%' : `${column2Width}px`
              }}
            >
              {/* Continuous Active Drag Handle Splitter (Aligned with TopBar Divider) */}
              {!isWebTabExpanded && (
                <div
                  className="absolute left-0 inset-y-0 w-3 -ml-1.5 cursor-col-resize z-30 group flex items-center justify-center select-none"
                  onMouseDown={handleStartResizeColumn2}
                  onDoubleClick={handleResetColumn2Width}
                  title="Drag to resize pane (double click to reset)"
                >
                  <div
                    className={`w-px h-full transition-colors ${
                      isResizingColumn ? 'bg-workbench-accent' : 'bg-white/10 group-hover:bg-workbench-accent'
                    }`}
                  />
                </div>
              )}
              <div className="flex-1 min-h-0 overflow-hidden relative">
                <OrcaTabbedContainer
                  resourceTabs={resourceTabs}
                  isPaneOpen={Boolean(isWebTabOpen)}
                  session={activeSession}
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
                  activeWorkspaceId={activeWorkspaceId}
                  onSelectWorkspace={setActiveWorkspaceId}
                  activeWorkspace={activeWorkspace}
                  workspaces={workspaces}
                />
              </div>
              <TabQuickLaunchMenu
                isOpen={isResourcePickerOpen}
                onClose={() => setIsResourcePickerOpen(false)}
                pickerResources={pickerResources}
                onOpenResource={(resId) => {
                  handleOpenTab(resId, undefined, undefined, pickerTargetPane);
                  setIsResourcePickerOpen(false);
                }}
                onLaunchAgent={(harness) => {
                  handleLaunchAgentTab(harness, pickerTargetPane);
                }}
                activeWorkspaceTitle={activeWorkspace?.name || activeWorkspace?.id}
                mode={mode}
              />
            </div>
          )}

          {/* FLOATING ACTIVITY DOCK (Only visible when tab canvas is maximized) */}
          {isWebTabExpanded && (
            <JobActivityDock
              onRestoreSplit={onToggleExpandWebTab}
              onOpenKanban={() => handleOpenTab('kanban')}
              lensBadge={lensMeta.badge}
              lensColor={lensMeta.activeColor}
            />
          )}
        </div>
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

      {/* Hidden File Input for Attachments (⌘U) */}
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileSelect}
        multiple
        className="hidden"
        aria-hidden="true"
      />

    </div>
  );
};
