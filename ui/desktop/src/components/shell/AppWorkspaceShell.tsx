import StopIcon from '~icons/lucide/square';
import React, { useState, useRef, useEffect, useMemo } from 'react';
import {
  Send,
  FolderGit2,
  Orbit,
  Plus,
  BookOpen,
  FlaskConical,
  MessageSquare,
  Sparkles,
  Layers,
  ShieldCheck,
  Bot,
  Activity
} from 'lucide-react';
import { Session } from '@/types';
import { OrcaTabbedContainer, type ResourceTabsState, type OrcaTabId } from '@/components/views/OrcaTabbedContainer';
import { WorkspaceSidebar } from '@/components/shell/WorkspaceSidebar';
import { AppHeader, type AppWorkspaceMode } from '@/components/shell/AppHeader';
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
import { formatKeyCombo } from '@/lib/utils';
import { SlashCommandMenu, SLASH_COMMANDS, type SlashCommandDef } from '@/components/chat/SlashCommandMenu';
import { daemonClient } from '@/api/daemon_client';
import { OpenAIIcon, ClaudeIcon, GeminiIcon, ProviderBrandIcon } from '@/components/common/AgentIcons';

const renderAgentIcon = (author?: string, currentMode?: AppWorkspaceMode) => {
  const name = (author || '').toLowerCase();
  if (name.includes('claude') || name.includes('anthropic')) return <ClaudeIcon size={13} />;
  if (name.includes('openai') || name.includes('gpt') || name.includes('codex')) return <OpenAIIcon size={13} />;
  if (name.includes('gemini') || name.includes('google')) return <GeminiIcon size={13} />;
  if (name.includes('antigravity')) return <ProviderBrandIcon provider="antigravity" size={13} />;
  if (name.includes('deepseek')) return <ProviderBrandIcon provider="deepseek" size={13} />;
  if (currentMode === 'code') return <OpenAIIcon size={13} />;
  if (currentMode === 'research') return <ClaudeIcon size={13} />;
  return <GeminiIcon size={13} />;
};


interface AppWorkspaceShellProps {
  mode: AppWorkspaceMode;
  onSwitchMode: (mode: AppWorkspaceMode) => void;
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onOpenFullChatIn?: (mode: AppWorkspaceMode) => void;
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
}

