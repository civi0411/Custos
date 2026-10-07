import React, { useState, useRef, useEffect } from 'react';
import {
  Plus,
  FolderGit2,
  Layers,
  Settings2,
  Mic,
  AudioWaveform,
  Bot,
  Send,
  RotateCcw,
  Check,
  Copy,
  ChevronRight,
  FlaskConical,
  Terminal,
  ShieldCheck,
  ChevronDown,
  LayoutGrid,
  X,
  Download
} from 'lucide-react';
import { Session } from '@/types';
import { WorkspaceSidebar } from '@/components/shell/WorkspaceSidebar';
import { DelegateTaskModal, type DelegateTaskData } from '@/components/modals';
import { OrcaTabbedContainer, type ResourceTabsState } from './OrcaTabbedContainer';
import { AssistantWorkspace } from '@/components/workspaces';
import { ClaudeIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface ClaudeChatViewProps {
  resourceTabs: ResourceTabsState;
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onNewSession: () => void;
  onSendMessage: (text: string) => void;
  onClearHistory: () => void;
  onShowToast?: (msg: string) => void;
  onOpenSettings?: () => void;
  onSwitchMode?: (mode: 'chat' | 'code' | 'research') => void;
  isSidebarCollapsed?: boolean;
  isWebTabOpen?: boolean;
  isWebTabExpanded?: boolean;
  onToggleExpandWebTab?: () => void;
  onToggleWebTab?: () => void;
}

export const ClaudeChatView: React.FC<ClaudeChatViewProps> = ({
  resourceTabs,
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
  onSendMessage,
  onClearHistory,
  onShowToast,
  onOpenSettings,
  onSwitchMode,
  isSidebarCollapsed,
  isWebTabOpen,
  isWebTabExpanded,
  onToggleExpandWebTab,
  onToggleWebTab
}) => {
  const { openSettings } = useAppContext();
  // Chat vs Cowork mode toggle in composer
  const [composerMode, setComposerMode] = useState<'chat' | 'cowork'>('chat');

  // Split pane state
  const [splitPercent, setSplitPercent] = useState(50);
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      let newPercent = ((e.clientX - rect.left) / rect.width) * 100;
      newPercent = Math.max(20, Math.min(80, newPercent));
      setSplitPercent(newPercent);
    };
    const handleMouseUp = () => { isDragging.current = false; };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);
  const [inputText, setInputText] = useState('');
  const [selectedModel, setSelectedModel] = useState('Sonnet 5.5 Medium');
  const [isModelDropdownOpen, setIsModelDropdownOpen] = useState(false);
  const [expandedTrace, setExpandedTrace] = useState<Record<string, boolean>>({});
  const [copiedCodeId, setCopiedCodeId] = useState<string | null>(null);

  // Vinh's SADE Cowork integrations
  const [isDelegateModalOpen, setIsDelegateModalOpen] = useState(false);
  const [showFleetWorkbench, setShowFleetWorkbench] = useState(false);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Dynamic day of the week
  const weekdayName = new Intl.DateTimeFormat('en-US', { weekday: 'long' }).format(new Date());

  // ── ALWAYS use real session data from props (sync with Code/Research modes) ──
  const activeSession = sessions.find((s) => s.id === activeSessionId) ?? sessions[0] ?? null;
  const hasMessages = (activeSession?.messages?.length ?? 0) > 0;

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [activeSession?.messages]);

  const handleSend = () => {
    if (!inputText.trim()) return;
    onSendMessage(inputText.trim());
    setInputText('');
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleCopy = (code: string, cid: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCodeId(cid);
    setTimeout(() => setCopiedCodeId(null), 2000);
  };

  const toggleTrace = (msgId: string) => {
    setExpandedTrace((prev) => ({ ...prev, [msgId]: !prev[msgId] }));
  };

  const exportConversation = () => {
    if (!activeSession) return;
    const md = activeSession.messages
      .map(m => `## ${m.role === 'user' ? 'User (Chí Vĩ)' : (m.author || 'Claude')}\n\n${m.text}`)
      .join('\n\n---\n\n');
    const blob = new Blob([`# ${activeSession.title}\n\n${md}\n`], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${activeSession.title.replace(/[^a-z0-9-_]+/gi, '-').toLowerCase()}.md`;
    a.click();
    URL.revokeObjectURL(url);
    if (onShowToast) onShowToast('Conversation exported to Markdown');
  };

  const handleDelegateSubmit = (taskData: DelegateTaskData) => {
    setIsDelegateModalOpen(false);
    onSendMessage(
      `[DELEGATED SADE TASK]: **${taskData.title}**\n- Strategy: ${taskData.strategy}\n- Model: ${taskData.model}\n- Budget: $${taskData.budgetUsd.toFixed(2)}\n- Goal: ${taskData.goal}`
    );
    if (onShowToast) onShowToast(`Delegated "${taskData.title}" to SADE agents`);
  };

  // ── STARTER PROMPTS matching Screenshot 1 ──
  const STARTERS = [
    {
      icon: Terminal,
      title: 'Claude Code IDE',
      desc: 'Implement invariant-checked permit gate in Rust',
      prompt: 'Implement an atomic permit acquisition gate in crates/custos-runtime/src/workflow/dispatcher.rs enforcing INV-01 double-dispatch fencing.'
    },
    {
      icon: FlaskConical,
      title: 'Claude Science Lab',
      desc: 'Extract empirical claims from arXiv literature',
      prompt: 'Analyze arXiv:2403.1189 and extract verified claims regarding non-repudiation cryptographic ledgers for autonomous multi-agent systems.'
    },
    {
      icon: ShieldCheck,
      title: 'SADE Safety Invariants',
      desc: 'Verify zero-IO proof closures & capability leases',
      prompt: 'Run an invariant audit across active worktrees to verify no external network or file mutations occurred without signed capability tickets.'
    }
  ];

  return (
    <div className="flex h-full w-full bg-[#1e1e1e] text-[#e6edf3] font-sans overflow-hidden select-none">
      <WorkspaceSidebar
        currentProject={currentProject}
        projectNames={projectNames}
        onSelectProject={onSelectProject}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        newTaskLabel="New"
        newTaskIcon={Plus}
        onNewSession={onNewSession}
        tools={[
          {
            icon: FolderGit2,
            label: 'Projects',
            onClick: () => onShowToast?.('Projects repository')
          },
          {
            icon: Layers,
            label: 'Artifacts',
            onClick: () => onShowToast?.('Artifacts & Documents')
          },
          {
            icon: Bot,
            label: 'Delegate Task',
            onClick: () => setIsDelegateModalOpen(true),
            rightElement: (
              <span className="text-[10px] bg-[#cc785c]/20 text-[#cc785c] px-1.5 py-0.5 rounded font-mono">
                SADE
              </span>
            )
          },
          {
            icon: Settings2,
            label: 'Preferences & Settings',
            onClick: () => openSettings('general'),
          }
        ]}
        sessionsTitle="Chats and tasks"
        emptySessionsMessage='No sessions yet. Click "New" to create one.'
        avatarColor="bg-[#cc785c]"
        avatarText="CV"
        onShowToast={onShowToast}
        onOpenSettings={onOpenSettings}
        onSwitchMode={onSwitchMode}
        isSidebarCollapsed={isSidebarCollapsed}
      />

      {/* ─────────────────────────────────────────────────────────────
          2. Center Canvas — Greeting + Chat thread + Floating Composer
      ───────────────────────────────────────────────────────────── */}
      <div id="chat-canvas-wrapper" ref={containerRef} className="flex-1 flex overflow-hidden min-w-0 relative">
        <main 
          className="flex flex-col min-w-0 h-full relative overflow-hidden bg-[#1e1e1e]" 
          style={{ width: isWebTabExpanded ? '0%' : (isWebTabOpen ? `${splitPercent}%` : '100%'), display: isWebTabExpanded ? 'none' : 'flex' }}
        >
        {/* Cowork mode toolbar (only shown in cowork mode) */}
        {composerMode === 'cowork' && (
          <div className="h-10 px-6 flex items-center justify-between shrink-0 z-10 border-b border-[#262c36]/60 bg-[#161b22]/40">
            <div className="flex items-center gap-2">
              <span className="flex items-center gap-1.5 text-xs text-[#e6edf3] font-medium bg-[#21262d] border border-[#262c36] px-2 py-0.5 rounded-full">
                <Bot className="w-3 h-3 text-[#8b949e]" />
                <span>Cowork Mode</span>
              </span>
              <button
                onClick={() => setIsDelegateModalOpen(true)}
                className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-white text-black hover:bg-neutral-200 text-[11.5px] font-medium transition shadow-sm"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Delegate Task</span>
              </button>
              <button
                onClick={() => setShowFleetWorkbench(!showFleetWorkbench)}
                className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-[11.5px] font-medium transition ${
                  showFleetWorkbench
                    ? 'bg-[#2d333b] border-[#384252] text-white'
                    : 'bg-[#1c2128] border-[#262c36] text-[#8b949e] hover:text-white'
                }`}
              >
                <LayoutGrid className="w-3.5 h-3.5" />
                <span>{showFleetWorkbench ? 'Hide Fleet Lab' : 'Fleet Workbench'}</span>
              </button>
            </div>
          </div>
        )}

        {/* Collapsible Fleet Workbench (Vinh's AssistantWorkspace) */}
        {composerMode === 'cowork' && showFleetWorkbench && (
          <div className="h-80 border-b border-[#30363d] bg-[#161b22] overflow-hidden shrink-0 relative animate-in slide-in-from-top-2 duration-150">
            <div className="absolute top-2 right-3 z-30">
              <button
                onClick={() => setShowFleetWorkbench(false)}
                className="p-1 rounded bg-[#21262d] text-[#8b949e] hover:text-white hover:bg-[#30363d] transition"
                title="Collapse Fleet Lab"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <AssistantWorkspace />
          </div>
        )}

        {/* Scrollable Canvas */}
        <div className="flex-1 overflow-y-auto px-4 md:px-8 pb-44 select-text flex flex-col">
          {!hasMessages ? (
            /* Empty State: Dynamic Greeting */
            <div className="flex-1 flex flex-col items-center justify-center text-center pb-12">
              <div className="flex items-center justify-center gap-3 mb-6">
                <span className="text-3xl text-[#cc785c] select-none" aria-hidden="true">✳</span>
                <h1
                  className="text-3xl md:text-4xl text-[#ececec] font-normal tracking-tight"
                  style={{ fontFamily: 'Georgia, Cambria, "Times New Roman", Times, serif' }}
                >
                  Happy {weekdayName}, Chí Vĩ
                </h1>
              </div>

              <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-[#2b2b2b] border border-[#383838] text-[12px] text-[#b3b3b3] mb-8">
                <Bot className="w-3.5 h-3.5 text-[#cc785c]" />
                <span>
                  Toggle <strong className="text-white">Cowork</strong> to dispatch autonomous SADE agents
                </span>
              </div>

              {/* Starter Prompt Cards */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 w-full max-w-2xl">
                {STARTERS.map((s) => {
                  const Icon = s.icon;
                  return (
                    <button
                      key={s.title}
                      onClick={() => {
                        setInputText(s.prompt);
                        textareaRef.current?.focus();
                      }}
                      className="text-left p-4 rounded-xl bg-[#252525] border border-[#353535] hover:border-[#cc785c]/40 hover:bg-[#2c2c2c] transition group"
                    >
                      <div className="flex items-center gap-2 mb-2">
                        <Icon className="w-4 h-4 text-[#cc785c]" />
                        <span className="font-semibold text-[13px] text-white">{s.title}</span>
                      </div>
                      <p className="text-[12px] text-[#8a8a8a]">{s.desc}</p>
                    </button>
                  );
                })}
              </div>
            </div>
          ) : (
            /* Message Thread */
            <div className="max-w-3xl mx-auto w-full space-y-6 pt-4">
              <div className="flex justify-end pb-2 gap-3">
                {onSwitchMode && (
                  <>
                    <button
                      onClick={() => {
                        onSwitchMode('code');
                        if (onShowToast) onShowToast('Opened Coding lens. Select context to create an implementation step.');
                      }}
                      className="flex items-center gap-1.5 text-[11px] text-[#58a6ff] hover:text-white transition"
                      title="Open this conversation in Coding"
                    >
                      <Terminal className="w-3 h-3" />
                      <span>Open Coding</span>
                    </button>
                    <button
                      onClick={() => {
                        onSwitchMode('research');
                        if (onShowToast) onShowToast('Opened Research lens. Select context to create a research step.');
                      }}
                      className="flex items-center gap-1.5 text-[11px] text-[#a371f7] hover:text-white transition"
                      title="Open this conversation in Research"
                    >
                      <FlaskConical className="w-3 h-3" />
                      <span>Open Research</span>
                    </button>
                  </>
                )}
                <button
                  onClick={exportConversation}
                  className="flex items-center gap-1 text-[11px] text-[#7d7d7d] hover:text-[#b3b3b3] transition"
                  title="Export conversation as Markdown"
                >
                  <Download className="w-3 h-3 text-[#cc785c]" />
                  <span>Export .md</span>
                </button>
                <button
                  onClick={onClearHistory}
                  className="flex items-center gap-1 text-[11px] text-[#7d7d7d] hover:text-[#b3b3b3] transition"
                >
                  <RotateCcw className="w-3 h-3" />
                  <span>Clear conversation</span>
                </button>
              </div>

              {activeSession!.messages.map((msg, i) => {
                const isUser = msg.role === 'user';
                const traceKey = msg.id || `trace-${i}`;
                const isTraceExpanded = expandedTrace[traceKey] ?? false;
                return (
                  <div key={msg.id || i} className="space-y-2">
                    {/* Message Header */}
                    <div className="flex items-center gap-2 text-xs">
                      {isUser ? (
                        <div className="w-6 h-6 rounded-full bg-[#cc785c] text-white font-bold text-[10px] flex items-center justify-center shrink-0">
                          CV
                        </div>
                      ) : (
                        <div className="w-6 h-6 rounded-full bg-[#2b2b2b] flex items-center justify-center shrink-0 p-1">
                          <ClaudeIcon size={14} />
                        </div>
                      )}
                      <span className="font-semibold text-white">{isUser ? 'Chí Vĩ' : msg.author || 'Claude'}</span>
                      {msg.badge && (
                        <span className="px-1.5 py-0.2 rounded text-[10px] bg-[#2b2b2b] text-[#9c9c9c]">
                          {msg.badge}
                        </span>
                      )}
                    </div>

                    {/* Thinking trace */}
                    {msg.stepName && (
                      <div className="rounded-lg bg-[#242424] border border-[#333333] overflow-hidden text-xs ml-8">
                        <div
                          onClick={() => toggleTrace(traceKey)}
                          className="px-3 py-1.5 flex items-center justify-between cursor-pointer text-[#8a8a8a] hover:text-[#b3b3b3] transition"
                        >
                          <div className="flex items-center gap-2">
                            <ChevronRight className={`w-3.5 h-3.5 transition-transform ${isTraceExpanded ? 'rotate-90' : ''}`} />
                            <span>{msg.stepName}</span>
                          </div>
                          {msg.duration && <span className="font-mono text-[10.5px]">{msg.duration}</span>}
                        </div>
                      </div>
                    )}

                    {/* Message Body */}
                    <div className="text-[14px] leading-relaxed text-[#dcdcdc] ml-8 space-y-3">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                          code({ className, children, ...props }) {
                            const match = /language-(\w+)/.exec(className || '');
                            const codeStr = String(children).replace(/\n$/, '');
                            const cid = `code-${i}-${String(children).slice(0, 8)}`;
                            if (match) {
                              return (
                                <div className="my-3 rounded-xl overflow-hidden bg-[#141414] border border-[#2d2d2d] font-mono text-[12px]">
                                  <div className="flex items-center justify-between px-3 py-1.5 bg-[#202020] text-[#8a8a8a] text-[11px]">
                                    <span className="uppercase text-[10px] font-semibold text-white">{match[1]}</span>
                                    <button
                                      onClick={() => handleCopy(codeStr, cid)}
                                      className="flex items-center gap-1 hover:text-white transition"
                                    >
                                      {copiedCodeId === cid ? <Check className="w-3.5 h-3.5 text-[#3fb950]" /> : <Copy className="w-3.5 h-3.5" />}
                                      <span>{copiedCodeId === cid ? 'Copied' : 'Copy'}</span>
                                    </button>
                                  </div>
                                  <pre className="p-3.5 overflow-x-auto text-[#e0e0e0]"><code>{codeStr}</code></pre>
                                </div>
                              );
                            }
                            return (
                              <code className="px-1.5 py-0.5 rounded bg-[#2b2b2b] text-[#cc785c] font-mono text-[12px]" {...props}>
                                {children}
                              </code>
                            );
                          }
                        }}
                      >
                        {msg.text}
                      </ReactMarkdown>
                    </div>

                    {/* Cowork mode agent fleet card */}
                    {composerMode === 'cowork' && !isUser && (
                      <div className="ml-8 mt-2 p-3.5 rounded-xl bg-[#262626] border border-[#383838] space-y-2">
                        <div className="flex items-center justify-between text-xs">
                          <div className="flex items-center gap-2 text-white font-medium">
                            <Bot className="w-4 h-4 text-[#cc785c]" />
                            <span>Custos Multi-Agent Coworker Dispatch</span>
                          </div>
                          <span className="px-2 py-0.5 rounded-full text-[10px] bg-emerald-500/15 text-emerald-400 font-medium">
                            Active Leases
                          </span>
                        </div>
                        <div className="grid grid-cols-2 gap-2 text-[11.5px] text-[#9c9c9c] pt-1">
                          <div className="p-2 rounded-lg bg-[#1c1c1c] border border-[#2d2d2d]">
                            <div className="font-semibold text-white">S2 Planner</div>
                            <div className="text-[10px]">Decomposed 3 invariant tasks</div>
                          </div>
                          <div className="p-2 rounded-lg bg-[#1c1c1c] border border-[#2d2d2d]">
                            <div className="font-semibold text-white">Codex Synthesizer</div>
                            <div className="text-[10px]">Fenced in worktree wt-simd</div>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>

        </main>

        {/* ── Floating Composer Box (Screenshot 1 Bottom) ── */}
        <div 
          className={`absolute pointer-events-none flex justify-center z-20 transition-all duration-500 ease-out ${
            isWebTabExpanded ? 'bottom-6 right-6 w-[360px] px-0' : 'bottom-4 left-0 right-0 px-4 md:px-8'
          }`}
          style={{
            width: isWebTabExpanded ? '360px' : (isWebTabOpen ? `calc(${splitPercent}% - 32px)` : '100%'),
            left: isWebTabExpanded ? 'calc(100% - 360px - 1.5rem)' : '0px'
          }}
        >
          <div className="w-full max-w-2xl bg-[#1c2128]/95 backdrop-blur-xl border border-[#262c36] rounded-2xl shadow-2xl p-3 pointer-events-auto transition focus-within:border-[#384252]">
            <textarea
              ref={textareaRef}
              rows={isWebTabExpanded ? 1 : 2}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={
                composerMode === 'cowork'
                  ? 'Delegate autonomous multi-agent task (Custos SADE)...'
                  : 'How can I help you today?'
              }
              className="w-full bg-transparent border-none outline-none text-[#ececec] placeholder-[#6e7681] text-[13px] resize-none leading-relaxed px-1"
            />

            <div className="flex items-center justify-between pt-2 border-t border-[#262c36]/60 mt-1 select-none">
              {/* Left: Plus + Pill [ Chat | Cowork ] */}
              <div className="flex items-center gap-2">
                <button
                  onClick={() => { if (onShowToast) onShowToast('File upload attached'); }}
                  className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition"
                  title="Attach file or context"
                >
                  <Plus className="w-4 h-4" />
                </button>

                {/* THE PILL [ Chat | Cowork ] */}
                <div className="flex items-center p-0.5 rounded-full bg-[#161b22] border border-[#262c36] text-[12px]">
                  <button
                    onClick={() => {
                      setComposerMode('chat');
                      if (onShowToast) onShowToast('Switched to Chat mode');
                    }}
                    className={`px-3 py-1 rounded-full font-medium transition ${
                      composerMode === 'chat'
                        ? 'bg-[#22272e] text-white shadow-sm'
                        : 'text-[#8b949e] hover:text-[#e0e0e0]'
                    }`}
                  >
                    Chat
                  </button>
                  <button
                    onClick={() => {
                      setComposerMode('cowork');
                      if (onShowToast) onShowToast('Switched to Cowork mode — autonomous SADE agents active');
                    }}
                    className={`px-3 py-1 rounded-full font-medium transition flex items-center gap-1.5 ${
                      composerMode === 'cowork'
                        ? 'bg-[#2d333b] text-white shadow-sm border border-[#384252]'
                        : 'text-[#8b949e] hover:text-[#e0e0e0]'
                    }`}
                  >
                    <Bot className="w-3.5 h-3.5" />
                    <span>Cowork</span>
                  </button>
                </div>
              </div>

              {/* Right: Model + Mic + Send */}
              <div className="flex items-center gap-2">
                <div className="relative">
                  <button
                    onClick={() => setIsModelDropdownOpen(!isModelDropdownOpen)}
                    className="flex items-center gap-1 text-[12px] font-medium text-[#8b949e] hover:text-white px-2 py-1 rounded-lg hover:bg-[#22272e] transition"
                  >
                    <span>{selectedModel}</span>
                    <ChevronDown className="w-3 h-3 text-[#6e7681]" />
                  </button>

                  {isModelDropdownOpen && (
                    <div className="absolute right-0 bottom-8 w-52 rounded-xl bg-[#1c2128] border border-[#262c36] shadow-2xl p-1.5 text-xs z-50">
                      {['Sonnet 5.5 Medium', 'Claude 3.7 Sonnet', 'Custos SADE Sovereign', 'GPT-6 Luna Light', 'DeepSeek V3'].map((m) => (
                        <div
                          key={m}
                          onClick={() => {
                            setSelectedModel(m);
                            setIsModelDropdownOpen(false);
                            if (onShowToast) onShowToast(`Selected model: ${m}`);
                          }}
                          className={`px-2.5 py-1.5 rounded-lg cursor-pointer transition ${
                            selectedModel === m
                              ? 'bg-[#2d333b] text-white font-medium'
                              : 'text-[#8b949e] hover:text-white hover:bg-[#22272e]'
                          }`}
                        >
                          {m}
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <button
                  onClick={() => { if (onShowToast) onShowToast('Voice recording active'); }}
                  className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition"
                  title="Voice input"
                >
                  <Mic className="w-4 h-4" />
                </button>

                {inputText.trim() ? (
                  <button
                    onClick={handleSend}
                    className="p-1.5 rounded-lg bg-neutral-200 text-neutral-900 hover:bg-white transition shadow-sm"
                    title="Send message (Enter)"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                ) : (
                  <button
                    onClick={() => { if (onShowToast) onShowToast('Voice synthesis active'); }}
                    className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition"
                    title="Voice synthesis"
                  >
                    <AudioWaveform className="w-4 h-4" />
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>

        {isWebTabOpen && !isWebTabExpanded && (
          <div
            className="w-1 bg-[#262c36]/60 hover:bg-[#384252] cursor-col-resize shrink-0 z-30 transition-colors"
            onMouseDown={() => { isDragging.current = true; }}
            title="Drag to resize"
          />
        )}
        
        <div 
            className="h-full shrink-0 border-l border-[#262c36] bg-[#0d1117] relative z-10"
            style={{ display: isWebTabOpen ? 'block' : 'none', width: isWebTabExpanded ? '100%' : `${100 - splitPercent}%`, position: isWebTabExpanded ? 'absolute' : 'relative', right: 0 }}
          >
            <OrcaTabbedContainer
              resourceTabs={resourceTabs}
              isPaneOpen={Boolean(isWebTabOpen)}
              session={sessions.find(s => s.id === activeSessionId)}
              onShowToast={onShowToast}
              isExpanded={isWebTabExpanded}
              onToggleExpand={onToggleExpandWebTab}
              mode="chat"
              splitPercent={splitPercent}
              onToggleWebTab={onToggleWebTab}
            />
          </div>
      </div>

      {/* Delegate Task Modal (Vinh's SADE Autonomous Delegation) */}
      <DelegateTaskModal
        isOpen={isDelegateModalOpen}
        onClose={() => setIsDelegateModalOpen(false)}
        sessionId={activeSessionId}
        activeSessionName={activeSession?.title}
        onSubmit={handleDelegateSubmit}
      />
    </div>
  );
};
