import React, { useState, useRef, useEffect } from 'react';
import {
  Send,
  Terminal,
  FolderGit2,
  GitBranch,
  Orbit,
  Settings2,
  Plus,
  Mic,
  AudioWaveform
} from 'lucide-react';
import { Session } from '@/types';
import { ResourceTabbedPane, type ResourceTabsState } from './ResourceTabbedPane';
import { WorkspaceSidebar } from '@/components/shell/WorkspaceSidebar';
import { WorktreeManagerModal, type ManagedWorktree } from '@/components/modals';
import { OpenAIIcon } from '@/components/common/AgentIcons';
import { useAppContext } from '@/context/AppContext';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface CodexOrcaViewProps {
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
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
  onShowToast?: (msg: string) => void;
  onOpenSettings?: () => void;
  onSwitchMode?: (mode: 'chat' | 'code' | 'research') => void;
  isSidebarCollapsed?: boolean;
  isWebTabOpen?: boolean;
  isWebTabExpanded?: boolean;
  onToggleExpandWebTab?: () => void;
  onToggleWebTab?: () => void;
}

export const CodexOrcaView: React.FC<CodexOrcaViewProps> = ({
  resourceTabs,
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
  onOpenSettings,
  onSwitchMode,
  isSidebarCollapsed,
  isWebTabOpen,
  isWebTabExpanded,
  onToggleExpandWebTab,
  onToggleWebTab
}) => {
  const { openSettings } = useAppContext();
  const [inputText, setInputText] = useState('');
  const [splitPercent, setSplitPercent] = useState(45);
  const [isWorktreeModalOpen, setIsWorktreeModalOpen] = useState(false);
  const [activeBranch, setActiveBranch] = useState('wt-simd');
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  const activeSession = sessions.find((s) => s.id === activeSessionId) ?? sessions[0] ?? null;
  const hasMessages = (activeSession?.messages?.length ?? 0) > 0;

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging.current || !containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newPct = ((e.clientX - rect.left) / rect.width) * 100;
      setSplitPercent(Math.min(75, Math.max(25, newPct)));
    };
    const handleMouseUp = () => { isDragging.current = false; };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

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


  return (
    <div className="flex h-full w-full bg-[#0d1117] text-[#c9d1d9] font-sans overflow-hidden select-none">
      <WorkspaceSidebar
        currentProject={currentProject}
        projectNames={projectNames}
        onSelectProject={onSelectProject}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={onSelectSession}
        newTaskLabel="New Engineering Task"
        newTaskIcon={Plus}
        newTaskIconColor="text-[#58a6ff]"
        onNewSession={onNewSession}
        tools={[
          {
            icon: FolderGit2,
            label: 'Workspace',
            onClick: () => onShowToast?.('Workspace Explorer')
          },
          {
            icon: GitBranch,
            label: 'Active Branch',
            onClick: () => setIsWorktreeModalOpen(true),
            rightElement: <span className="text-[10px] bg-[#21262d] text-[#58a6ff] px-1.5 py-0.5 rounded font-sans">{activeBranch}</span>
          },
          {
            icon: Orbit,
            label: 'Automations',
            onClick: () => onShowToast?.('SADE Orbits & Automations')
          }
        ]}
        sessionsTitle="Engineering Sessions"
        emptySessionsMessage="No sessions yet"
        projectActiveColor="bg-[#58a6ff]"
        avatarColor="bg-[#58a6ff]"
        avatarText="CV"
        avatarTextColor="text-[#090d13]"
        onShowToast={onShowToast}
        onOpenSettings={onOpenSettings}
        onSwitchMode={onSwitchMode}
        isSidebarCollapsed={isSidebarCollapsed}
      />

      {/* ─────────────────────────────────────────────────────────────
          COLUMNS 2 & 3 CONTAINER (Middle Chat + Right Orca ADE Tabs)
      ───────────────────────────────────────────────────────────── */}
      <div ref={containerRef} className="flex-1 flex overflow-hidden min-w-0 relative">
        {/* ─────────────────────────────────────────────────────────────
            COLUMN 2: MIDDLE CHAT / DIALOGUE CANVAS
        ───────────────────────────────────────────────────────────── */}
        <div
          className="flex flex-col h-full overflow-hidden bg-[#0d1117] relative select-text"
          style={{ width: isWebTabOpen ? `${splitPercent}%` : '100%' }}
        >
          {/* Dialogue Messages Canvas */}
          <div className="flex-1 overflow-y-auto px-6 pb-44 pt-6 space-y-6">
            {!hasMessages ? (
              <div className="h-full flex flex-col items-center justify-center text-center my-auto pb-12">
                <div className="w-16 h-16 rounded-2xl bg-[#161b22] border border-[#30363d] flex items-center justify-center text-[#8b949e] mb-4 shadow-sm">
                  <span className="font-sans text-2xl text-[#8b949e] select-none">&#123; _ &#125;</span>
                </div>
                <h1 className="text-xl md:text-2xl font-semibold text-white tracking-tight max-w-md">
                  What should we build in{' '}
                  <span className="text-white underline decoration-[#30363d]">{currentProject}</span>?
                </h1>
                <p className="text-[12px] text-[#8b949e] mt-2 max-w-sm">
                  Connected to Custos sovereign runtime. Tools and worktrees active on the right.
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
                          <div className="w-5 h-5 rounded-md bg-[#58a6ff] text-white font-bold text-[9px] flex items-center justify-center shrink-0">
                            CV
                          </div>
                        ) : (
                          <div className="w-5 h-5 rounded-md bg-[#21262d] text-white flex items-center justify-center shrink-0 p-0.5 border border-[#30363d]">
                            <OpenAIIcon size={13} />
                          </div>
                        )}
                        <span className="font-semibold text-white">{isUser ? 'Chí Vĩ' : msg.author || 'Codex'}</span>
                        {msg.badge && (
                          <span className="px-1.5 py-0.2 rounded text-[10px] bg-[#161b22] text-[#8b949e] border border-[#30363d]">
                            {msg.badge}
                          </span>
                        )}
                      </div>

                      <div className="text-[13px] leading-relaxed text-[#c9d1d9] space-y-3">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ className, children, ...props }) {
                              const match = /language-(\w+)/.exec(className || '');
                              if (match) {
                                return (
                                  <div className="my-3 rounded-xl overflow-hidden bg-[#090d13] border border-[#30363d] font-mono text-[12px]">
                                    <div className="px-3 py-1.5 bg-[#161b22] text-[10px] text-[#8b949e] flex items-center gap-2">
                                      <Terminal className="w-3 h-3" />
                                      {match[1]}
                                    </div>
                                    <pre className="p-3.5 overflow-x-auto text-[#e6edf3]"><code>{String(children).replace(/\n$/, '')}</code></pre>
                                  </div>
                                );
                              }
                              return (
                                <code className="px-1.5 py-0.5 rounded bg-[#161b22] text-[#58a6ff] font-mono text-[12px]" {...props}>
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

        </div>

        {/* Chat Composer Input - Moved out to float over both columns */}
        <div className={`absolute z-10 transition-all duration-100 ease-out flex justify-center pointer-events-none ${
          isWebTabExpanded ? 'bottom-6 right-6 w-[360px] px-0' : 'bottom-4 left-0 right-0 px-4'
        }`}
        style={{ width: isWebTabExpanded ? '360px' : (isWebTabOpen ? `${splitPercent}%` : '100%'), left: isWebTabExpanded ? 'calc(100% - 360px - 1.5rem)' : '0px' }}
        >
          <div className="w-full max-w-3xl bg-[#1c2128]/95 backdrop-blur-xl border border-[#262c36] rounded-2xl shadow-2xl p-2.5 pointer-events-auto transition-all group">
            <textarea
              rows={isWebTabExpanded ? 1 : 2}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Message Codex to write code or run terminal tasks..."
              className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 focus:border-transparent text-[#ececec] placeholder-[#6e7681] text-[13px] resize-none leading-relaxed px-3 py-1.5"
            />
            <div className="flex flex-wrap items-center justify-between gap-2 pt-1.5 px-1 border-t border-[#262c36]/60">
              <div className="flex items-center gap-2">
                <button
                  onClick={() => onShowToast?.('Attach file or symbol')}
                  className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition-colors"
                  title="Attach context"
                >
                  <Plus className="w-4 h-4" />
                </button>
                <span className="text-[10px] font-sans font-medium px-2 py-0.5 rounded-md bg-white/5 text-neutral-300 border border-white/10">
                  SADE Copilot
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => onShowToast?.('Voice input active')}
                  className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition-colors"
                  title="Voice input"
                >
                  <Mic className="w-4 h-4" />
                </button>
                {inputText.trim() ? (
                  <button
                    onClick={handleSend}
                    disabled={!inputText.trim()}
                    className="p-1.5 rounded-lg bg-neutral-200 text-neutral-900 hover:bg-white transition shadow-sm"
                    title="Send message"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                ) : (
                  <button
                    onClick={() => onShowToast?.('Voice synthesis')}
                    className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#22272e] transition-colors"
                    title="Voice synthesis"
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
            COLUMN 3: RIGHT ORCA TABS (Engineering ADE)
        ───────────────────────────────────────────────────────────── */}
        
        <div 
            className="h-full shrink-0 border-l border-[#262c36] bg-[#0d1117] relative z-10"
            style={{ display: isWebTabOpen ? 'block' : 'none', width: isWebTabExpanded ? '100%' : `${100 - splitPercent}%`, position: isWebTabExpanded ? 'absolute' : 'relative', right: 0 }}
          >
            <ResourceTabbedPane
              resourceTabs={resourceTabs}
              isPaneOpen={Boolean(isWebTabOpen)}
              session={activeSession}
              onAcceptAndRun={onAcceptAndRun}
              onRejectDiff={onRejectDiff}
              onCopyDiff={onCopyDiff}
              onShowToast={onShowToast}
              isExpanded={isWebTabExpanded}
              onToggleExpand={onToggleExpandWebTab}
              mode="code"
              splitPercent={splitPercent}
              onToggleWebTab={onToggleWebTab}
            />
          </div>
      </div>

      {/* Worktree Manager Modal (Vinh's 5-Stage Lifecycle) */}
      <WorktreeManagerModal
        isOpen={isWorktreeModalOpen}
        onClose={() => setIsWorktreeModalOpen(false)}
        activeWorktreeId={activeBranch}
        onSelectWorktree={(wt: ManagedWorktree) => {
          setActiveBranch(wt.id);
          if (onShowToast) onShowToast(`Switched active worktree to: ${wt.branch}`);
        }}
      />
    </div>
  );
};

export const CodeView = CodexOrcaView;
export type CodeViewProps = CodexOrcaViewProps;