export const AppWorkspaceShell: React.FC<AppWorkspaceShellProps> = ({
  mode,
  onSwitchMode,
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  activeSessionId,
  onSelectSession,
  onOpenFullChatIn,
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
  openResources
}) => {
  const { openSettings, selectedModel, handleSelectModel, handleStopRun } = useAppContext();
  const [inputText, setInputText] = useState('');
  const [splitPercent, setSplitPercent] = useState<number>(() => {
    return mode === 'chat' ? 50 : 42;
  });
  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false);

  // Populated only from daemon approval/effect events. Never fabricate authority requests.
  const [attentionItems, setAttentionItems] = useState<AttentionItem[]>([]);

  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  // Active session lookup
  const activeSession = sessions.find((s) => s.id === activeSessionId) || {
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
          onShowToast?.(`Opened Fleet Kanban (${formatKeyCombo({ ctrlOrCmd: true, alt: true, key: 'K' })})`);
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

  const liveMessage = activeSession.messages.find(message => message.runId && !['completed','failed','cancelled','interrupted'].includes(message.runStatus || ''));
  const dialogueRef = useRef<HTMLDivElement>(null);
  const followBottom = useRef(true);
  useEffect(() => { followBottom.current = true; }, [activeSessionId]);
  useEffect(() => {
    if (followBottom.current && dialogueRef.current) dialogueRef.current.scrollTop = dialogueRef.current.scrollHeight;
  }, [activeSession.messages]);

  const { jobs, dispatchJob, updateJob } = useJobRegistry();

  const executeSlashCommand = (cmd: SlashCommandDef, args: string) => {
    setInputText('');

    // 1. Dispatch in JobRegistry so active tabs & sidebar show running status immediately
    const entryId = dispatchJob({
      jobId: cmd.jobId,
      jobName: cmd.label,
      sourceTabId: mode === 'code' ? 'changes' : mode === 'research' ? 'literature' : undefined,
    });

    // 2. Transmit to session chat so conversation timeline captures user action transparently
    const formattedPrompt = args.trim()
      ? `${cmd.command} ${args.trim()}`
      : `${cmd.command} - ${cmd.description}`;
    onSendMessage(formattedPrompt);

    // 3. Trigger daemon packs job
    daemonClient
      .runPacksJob({
        jobId: cmd.jobId,
        sessionId: activeSession.id,
        inputs: cmd.getInputs(args, { sessionId: activeSession.id }),
      })
      .then((res) => {
        const taskId = res.taskId || res.task_id || 'unknown';
        updateJob(entryId, { taskId, status: 'done', endedAt: Date.now() });
        onShowToast?.(`Executed ${cmd.command} (Task ${taskId})`);
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
    if (e.key === 'Enter' && !e.shiftKey) {
      // If a slash menu might be selecting, let enter submit command if handled
      e.preventDefault();
      handleSend();
    }
  };

  const openTab = (id: OrcaTabId, title: string, url: string) => {
    resourceTabs.setTabs((prev) => (prev.some((t) => t.id === id) ? prev : [...prev, { id, title, url }]));
    resourceTabs.setActiveTabId(id);
    openResources(false);
  };
  const runningJobsByTab = useMemo(() => {
    const counts: Partial<Record<OrcaTabId, number>> = {};
    jobs.forEach((j) => {
      if ((j.status === 'running' || j.status === 'queued') && j.sourceTabId) {
        counts[j.sourceTabId] = (counts[j.sourceTabId] || 0) + 1;
      }
    });
    return counts as Record<OrcaTabId, number>;
  }, [jobs]);

  const handleOpenTab = (id: OrcaTabId) => {
    const existing = resourceTabs.tabs.find((t) => t.id === id);
    if (existing) {
      resourceTabs.setActiveTabId(id);
    } else {
      openTab(id, id.charAt(0).toUpperCase() + id.slice(1), `custos://resource/${id}`);
    }
    openResources(false);
  };

  const handleCloseTab = (e: React.MouseEvent, tabId: OrcaTabId) => {
    e.stopPropagation();
    if (resourceTabs.tabs.length <= 1) {
      resourceTabs.setTabs([{ id: 'tools', title: 'Resources', url: 'custos://resources' }]);
      resourceTabs.setActiveTabId('tools');
      return;
    }
    const nextTabs = resourceTabs.tabs.filter((t) => t.id !== tabId);
    resourceTabs.setTabs(nextTabs);
    if (resourceTabs.activeTabId === tabId) {
      resourceTabs.setActiveTabId(nextTabs[nextTabs.length - 1].id);
    }
  };

  const handleAddNewTab = () => {
    resourceTabs.setTabs((prev) => (prev.some((t) => t.id === 'tools') ? prev : [...prev, { id: 'tools', title: 'Resources', url: 'custos://resources' }]));
    resourceTabs.setActiveTabId('tools');
  };

  // Lens metadata & personality
  const lensMeta = {
    chat: {
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
    code: {
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

  // Dynamic Sidebar Tools per Mode
  const sidebarTools = [
    ...(mode === 'code'
      ? [
          {
            icon: FolderGit2,
            label: 'Workspace Explorer',
            onClick: () => openTab('files', 'Files', 'custos://files')
          },
          {
            icon: Orbit,
            label: 'SADE Automations',
            onClick: () => onShowToast?.('SADE Orbits & Automations')
          }
        ]
      : []),
    ...(mode === 'research'
      ? [
          {
            icon: BookOpen,
            label: 'Literature Corpus',
            onClick: () => openTab('literature', 'Sources', 'custos://resource/literature')
          },
          {
            icon: ShieldCheck,
            label: 'Claims Matrix',
            onClick: () => openTab('claims', 'Claims', 'custos://resource/claims')
          },
          {
            icon: FlaskConical,
            label: 'Research Methods',
            onClick: () => openTab('knowledge', 'Methods', 'custos://resource/knowledge')
          },
          {
            icon: Activity,
            label: 'Runs Ledger',
            onClick: () => openTab('runs', 'Runs', 'custos://resource/runs')
          },
          {
            icon: Layers,
            label: 'Synthesis & Handoff',
            onClick: () => openTab('synthesis', 'Synthesis', 'custos://resource/synthesis')
          }
        ]
      : []),
    ...(mode === 'chat'
      ? [
          {
            icon: Layers,
            label: 'Delegate Subtask',
            onClick: () => onShowToast?.('Delegation requires the daemon workflow API; no task was created.')
          },
          {
            icon: Sparkles,
            label: 'Skills Catalog',
            onClick: () => onShowToast?.('Skills Catalog')
          }
        ]
      : []),
    {
      icon: Bot,
      label: 'Fleet Kanban',
      onClick: () => openTab('kanban', 'Fleet Kanban', 'orca://kanban'),
      rightElement: <span className="text-[10px] text-fg-muted font-mono">{formatKeyCombo({ ctrlOrCmd: true, alt: true, key: 'K' })}</span>
    },
  ];

  return (
    <div className="custos-workspace flex flex-col h-full w-full font-sans overflow-hidden select-none" data-workbench={mode}>
      {/* ── 1. UNIFIED APP HEADER WITH ORCA CONTROLS ── */}
      <AppHeader
        mode={mode}
        workspaceTitle={activeSession.title}
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
          currentProject={currentProject}
          projectNames={projectNames}
          onSelectProject={onSelectProject}
          sessions={sessions}
          activeSessionId={activeSessionId}
          onSelectSession={onSelectSession}
          onOpenFullChatIn={onOpenFullChatIn}
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
            <div ref={dialogueRef} onScroll={event => { const element = event.currentTarget; followBottom.current = element.scrollHeight - element.scrollTop - element.clientHeight < 80; }} className="h-full overflow-y-auto px-6 py-6 space-y-6">
              {!hasMessages ? (
                <div className="h-full flex flex-col items-center justify-center text-center my-auto pb-12">
                  <div className="w-14 h-14 rounded-xl bg-surface-1 border border-border-default flex items-center justify-center text-fg-muted mb-5">
                    {mode === 'code' ? (
                      <span className="font-mono text-2xl workbench-accent select-none">&#123;_&#125;</span>
                    ) : mode === 'research' ? (
                      <FlaskConical className="w-7 h-7 workbench-accent" />
                    ) : (
                      <MessageSquare className="w-7 h-7 workbench-accent" />
                    )}
                  </div>
                  <div className="workbench-kicker mb-2">{lensMeta.badge} workbench</div>
                  <h1 className="text-xl md:text-2xl font-semibold text-fg-editor tracking-[-0.02em] max-w-md">
                    {mode === 'code'
                      ? `What should we build in `
                      : mode === 'research'
                      ? `What hypothesis are we testing in `
                      : `How can Custos assist with `}
                    <span className="text-fg-editor underline decoration-[var(--color-border-emphasis)] underline-offset-4">{currentProject}</span>?
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
                        className="space-y-2.5 pb-6 border-b border-border-muted/40 last:border-b-0 min-w-0"
                      >
                        {/* Message Header */}
                        <div className="flex items-center gap-2.5 text-xs">
                          {isUser ? (
                            <div className={`w-5 h-5 rounded-md ${lensMeta.avatarBg} text-fg-on-emphasis font-bold text-[9px] flex items-center justify-center shrink-0 shadow-xs`}>
                              {lensMeta.avatarText}
                            </div>
                          ) : (
                            <div className="w-5 h-5 rounded-md bg-surface-2 text-fg-editor flex items-center justify-center shrink-0 p-0.5 border border-border-default shadow-xs">
                              {renderAgentIcon(msg.author, mode)}
                            </div>
                          )}
                          <span className="font-semibold text-fg-editor text-[12.5px]">
                            {isUser ? 'You' : msg.author || 'Custos Agent'}
                          </span>
                          {msg.badge && (
                            <span className="px-1.5 py-0.5 rounded text-[10px] bg-surface-1 text-fg-muted border border-border-default font-mono">
                              {msg.badge}
                            </span>
                          )}
                          {msg.runId && !['completed', 'failed', 'cancelled', 'interrupted'].includes(msg.runStatus || '') && (
                            <button
                              type="button"
                              onClick={() => void handleStopRun(msg.runId!)}
                              className="rounded border border-border-default px-2 py-0.5 text-[10px] text-fg-editor hover:bg-surface-2 transition flex items-center gap-1"
                            >
                              <StopIcon className="w-2.5 h-2.5" /> Stop
                            </button>
                          )}
                        </div>

                        {/* Message Content */}
                        <div className="conversation-markdown text-[13px] leading-relaxed text-fg-editor pl-7 space-y-3 min-w-0 break-words [overflow-wrap:anywhere]">
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
                            {msg.text}
                          </ReactMarkdown>
                          {!msg.text && msg.runId && !['completed','failed','cancelled','interrupted'].includes(msg.runStatus || '') && <p role="status" className="text-xs text-fg-muted">{msg.runStatus === 'thinking' ? 'Thinking...' : 'Running...'}</p>}
                          {msg.liveError && <p role="alert" className="mt-2 text-xs text-amber-400">{msg.liveError}</p>}
                          {msg.activities?.map((activity, index) => <details key={`${msg.runId}-activity-${index}`} className="mt-2 rounded-lg border border-border-muted bg-surface-1 p-2 text-xs"><summary className="cursor-pointer text-fg-muted">{activity.kind === 'reasoning_summary' ? 'Reasoning summary' : activity.kind === 'tool_input' ? 'Tool input' : 'Tool output'}</summary><pre className="mt-2 whitespace-pre-wrap break-words max-h-60 overflow-auto text-[11px]">{JSON.stringify(activity.payload, null, 2)}</pre></details>)}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
            </div>

            {/* DOCKED CHAT COMPOSER (Natural Flex Child) */}
            <div className="shrink-0 p-4 bg-canvas">
              <div className="workbench-composer max-w-2xl mx-auto p-3 transition group relative">
                <SlashCommandMenu
                  query={inputText}
                  mode={mode}
                  isOpen={inputText.startsWith('/') && (!inputText.includes(' ') || inputText.length <= 15)}
                  onSelectCommand={handleSelectSlashCommand}
                  onClose={() => {}}
                />
                <textarea
                  rows={2}
                  value={inputText}
                  onChange={(e) => setInputText(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={lensMeta.placeholder}
                  className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-fg-editor placeholder-fg-subtle text-[13px] resize-none leading-relaxed"
                />

                <div className="flex items-center justify-between mt-2 pt-2 border-t border-border-muted">
                  <div className="flex items-center gap-2 text-fg-muted">
                    <button
                      onClick={() => openResources(false)}
                      className="p-1 hover:text-fg-editor transition rounded"
                      title="Open Resources"
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
                      onClick={() => liveMessage?.runId ? void handleStopRun(liveMessage.runId) : handleSend()}
                      disabled={!liveMessage && !inputText.trim()}
                      aria-label={liveMessage ? 'Stop response' : 'Send message'}
                      title={liveMessage ? 'Stop response' : 'Send message'}
                      className={`p-1.5 rounded-lg transition ${
                        liveMessage
                          ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
                          : inputText.trim()
                          ? 'workbench-primary-action'
                          : 'text-fg-muted bg-surface-2 cursor-not-allowed opacity-60'
                      }`}
                    >
                      {liveMessage ? (
                        <StopIcon className="w-4 h-4 fill-current" aria-hidden="true" />
                      ) : (
                        <Send className="w-4 h-4" aria-hidden="true" />
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
