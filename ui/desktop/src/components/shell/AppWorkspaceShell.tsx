import React, { useState, useRef, useEffect } from 'react';
import {
  Send,
  Terminal,
  FolderGit2,
  GitBranch,
  Orbit,
  Settings2,
  Plus,
  BookOpen,
  FlaskConical,
  MessageSquare,
  Sparkles,
  Minimize2,
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
import { OpenAIIcon, ClaudeIcon } from '@/components/common/AgentIcons';
import { ModelSelector } from '@/components/chat/ModelSelector';
import { useAppContext } from '@/context/AppContext';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { formatKeyCombo } from '@/lib/utils';

interface AppWorkspaceShellProps {
  mode: AppWorkspaceMode;
  onSwitchMode: (mode: AppWorkspaceMode) => void;
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onNewSession: () => void;
  onSendMessage: (text: string) => void;
  onClearHistory: () => void;
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
  onNewSession,
  onSendMessage,
  onClearHistory: _onClearHistory,
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
  const { openSettings, selectedModel, handleSelectModel } = useAppContext();
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
  const activeSession = sessions.find((s) => s.id === activeSessionId) || sessions[0] || {
    id: 'default',
    title: 'New Session',
    messages: []
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

  const handleSend = () => {
    if (!inputText.trim()) return;
    onSendMessage(inputText);
    setInputText('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const openTab = (id: OrcaTabId, title: string, url: string) => {
    resourceTabs.setTabs((prev) => (prev.some((t) => t.id === id) ? prev : [...prev, { id, title, url }]));
    resourceTabs.setActiveTabId(id);
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
            icon: GitBranch,
            label: 'Worktrees',
            onClick: () => openTab('worktrees', 'Worktrees', 'custos://worktrees')
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
    {
      icon: Settings2,
      label: 'Preferences & Settings',
      onClick: () => openSettings('general')
    }
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
            <div className="flex-1 overflow-y-auto px-6 py-6 space-y-6">
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
                    One conversation, three workbenches. Sources, changes and evidence stay attached to this session.
                  </p>
                </div>
              ) : (
                <div className="space-y-6 max-w-2xl mx-auto w-full">
                  {activeSession.messages.map((msg, i) => {
                    const isUser = msg.role === 'user';
                    return (
                      <div key={msg.id || i} className="space-y-2">
                        <div className="flex items-center gap-2 text-xs">
                          {isUser ? (
                            <div className={`w-5 h-5 rounded-md ${lensMeta.avatarBg} text-fg-on-emphasis font-bold text-[9px] flex items-center justify-center shrink-0`}>
                              {lensMeta.avatarText}
                            </div>
                          ) : (
                            <div className="w-5 h-5 rounded-md bg-surface-2 text-fg-editor flex items-center justify-center shrink-0 p-0.5 border border-border-default">
                              {mode === 'code' ? <OpenAIIcon size={13} /> : <ClaudeIcon size={13} />}
                            </div>
                          )}
                          <span className="font-semibold text-fg-editor">{isUser ? 'You' : msg.author || 'Custos'}</span>
                          {msg.badge && (
                            <span className="px-1.5 py-0.2 rounded text-[10px] bg-surface-1 text-fg-muted border border-border-default">
                              {msg.badge}
                            </span>
                          )}
                        </div>

                        <div className="text-[13px] leading-relaxed text-fg-editor space-y-3">
                          <ReactMarkdown
                            remarkPlugins={[remarkGfm]}
                            components={{
                              code({ className, children, ...props }) {
                                const match = /language-(\w+)/.exec(className || '');
                                if (match) {
                                  return (
                                    <div className="my-3 rounded-xl overflow-hidden bg-surface-0 border border-border-default font-mono text-[12px]">
                                      <div className="px-3 py-1.5 bg-surface-1 text-[10px] text-fg-muted flex items-center gap-2">
                                        <Terminal className="w-3 h-3" />
                                        {match[1]}
                                      </div>
                                      <pre className="p-3.5 overflow-x-auto text-fg-editor">
                                        <code>{String(children).replace(/\n$/, '')}</code>
                                      </pre>
                                    </div>
                                  );
                                }
                                return (
                                  <code className="px-1.5 py-0.5 rounded bg-surface-1 text-fg-editor border border-border-muted font-mono text-[12px]" {...props}>
                                    {children}
                                  </code>
                                );
                              }
                            }}
                          >
                            {msg.text}
                          </ReactMarkdown>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            {/* DOCKED CHAT COMPOSER (Natural Flex Child) */}
            <div className="shrink-0 p-4 bg-canvas">
              <div className="workbench-composer max-w-2xl mx-auto p-3 transition group">
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
                    <ModelSelector
                      currentModel={activeSession?.model || selectedModel}
                      onSelectModel={handleSelectModel}
                    />
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleSend}
                      disabled={!inputText.trim()}
                      className={`p-1.5 rounded-lg transition ${
                        inputText.trim()
                          ? 'workbench-primary-action'
                          : 'text-fg-muted bg-surface-2'
                      }`}
                    >
                      <Send className="w-4 h-4" />
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
              />
            </div>
          )}

          {/* FLOATING MINI DOCK (Only visible when tab canvas is maximized) */}
          {isWebTabExpanded && (
            <div className="absolute z-50 bottom-5 right-5 w-[380px] pointer-events-auto animate-in slide-in-from-bottom-2 duration-150">
              <div className="bg-surface-1/95 backdrop-blur-xl border border-border-default rounded-2xl p-3 shadow-[0_12px_45px_-10px_rgba(0,0,0,0.85)] focus-within:border-border-emphasis">
                <div className="flex items-center justify-between pb-1.5 mb-1.5 border-b border-border-muted text-[11px] text-fg-muted">
                  <div className="flex items-center gap-1.5">
                    <span className={`w-2 h-2 rounded-full ${lensMeta.activeColor}`} />
                    <span className="font-medium text-fg-editor">{lensMeta.badge} (Floating)</span>
                  </div>
                  <button
                    onClick={onToggleExpandWebTab}
                    className="hover:text-fg-editor flex items-center gap-1 text-[10px] bg-surface-2 px-1.5 py-0.5 rounded transition"
                    title="Restore split layout"
                  >
                    <Minimize2 size={11} /> Restore Split
                  </button>
                </div>

                <textarea
                  rows={1}
                  value={inputText}
                  onChange={(e) => setInputText(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={lensMeta.placeholder}
                  className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-fg-editor placeholder-fg-subtle text-[13px] resize-none"
                />

                <div className="flex items-center justify-between mt-1.5 pt-1.5 border-t border-border-muted">
                  <ModelSelector
                    currentModel={activeSession?.model || selectedModel}
                    onSelectModel={handleSelectModel}
                    compact
                  />
                  <button
                    onClick={handleSend}
                    disabled={!inputText.trim()}
                    className={`p-1 rounded-lg transition ${
                      inputText.trim()
                        ? `${lensMeta.activeColor} text-fg-on-emphasis hover:opacity-90`
                        : 'text-fg-muted bg-surface-2'
                    }`}
                  >
                    <Send className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            </div>
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
