import StopIcon from '~icons/lucide/square';
import React, { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import {
  Send,
  Plus,
  FlaskConical,
  MessageSquare,
  Compass
} from 'lucide-react';
import { AgentActionCard, AgentWorkingStatus } from '@/components/chat';
import { Session } from '@/types';
import { OrcaTabbedContainer, resolveResources, type ResourceTabsState, type OrcaTabId, type OrcaTab, getTabType } from '@/components/workbenches/OrcaTabbedContainer';
import { WorkspaceSidebar, type SidebarTool } from '@/components/shell/WorkspaceSidebar';
import { AppHeader, type ActiveWorkbench } from '@/components/shell/AppHeader';
import { CommandPalette } from '@/components/shell/CommandPalette';
import { AttentionBanner, type AttentionItem } from '@/components/shell/AttentionBanner';
import { JobActivityDock } from '@/components/shell/JobActivityDock';
import { useJobRegistry } from '@/hooks/useJobRegistry';
import { ModelSelector } from '@/components/chat/ModelSelector';

import { useAppContext } from '@/context/AppContext';
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
  onToggleSidebar,
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
    handleSelectModel,
    handleStopRun,
    workspaces: ctxWorkspaces,
    activeWorkspaceId: ctxActiveWorkspaceId,
    setActiveWorkspaceId: ctxSetActiveWorkspaceId,
    activeWorkspace: ctxActiveWorkspace,
    refreshWorkspaces: ctxRefreshWorkspaces,
  } = useAppContext();

  const workspaces = propWorkspaces ?? ctxWorkspaces;
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const setActiveWorkspaceId = propOnSelectWorkspace ?? propSetActiveWorkspaceId ?? ctxSetActiveWorkspaceId;
  const activeWorkspace = propActiveWorkspace ?? ctxActiveWorkspace;
  const refreshWorkspaces = propRefreshWorkspaces ?? ctxRefreshWorkspaces;
  const [inputText, setInputText] = useState('');
  const [splitPercent, setSplitPercent] = useState<number>(() => {
    return mode === 'copilot' ? 50 : 42;
  });
  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false);
  const [isResourcePickerOpen, setIsResourcePickerOpen] = useState(false);
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

  // Populated only from daemon approval/effect events. Never fabricate authority requests.
  const [attentionItems, setAttentionItems] = useState<AttentionItem[]>([]);

  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

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

  const hasMessages = activeSession.messages && activeSession.messages.length > 0;

  // Split-pane dragging logic
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newPercent = ((e.clientX - rect.left) / rect.width) * 100;
      if (newPercent >= 20 && newPercent <= 75) {
        setSplitPercent(newPercent);
      }
    };

    const handleMouseUp = () => {
      isDragging.current = false;
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

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
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onShowToast]);

  const liveMessage = activeSession.messages.find(message => message.runId && !['completed', 'failed', 'cancelled', 'interrupted'].includes(message.runStatus || ''));
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

  const { jobs, dispatchJob, updateJob } = useJobRegistry();

  const executeSlashCommand = (cmd: SlashCommandDef, args: string) => {
    setInputText('');

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
      onShowToast?.(`Steered agent: "${trimmed}"`);
    } catch (err: any) {
      onShowToast?.(`Failed to steer agent: ${err?.message || err}`);
    }
  };

  const handleSend = () => {
    const trimmed = inputText.trim();
    if (!trimmed || liveMessage) return;

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

    onSendMessage(inputText);
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
    workspaceId?: string
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
          resourceTabs.setActiveTabId(existing.id);
          return;
        }
      }
      resourceTabs.setTabs((prev) => [...prev, newTab]);
    }
    resourceTabs.setActiveTabId(newTabId);
  }, [isWebTabOpen, onToggleWebTab, resourceTabs]);

  const handleLaunchAgentTab = useCallback((harness: HarnessDescriptor) => {
    const existingAgentTabs = resourceTabs.tabs.filter((t) => t.type === 'agents' || t.agentId);
    const count = existingAgentTabs.length + 1;
    const title = count > 1 ? `${harness.name} #${count}` : harness.name;
    openTab('agents', title, `custos://agent/${harness.id}`, harness.id, activeWorkspaceId);
    setIsResourcePickerOpen(false);
    onShowToast?.(`Launched ${harness.name} in parallel tab`);
  }, [openTab, resourceTabs.tabs, activeWorkspaceId, onShowToast]);

  const runningJobsByTab = useMemo(() => {
    const counts: Record<string, number> = {};
    jobs.forEach((j) => {
      if ((j.status === 'running' || j.status === 'queued') && j.sourceTabId) {
        counts[j.sourceTabId] = (counts[j.sourceTabId] || 0) + 1;
      }
    });
    return counts;
  }, [jobs]);

  const handleOpenTab = useCallback((id: OrcaTabId) => {
    const titles: Partial<Record<OrcaTabId, string>> = {
      files: 'Files & Changes',
      activity: 'Activity',
      worktrees: 'Worktree Manager',
      terminal: 'Terminal',
      browser: 'Browser Preview',
      emulator: 'Phone Simulator',
      literature: 'Sources',
      findings: 'Findings',
      lab: 'Lab',
      knowledge: 'Methods',
      artifacts: 'Artifacts',
      synthesis: 'Handoff',
      drafts: 'Drafts',
      outbox: 'Outbox',
      contacts: 'Contacts',
      calendar: 'Calendar',
      notes: 'Notes',
    };
    const title = titles[id] || id.charAt(0).toUpperCase() + id.slice(1);
    openTab(id, title, `custos://resource/${id}`);
  }, [openTab]);
  const handleResourceAskAgent = useCallback((draft: string) => {
    setInputText(previous => previous ? `${previous}\n\n${draft}` : draft);
  }, []);

  const handleCloseTab = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    if (resourceTabs.tabs.length <= 1) {
      onToggleWebTab();
      return;
    }
    const nextTabs = resourceTabs.tabs.filter((t) => t.id !== tabId);
    resourceTabs.setTabs(nextTabs);
    if (resourceTabs.activeTabId === tabId) {
      const closedIndex = resourceTabs.tabs.findIndex((t) => t.id === tabId);
      const nextActive = nextTabs[Math.max(0, closedIndex - 1)] || nextTabs[0];
      resourceTabs.setActiveTabId(nextActive.id);
    }
  };

  const handleAddNewTab = () => {
    if (!isWebTabOpen) {
      onToggleWebTab();
    }
    setIsResourcePickerOpen((previous) => !previous);
  };

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
      {/* ── 1. UNIFIED APP HEADER WITH ORCA CONTROLS ── */}
      <AppHeader
        mode={mode}
        workspaceTitle={activeSession.title}
        activeWorkspace={activeWorkspace}
        onSwitchMode={onSwitchMode}
        isSidebarCollapsed={isSidebarCollapsed}
        onToggleSidebar={onToggleSidebar}
        onShowToast={onShowToast}
        splitPercent={splitPercent}
        isWebTabOpen={isWebTabOpen}
        isWebTabExpanded={isWebTabExpanded}
        onToggleWebTab={onToggleWebTab}
        onToggleExpandWebTab={onToggleExpandWebTab}
        resourceTabs={resourceTabs}
        onCloseTab={handleCloseTab}
        onAddNewTab={handleAddNewTab}
        lensBadge={lensMeta.badge}
        lensIconColor={lensMeta.iconColor}
        onOpenCommandPalette={() => setIsCommandPaletteOpen(true)}
        attentionCount={attentionItems.length}
        onOpenAttention={() => {
          if (attentionItems.length > 0) {
            onShowToast?.(`Active attention request: ${attentionItems[0].title}`);
          }
        }}
        onOpenKanban={() => openTab('kanban', 'Fleet Kanban', 'orca://kanban')}
        runningJobsByTab={runningJobsByTab}
      />

      {/* ── 2. SUPERVISORY ATTENTION STRIP (OrCa Attention + SADE Authority Gate) ── */}
      <AttentionBanner
        items={attentionItems}
        onDismiss={(id) => setAttentionItems((prev) => prev.filter((i) => i.id !== id))}
      />

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
          onOpenFullChatIn={onOpenFullChatIn}
          onOpenSessionHere={onOpenSessionHere}
          onDeleteSession={onDeleteConversation}
          newTaskLabel={lensMeta.taskLabel}
          newTaskIcon={Plus}
          newTaskIconColor={lensMeta.iconColor}
          onNewSession={onNewSession}
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
            className="flex flex-col h-full overflow-hidden bg-canvas relative select-text transition-all duration-150"
            style={{
              width: isWebTabExpanded
                ? '0%'
                : isWebTabOpen
                  ? `${splitPercent}%`
                  : '100%',
              display: isWebTabExpanded ? 'none' : 'flex'
            }}
          >
            {/* Dialogue Messages Canvas */}
            <div className="relative flex-1 min-h-0">
              <div
                ref={dialogueRef}
                onScroll={event => {
                  const element = event.currentTarget;
                  followBottom.current = element.scrollHeight - element.scrollTop - element.clientHeight < 80;
                  updateScrollEdges();
                }}
                className="h-full overflow-y-auto px-6 py-6 space-y-6"
              >
                {!hasMessages ? (
                  <div className="h-full flex flex-col items-center justify-center text-center my-auto pb-12">
                    <div className="w-14 h-14 rounded-xl bg-surface-1 border border-border-default flex items-center justify-center text-fg-muted mb-5">
                      {mode === 'engineering' ? (
                        <span className="font-mono text-2xl workbench-accent select-none">&#123;_&#125;</span>
                      ) : mode === 'research' ? (
                        <FlaskConical className="w-7 h-7 workbench-accent" />
                      ) : (
                        <MessageSquare className="w-7 h-7 workbench-accent" />
                      )}
                    </div>
                    <div className="workbench-kicker mb-2">{lensMeta.badge} workbench</div>
                    <h1 className="text-xl md:text-2xl font-semibold text-fg-editor tracking-[-0.02em] max-w-md">
                      {currentProject === 'Custos Workspace' ? (
                        mode === 'engineering' ? 'What should we build?' : mode === 'research' ? 'What should we investigate?' : 'How can Custos help?'
                      ) : (
                        <>
                          {mode === 'engineering' ? 'What should we build in ' : mode === 'research' ? 'What should we investigate in ' : 'How can Custos help with '}
                          <span className="underline decoration-[var(--color-border-emphasis)] underline-offset-4">{currentProject}</span>?
                        </>
                      )}
                    </h1>
                    <p className="text-[12px] text-fg-muted mt-2 max-w-sm">
                      Each workbench keeps its own conversations. Open a full saved chat in another workbench when you want to continue there.
                    </p>
                  </div>
                ) : (
                  <div className="space-y-6 max-w-2xl mx-auto w-full">
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

              <div aria-hidden="true" className={`conversation-scroll-blur conversation-scroll-blur-top ${scrollEdges.top ? 'is-visible' : ''}`} />
              <div aria-hidden="true" className={`conversation-scroll-blur conversation-scroll-blur-bottom ${scrollEdges.bottom ? 'is-visible' : ''}`} />
            </div>

            {/* DOCKED CHAT COMPOSER (Natural Flex Child) */}
            <div className="shrink-0 p-4 bg-canvas">
              {liveMessage && (
                <div className="max-w-2xl mx-auto mb-2.5">
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
              <div className="workbench-composer max-w-2xl mx-auto p-3 transition group relative">
                <SlashCommandMenu
                  query={inputText}
                  mode={mode}
                  isOpen={inputText.startsWith('/') && (!inputText.includes(' ') || inputText.length <= 15)}
                  onSelectCommand={handleSelectSlashCommand}
                  onClose={() => { }}
                />
                <textarea
                  ref={textareaRef}
                  rows={2}
                  value={inputText}
                  onChange={(e) => setInputText(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={
                    liveMessage
                      ? 'Agent is working… Type guidance here to steer agent in-flight (or press Stop/Esc)...'
                      : lensMeta.placeholder
                  }
                  className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-fg-editor placeholder-fg-subtle text-[13px] resize-none leading-relaxed"
                />

                <div className="flex items-center justify-between mt-2 pt-2">
                  <div className="flex items-center gap-2 text-fg-muted">
                    <button
                      onClick={handleAddNewTab}
                      className="p-1 hover:text-fg-editor transition rounded"
                      title="Launch agent or open tab"
                    >
                      <Plus className="w-4 h-4" />
                    </button>
                    <span className={`text-[11px] font-mono border border-border-default bg-surface-1 px-2 py-0.5 rounded ${lensMeta.iconColor}`}>
                      {lensMeta.badge}
                    </span>
                    {activeSession.nativeHarnessId ? (
                      <span className="rounded border border-border-default px-2 py-1 text-xs text-fg-editor">
                        Native · {activeSession.nativeHarnessId}
                      </span>
                    ) : (
                      <ModelSelector
                        currentModel={activeSession?.model || selectedModel}
                        onSelectModel={handleSelectModel}
                      />
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    <button
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
                      disabled={!liveMessage && !inputText.trim()}
                      aria-label={
                        liveMessage
                          ? inputText.trim()
                            ? 'Steer running agent'
                            : 'Stop response'
                          : 'Send message'
                      }
                      title={
                        liveMessage
                          ? inputText.trim()
                            ? 'Steer running agent (send guidance mid-run)'
                            : 'Stop response'
                          : 'Send message'
                      }
                      className={`p-1.5 rounded-lg transition ${
                        liveMessage
                          ? inputText.trim()
                            ? 'bg-amber-500/10 text-amber-400 hover:bg-amber-500/20'
                            : 'bg-transparent text-rose-400 hover:opacity-80'
                          : inputText.trim()
                          ? 'bg-transparent workbench-accent hover:opacity-80'
                          : 'text-fg-muted bg-transparent cursor-not-allowed opacity-60'
                      }`}
                    >
                      {liveMessage ? (
                        inputText.trim() ? (
                          <Compass className="w-4 h-4 text-amber-400 animate-spin-slow" aria-hidden="true" />
                        ) : (
                          <StopIcon className="w-4 h-4 fill-current" aria-hidden="true" />
                        )
                      ) : (
                        <Send className={`w-4 h-4 ${inputText.trim() ? 'fill-current' : ''}`} aria-hidden="true" />
                      )}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* DRAG HANDLE SPLITTER */}
          {isWebTabOpen && !isWebTabExpanded && (
            <div
              className="w-px bg-[var(--color-border-muted)] hover:bg-[var(--workbench-accent)] cursor-col-resize shrink-0 z-30 transition-colors"
              onMouseDown={() => {
                isDragging.current = true;
              }}
              onDoubleClick={() => setSplitPercent(45)}
              title="Drag to resize (double click to reset)"
            />
          )}

          {/* COLUMN 2: RIGHT RESOURCE CANVAS & TABBED CONTAINER */}
          {isWebTabOpen && (
            <div
              className="h-full shrink-0 bg-surface-0 relative z-20 flex flex-col overflow-hidden"
              style={{
                width: isWebTabExpanded ? '100%' : `${100 - splitPercent}%`
              }}
            >
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
                onAskAgent={handleResourceAskAgent}
                activeWorkspaceId={activeWorkspaceId}
                onSelectWorkspace={setActiveWorkspaceId}
                activeWorkspace={activeWorkspace}
                workspaces={workspaces}
              />
              <TabQuickLaunchMenu
                isOpen={isResourcePickerOpen}
                onClose={() => setIsResourcePickerOpen(false)}
                pickerResources={pickerResources}
                onOpenResource={(resId) => {
                  handleOpenTab(resId);
                  setIsResourcePickerOpen(false);
                }}
                onLaunchAgent={handleLaunchAgentTab}
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

    </div>
  );
};
