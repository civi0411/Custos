/**
 * ResearchView — Claude Science Workbench
 *
 * 3-Column layout mirroring CodexOrcaView:
 *   Col 1: Unified Chat-style Sidebar (w-64)
 *   Col 2: Research AI Chat (literature Q&A with assistant)
 *   Col 3: ResearchWorkspace tabbed panel (Literature | Claims | Experiments | Synthesis)
 */
import React, { useState, useRef, useEffect } from 'react';
import {
  Send,
  Mic,
  AudioWaveform,
  RotateCcw,
  Sparkles,
  Activity,
  ShieldCheck,
  FileText,
  Settings2,
  Plus,
  BookOpen,
  FlaskConical,
  MessageSquare,
  Code,
  Download
} from 'lucide-react';
import { Session } from '@/types';import { OrcaTabbedContainer } from './OrcaTabbedContainer';
import { WorkspaceSidebar } from '@/components/shell/WorkspaceSidebar';
import { ClaudeIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface ResearchViewProps {
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
  onWebTabCountChange?: (count: number) => void;
}

export const ResearchView: React.FC<ResearchViewProps> = ({
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
  onOpenSettings: _onOpenSettings,
  onSwitchMode,
  isSidebarCollapsed,
  isWebTabOpen,
  isWebTabExpanded,
  onToggleExpandWebTab,
  onWebTabCountChange
}) => {
  const { openSettings } = useAppContext();
  const [inputText, setInputText] = useState('');

  // Resizable split between chat (col 2) and research panel (col 3)
  const [splitPercent, setSplitPercent] = useState(42);
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  const activeSession = sessions.find((s) => s.id === activeSessionId) ?? sessions[0] ?? null;
  const hasMessages = (activeSession?.messages?.length ?? 0) > 0;

  const messagesEndRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [activeSession?.messages]);

  const handleSend = () => {
    if (!inputText.trim()) return;
    onSendMessage(inputText.trim());
    setInputText('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const exportConversation = () => {
    if (!activeSession) return;
    const md = activeSession.messages
      .map(m => `## ${m.role === 'user' ? 'User (Chí Vĩ)' : (m.author || 'Claude Science')}\n\n${m.text}`)
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

  // Drag handler for split resizer
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newPct = ((e.clientX - rect.left) / rect.width) * 100;
      setSplitPercent(Math.min(70, Math.max(30, newPct)));
    };
    const handleMouseUp = () => { isDragging.current = false; };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

  const RESEARCH_STARTERS = [
    { icon: BookOpen, label: 'Summarize literature', prompt: 'Summarize the key findings from the SADE research corpus on non-repudiation permit gating.' },
    { icon: ShieldCheck, label: 'Verify a claim', prompt: 'Verify CLM-01: "Atomic ticket acquisition prevents phantom double-dispatches in multi-agent turns." against evidence in arXiv:2403.1189.' },
    { icon: Activity, label: 'Run experiment', prompt: 'Run PermitGate Fencing Stress Test with 1000 concurrent agents. Log reproducibility score and duration.' },
    { icon: FileText, label: 'Synthesize notes', prompt: 'Synthesize verified claims CLM-01 and CLM-02 into a structured invariant specification for the Engineering workbench.' },
    { icon: FlaskConical, label: 'Extract claims', prompt: 'Extract empirical claims from the Custos Technical Monograph (custos://doi/2025.01.sade).' },
    { icon: Sparkles, label: 'Literature search', prompt: 'Search for recent arXiv papers on multi-agent supervisor architectures with formal safety invariants, published 2024-2025.' }
  ];

  return (
    <div className="flex h-full w-full text-[#c9d1d9] font-sans overflow-hidden select-none" style={{ background: '#0d1117' }}>
      <WorkspaceSidebar
        currentProject={currentProject}
        projectNames={projectNames}
        onSelectProject={onSelectProject}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        newTaskLabel="New Research Task"
        newTaskIcon={Plus}
        newTaskIconColor="text-[#a371f7]"
        onNewSession={onNewSession}
        tools={[
          {
            icon: BookOpen,
            label: 'Corpus & Sources',
            onClick: () => onShowToast?.('Search Literature Corpus')
          },
          {
            icon: Activity,
            label: 'Experiments',
            onClick: () => onShowToast?.('Experiment Results')
          },
          {
            icon: ShieldCheck,
            label: 'Claims Matrix',
            onClick: () => onShowToast?.('Verified Claim Matrix')
          },
          {
            icon: Settings2,
            label: 'Preferences & Settings',
            onClick: () => openSettings('general')
          }
        ]}
        sessionsTitle="Research Sessions"
        emptySessionsMessage="No sessions yet"
        projectActiveColor="bg-[#a371f7]"
        avatarColor="bg-[#a371f7]"
        avatarText="CV"
        onShowToast={onShowToast}
        onOpenAppGrid={() => onShowToast?.('App Switcher')}
        isSidebarCollapsed={isSidebarCollapsed}
      />

      {/* ─────────────────────────────────────────────────────────────
          COLUMN 2 + 3 CONTAINER (resizable)
      ───────────────────────────────────────────────────────────── */}
      <div ref={containerRef} className="flex-1 flex overflow-hidden min-w-0 relative">
        {/* ─────────────────────────────────────────────────────────────
            COLUMN 2: RESEARCH CHAT (AI Science Assistant)
        ───────────────────────────────────────────────────────────── */}
        <div
          className="flex flex-col h-full overflow-hidden relative select-text"
          style={{ width: isWebTabExpanded ? '0%' : (isWebTabOpen ? `${splitPercent}%` : '100%'), display: isWebTabExpanded ? 'none' : 'flex', background: '#0d1117' }}
        >
          {/* Chat Header */}
          <div className="h-10 px-4 border-b border-[#21262d] flex items-center justify-between shrink-0 bg-[#090d13]">
            <div className="flex items-center gap-2">
              <FlaskConical className="w-3.5 h-3.5 text-[#a371f7]" />
              <span className="font-semibold text-white text-[13px]">Research Assistant</span>
            </div>
            
            <div className="flex items-center gap-2">
              {/* Context Transfer Buttons (Send to Chat & Send to Code) */}
              {onSwitchMode && (
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => {
                      onSwitchMode('chat');
                      if (onShowToast) onShowToast('Transferred research context to Claude Chat');
                    }}
                    className="flex items-center gap-1.5 text-[11px] px-2 py-1 rounded bg-[#21262d] text-[#8b949e] hover:text-white transition"
                    title="Take this research context and discuss in Chat"
                  >
                    <MessageSquare className="w-3 h-3 text-[#3fb950]" />
                    <span>Send to Chat</span>
                  </button>
                  <button
                    onClick={() => {
                      onSwitchMode('code');
                      if (onShowToast) onShowToast('Transferred knowledge context to Codex & Orca ADE');
                    }}
                    className="flex items-center gap-1.5 text-[11px] px-2 py-1 rounded bg-[#21262d] text-[#8b949e] hover:text-white transition"
                    title="Take this research context and write code"
                  >
                    <Code className="w-3 h-3 text-[#58a6ff]" />
                    <span>Send to Code</span>
                  </button>
                </div>
              )}
              {hasMessages && (
                <>
                  <button
                    onClick={exportConversation}
                    className="flex items-center gap-1 text-[11px] text-[#8b949e] hover:text-white transition"
                    title="Export conversation as Markdown"
                  >
                    <Download className="w-3 h-3 text-[#a371f7]" />
                    <span>Export .md</span>
                  </button>
                  <button
                    onClick={onClearHistory}
                    className="flex items-center gap-1 text-[11px] text-[#8b949e] hover:text-white transition"
                  >
                    <RotateCcw className="w-3 h-3" />
                    <span>Clear</span>
                  </button>
                </>
              )}
            </div>
          </div>

          {/* Messages Canvas */}
          <div className="flex-1 overflow-y-auto px-6 pb-44 pt-6 space-y-6">
            {!hasMessages ? (
              /* Empty State */
              <div className="h-full flex flex-col items-center justify-center text-center pb-12">
                <div className="w-14 h-14 rounded-2xl bg-[#161b22] border border-[#21262d] flex items-center justify-center mb-5 shadow">
                  <FlaskConical className="w-7 h-7 text-[#a371f7]" />
                </div>
                <h1 className="text-xl font-semibold text-white mb-2 tracking-tight">
                  Claude Science Lab
                </h1>
                <p className="text-[12px] text-[#8b949e] max-w-xs mb-6">
                  Research, verify claims, run experiments. The Evidence Lab is on the right.
                </p>
                {/* Quick prompt grid */}
                <div className="grid grid-cols-2 gap-2 w-full max-w-sm">
                  {RESEARCH_STARTERS.map((s) => {
                    const Icon = s.icon;
                    return (
                      <button
                        key={s.label}
                        onClick={() => setInputText(s.prompt)}
                        className="text-left p-3 rounded-xl bg-[#161b22] border border-[#21262d] hover:border-[#a371f7]/40 hover:bg-[#1c2128] transition"
                      >
                        <div className="flex items-center gap-1.5 mb-1">
                          <Icon className="w-3.5 h-3.5 text-[#a371f7]" />
                          <span className="font-semibold text-[11.5px] text-white">{s.label}</span>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : (
              /* Message Thread */
              <div className="space-y-6 max-w-2xl mx-auto w-full">
                {activeSession!.messages.map((msg, i) => {
                  const isUser = msg.role === 'user';
                  return (
                    <div key={msg.id || i} className="space-y-2">
                      <div className="flex items-center gap-2 text-xs">
                        <div
                          className="w-6 h-6 rounded-full flex items-center justify-center text-white font-bold text-[10px] shrink-0 p-1"
                          style={{ background: isUser ? '#a371f7' : '#161b22', border: '1px solid #30363d' }}
                        >
                          {isUser ? 'CV' : <ClaudeIcon size={13} />}
                        </div>
                        <span className="font-semibold text-white">{isUser ? 'Chí Vĩ' : msg.author || 'Claude Science'}</span>
                        {msg.badge && (
                          <span className="px-1.5 py-0.2 rounded text-[10px] bg-[#161b22] text-[#8b949e] border border-[#30363d]">
                            {msg.badge}
                          </span>
                        )}
                      </div>
                      <div className="text-[13px] leading-relaxed text-[#c9d1d9] ml-8">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ className, children, ...props }) {
                              const match = /language-(\w+)/.exec(className || '');
                              if (match) {
                                return (
                                  <div className="my-3 rounded-xl overflow-hidden bg-[#141414] border border-[#21262d] font-mono text-[12px]">
                                    <div className="px-3 py-1.5 bg-[#161b22] text-[10px] uppercase font-semibold text-[#a371f7]">{match[1]}</div>
                                    <pre className="p-3.5 overflow-x-auto text-[#e0e0e0]"><code>{String(children).replace(/\n$/, '')}</code></pre>
                                  </div>
                                );
                              }
                              return <code className="px-1.5 py-0.5 rounded bg-[#161b22] text-[#a371f7] font-mono text-[12px]" {...props}>{children}</code>;
                            }
                          }}
                        >
                          {msg.text}
                        </ReactMarkdown>
                      </div>
                    </div>
                  );
                })}
                <div ref={messagesEndRef} />
              </div>
            )}
          </div>

          {/* Composer */}
          <div className={`absolute pointer-events-none flex justify-center z-50 transition-all duration-300 ${isWebTabExpanded ? 'bottom-6 left-1/2 -translate-x-1/2 w-[600px]' : 'bottom-4 left-4 right-4'}`}>
            <div
              className="w-full bg-[#161b22] border border-[#30363d] rounded-2xl shadow-2xl p-3 pointer-events-auto transition focus-within:border-[#a371f7]/50"
            >
              <textarea
                rows={2}
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Ask Claude Science to search, verify, or synthesize research..."
                className="w-full bg-transparent border-none outline-none text-[#ececec] placeholder-[#6e7681] text-[13px] resize-none leading-relaxed px-1"
              />
              <div className="flex items-center justify-between pt-2 border-t border-[#21262d] mt-1">
                <div className="flex items-center gap-1.5">
                  <button
                    onClick={() => onShowToast?.('Attach paper / file / URL')}
                    className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#21262d] transition"
                  >
                    <Plus className="w-4 h-4" />
                  </button>
                  <span className="text-[11px] font-mono px-2 py-0.5 rounded bg-[#0d1117] text-[#a371f7] border border-[#30363d]">
                    Claude Science
                  </span>
                </div>
                <div className="flex items-center gap-1.5">
                  <button
                    onClick={() => onShowToast?.('Voice input active')}
                    className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#21262d] transition"
                  >
                    <Mic className="w-4 h-4" />
                  </button>
                  {inputText.trim() ? (
                    <button
                      onClick={handleSend}
                      className="p-1.5 rounded-lg bg-[#a371f7] text-white hover:bg-[#8957e5] transition shadow-sm"
                    >
                      <Send className="w-4 h-4" />
                    </button>
                  ) : (
                    <button
                      onClick={() => onShowToast?.('Voice synthesis')}
                      className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#21262d] transition"
                    >
                      <AudioWaveform className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* ── DRAG HANDLE ── */}
        {isWebTabOpen && !isWebTabExpanded && (
          <div
            className="w-1 bg-[#21262d] hover:bg-[#a371f7]/60 cursor-col-resize shrink-0 z-30 transition-colors"
            onMouseDown={() => { isDragging.current = true; }}
            title="Drag to resize"
          />
        )}

        {/* ─────────────────────────────────────────────────────────────
            COLUMN 3: WEB TAB (OrcaTabbedContainer)
        ───────────────────────────────────────────────────────────── */}
        <div 
            className="h-full shrink-0 border-l border-[#21262d] bg-[#0d1117] relative z-20"
            style={{ display: isWebTabOpen ? 'block' : 'none', width: isWebTabExpanded ? '100%' : `${100 - splitPercent}%`, position: isWebTabExpanded ? 'absolute' : 'relative', right: 0 }}
          >
            <OrcaTabbedContainer
              session={sessions.find(s => s.id === activeSessionId)}
              onShowToast={onShowToast}
              isExpanded={isWebTabExpanded}
              onToggleExpand={onToggleExpandWebTab}
              mode="research"
              onTabsCountChange={onWebTabCountChange}
            />
          </div>
      </div>
    </div>
  );
};
