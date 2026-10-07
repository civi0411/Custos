/**
 * ResearchView — Custos Research Workbench
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
  Sparkles,
  Activity,
  ShieldCheck,
  FileText,
  Settings2,
  Plus,
  BookOpen,
  FlaskConical,
  Code,
  TerminalSquare
} from 'lucide-react';
import { Session } from '@/types';
import { OrcaTabbedContainer, type OrcaTabId, type ResourceTabsState } from './OrcaTabbedContainer';
import { WorkspaceSidebar } from '@/components/shell/WorkspaceSidebar';
import { ClaudeIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface ResearchViewProps {
  resourceTabs: ResourceTabsState;
  onOpenResourcePane: () => void;
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

export const ResearchView: React.FC<ResearchViewProps> = ({
  resourceTabs,
  onOpenResourcePane,
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
  onSendMessage,
  onClearHistory: _onClearHistory,
  onShowToast,
  onOpenSettings: _onOpenSettings,
  onSwitchMode,
  isSidebarCollapsed,
  isWebTabOpen,
  isWebTabExpanded,
  onToggleExpandWebTab,
  onToggleWebTab
}) => {
  const { openSettings } = useAppContext();
  const [inputText, setInputText] = useState('');

  // Resizable split between chat (col 2) and research panel (col 3)
  const [splitPercent, setSplitPercent] = useState(42);
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  const activeSession = sessions.find((s) => s.id === activeSessionId) ?? sessions[0] ?? null;
  const openResearchResource = (id: OrcaTabId, title: string) => {
    resourceTabs.setTabs((previous) => previous.some((tab) => tab.id === id)
      ? previous
      : [...previous, { id, title, url: `custos://research/${id}` }]);
    resourceTabs.setActiveTabId(id);
    onOpenResourcePane();
  };
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
    { icon: BookOpen, label: 'Read a source', prompt: 'Help me analyze a paper or URL that I provide. Separate what the source says from your inference.', color: 'text-blue-400', bg: 'bg-blue-400/10' },
    { icon: ShieldCheck, label: 'Check a claim', prompt: 'Help me evaluate a claim against sources I provide. Mark unsupported or unavailable evidence as unknown.', color: 'text-purple-400', bg: 'bg-purple-400/10' },
    { icon: Activity, label: 'Plan an experiment', prompt: 'Design a reproducible experiment plan with dataset, baseline, environment, metrics, cost, and stop conditions.', color: 'text-emerald-400', bg: 'bg-emerald-400/10' },
    { icon: FileText, label: 'Synthesize findings', prompt: 'Help me synthesize selected findings into a brief. Keep the source and uncertainty of each conclusion visible.', color: 'text-amber-400', bg: 'bg-amber-400/10' },
    { icon: FlaskConical, label: 'Extract claims', prompt: 'From a source I provide, identify atomic claims and the passages needed to evaluate them.', color: 'text-indigo-400', bg: 'bg-indigo-400/10' },
    { icon: Sparkles, label: 'Compare approaches', prompt: 'Help me compare two approaches using the same evaluation criteria and note what evidence is missing.', color: 'text-rose-400', bg: 'bg-rose-400/10' }
  ];

  return (
    <div className="flex h-full w-full text-[#c9d1d9] font-sans overflow-hidden select-none bg-[#04080F]">
      <WorkspaceSidebar
        currentProject={currentProject}
        projectNames={projectNames}
        onSelectProject={onSelectProject}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        newTaskLabel="New Research Task"
        newTaskIcon={Plus}
        newTaskIconColor="text-indigo-400"
        onNewSession={onNewSession}
        tools={[
          {
            icon: BookOpen,
            label: 'Corpus & Sources',
            onClick: () => openResearchResource('literature', 'Literature')
          },
          {
            icon: ShieldCheck,
            label: 'Claims Matrix',
            onClick: () => openResearchResource('claims', 'Claims')
          },
          {
            icon: TerminalSquare,
            label: 'Notebook & Compute',
            onClick: () => openResearchResource('experiments', 'Experiments')
          },
          {
            icon: FlaskConical,
            label: 'Runs Ledger',
            onClick: () => openResearchResource('synthesis', 'Runs Ledger')
          },
          {
            icon: Code,
            label: 'Deep Inspector',
            onClick: () => openResearchResource('artifacts', 'Artifacts')
          },
          {
            icon: Settings2,
            label: 'Preferences & Settings',
            onClick: () => openSettings('general')
          }
        ]}
        sessionsTitle="Research Sessions"
        emptySessionsMessage="No sessions yet"
        projectActiveColor="bg-gradient-to-r from-indigo-500 to-purple-500"
        avatarColor="bg-indigo-500"
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
          className="flex flex-col h-full overflow-hidden relative select-text bg-[#04080F]"
          style={{ width: isWebTabExpanded ? '0%' : (isWebTabOpen ? `${splitPercent}%` : '100%'), display: isWebTabExpanded ? 'none' : 'flex' }}
        >
          {/* Subtle chat background glow */}
          <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-indigo-500/5 blur-[120px] rounded-full pointer-events-none" />

          {/* Messages Canvas */}
          <div className="flex-1 overflow-y-auto px-6 pb-44 pt-8 space-y-8 relative z-0 scroll-smooth">
            {!hasMessages ? (
              /* Empty State */
              <div className="h-full flex flex-col items-center justify-center text-center pb-12">
                <div className="w-20 h-20 rounded-3xl bg-gradient-to-br from-[#111722] to-[#080d16] border border-[#1e2430] flex items-center justify-center mb-6 shadow-[0_0_40px_rgba(99,102,241,0.1)] relative">
                  <div className="absolute inset-0 bg-indigo-500/10 blur-xl rounded-3xl" />
                  <FlaskConical className="w-10 h-10 text-indigo-400 relative z-10" />
                </div>
                <h1 className="text-2xl font-black text-white mb-3 tracking-tight">
                  Research Workbench
                </h1>
                <p className="text-[13px] text-[#8b949e] max-w-md mb-10 leading-relaxed">
                  Discuss sources, design verifiable experiments, and extract grounded claims. Open Literature, Claims or Experiments for structured work.
                </p>
                
                {/* Quick prompt grid */}
                <div className="grid grid-cols-2 gap-3 w-full max-w-lg">
                  {RESEARCH_STARTERS.map((s) => {
                    const Icon = s.icon;
                    return (
                      <button
                        key={s.label}
                        onClick={() => setInputText(s.prompt)}
                        className="group text-left p-4 rounded-2xl bg-[#080d16]/80 border border-[#1e2430] hover:border-indigo-500/40 hover:bg-[#0d131f] transition-all hover:shadow-lg hover:-translate-y-0.5"
                      >
                        <div className="flex items-center gap-3 mb-2">
                          <div className={`p-1.5 rounded-lg ${s.bg}`}>
                            <Icon className={`w-4 h-4 ${s.color}`} />
                          </div>
                          <span className="font-bold text-[13px] text-white tracking-wide">{s.label}</span>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : (
              /* Message Thread */
              <div className="space-y-8 max-w-3xl mx-auto w-full">
                {activeSession!.messages.map((msg, i) => {
                  const isUser = msg.role === 'user';
                  return (
                    <div key={msg.id || i} className="space-y-3 group">
                      <div className="flex items-center gap-3 text-sm">
                        <div
                          className="w-8 h-8 rounded-full flex items-center justify-center text-white font-bold text-[11px] shrink-0 shadow-sm"
                          style={{ background: isUser ? 'linear-gradient(135deg, #6366f1, #a855f7)' : '#111722', border: isUser ? 'none' : '1px solid #2d3342' }}
                        >
                          {isUser ? 'CV' : <ClaudeIcon size={16} />}
                        </div>
                        <span className="font-bold text-[#e0e6ed]">{isUser ? 'You' : msg.author || 'Assistant'}</span>
                        {msg.badge && (
                          <span className="px-2 py-0.5 rounded-md text-[10px] bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 font-mono font-bold uppercase tracking-wider">
                            {msg.badge}
                          </span>
                        )}
                      </div>
                      <div className={`text-[14.5px] leading-[1.8] ml-11 ${isUser ? 'text-[#c9d1d9]' : 'text-[#e0e6ed]'}`}>
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ className, children, ...props }) {
                              const match = /language-(\w+)/.exec(className || '');
                              if (match) {
                                return (
                                  <div className="my-4 rounded-xl overflow-hidden bg-[#04080F] border border-[#1e2430] font-mono text-[13px] shadow-lg">
                                    <div className="px-4 py-2 bg-[#080d16] border-b border-[#1e2430] text-[11px] uppercase font-bold tracking-widest text-[#8b949e] flex justify-between items-center">
                                      <span>{match[1]}</span>
                                    </div>
                                    <pre className="p-4 overflow-x-auto text-[#e6edf3]"><code>{String(children).replace(/\n$/, '')}</code></pre>
                                  </div>
                                );
                              }
                              return <code className="px-1.5 py-0.5 rounded-md bg-[#1e2430] text-indigo-300 font-mono text-[13px] border border-[#2d3342]" {...props}>{children}</code>;
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

        </div>

        {/* Composer - Floating with controlled z-index */}
        <div className={`absolute z-20 pointer-events-none flex justify-center transition-all duration-500 ease-out ${
          isWebTabExpanded ? 'bottom-6 right-6 w-[360px]' : 'bottom-6 left-6 w-[calc(100%-48px)] max-w-[calc(100vw-300px)]'
        }`}
        style={{ width: isWebTabExpanded ? '360px' : (isWebTabOpen ? `calc(${splitPercent}% - 48px)` : 'calc(100% - 48px)'), left: isWebTabExpanded ? 'calc(100% - 360px - 1.5rem)' : '1.5rem' }}
        >
          <div
            className="w-full max-w-4xl bg-[#1c2128]/95 backdrop-blur-xl border border-[#262c36] rounded-2xl shadow-2xl p-2.5 pointer-events-auto transition-all focus-within:border-[#384252] group"
          >
            <textarea
              rows={isWebTabExpanded ? 1 : 2}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask about a source, claim, dataset, or experiment..."
              className="w-full bg-transparent border-none outline-none text-[#ececec] placeholder-[#6e7681] text-[13px] resize-none leading-relaxed px-3 py-1.5"
            />
            <div className="flex items-center justify-between pt-1.5 px-1 border-t border-[#262c36]/60">
              <div className="flex items-center gap-2">
                <button
                  onClick={() => onShowToast?.('Attach paper / file / URL')}
                  className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition-colors"
                >
                  <Plus className="w-4 h-4" />
                </button>
                <span className="text-[10px] font-mono font-medium px-2 py-0.5 rounded-md bg-white/5 text-neutral-300 border border-white/10">
                  Research Tool
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => onShowToast?.('Voice input active')}
                  className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition-colors"
                >
                  <Mic className="w-4 h-4" />
                </button>
                {inputText.trim() ? (
                  <button
                    onClick={handleSend}
                    className="p-1.5 rounded-lg bg-neutral-200 text-neutral-900 hover:bg-white transition shadow-sm"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                ) : (
                  <button
                    onClick={() => onShowToast?.('Voice synthesis')}
                    className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition-colors"
                  >
                    <AudioWaveform className="w-4 h-4" />
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* ── DRAG HANDLE ── */}
        {isWebTabOpen && !isWebTabExpanded && (
          <div
            className="w-1 bg-[#262c36]/60 hover:bg-[#384252] cursor-col-resize shrink-0 z-30 transition-colors"
            onMouseDown={() => { isDragging.current = true; }}
            title="Drag to resize"
          />
        )}

        {/* ─────────────────────────────────────────────────────────────
            COLUMN 3: WEB TAB (OrcaTabbedContainer)
        ───────────────────────────────────────────────────────────── */}
        <div 
            className="h-full shrink-0 border-l border-[#262c36] bg-[#04080F] relative z-10"
            style={{ display: isWebTabOpen ? 'block' : 'none', width: isWebTabExpanded ? '100%' : `${100 - splitPercent}%`, position: isWebTabExpanded ? 'absolute' : 'relative', right: 0 }}
          >
            <OrcaTabbedContainer
              resourceTabs={resourceTabs}
              isPaneOpen={Boolean(isWebTabOpen)}
              session={sessions.find(s => s.id === activeSessionId)}
              onShowToast={onShowToast}
              isExpanded={isWebTabExpanded}
              onToggleExpand={onToggleExpandWebTab}
              mode="research"
              splitPercent={splitPercent}
              onToggleWebTab={onToggleWebTab}
              onAskAgent={(draft) => {
                setInputText((prev) => (prev ? `${prev}\n\n${draft}` : draft));
                if (onShowToast) onShowToast('Drafted into Research Composer');
              }}
              onHandoffToCoding={(claims) => {
                if (onSwitchMode) onSwitchMode('code');
                if (onShowToast) onShowToast(`Handed off ${claims.length} verified claim(s) to Coding Workbench!`);
              }}
            />
          </div>
      </div>
    </div>
  );
};
