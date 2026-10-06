import React, { useState, useRef, useEffect } from 'react';
import {
  RotateCcw,
  Download,
  ArrowUp,
  ChevronDown,
  ChevronRight,
  Copy,
  Check,
  CheckCircle2,
  Target,
  Paperclip,
  Sparkles,
  Terminal,
  FlaskConical,
  ShieldCheck
} from 'lucide-react';
import { Session } from '@/types';
import { motion, AnimatePresence } from 'framer-motion';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface ChatSectionProps {
  session: Session | null;
  onSendMessage: (text: string) => void;
  onClearHistory: () => void;
  onShowToast: (msg: string) => void;
}

const STARTER_PROMPTS = [
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

function CodeBlock({ lang, code, onCopy, copied }: {
  lang: string;
  code: string;
  onCopy: () => void;
  copied: boolean;
}) {
  return (
    <div 
      className="my-3 rounded-xl overflow-hidden font-mono text-[12px]"
      style={{
        background: 'var(--color-canvas-inset, #010409)',
        border: '1px solid var(--color-border-default, #30363d)'
      }}
    >
      <div 
        className="flex items-center justify-between px-3 py-1.5 text-[11px] select-none"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          borderBottom: '1px solid var(--color-border-muted, #21262d)',
          color: 'var(--color-fg-muted, #8b949e)'
        }}
      >
        <span className="font-semibold text-white uppercase text-[10px] tracking-wider">{lang || 'text'}</span>
        <button
          onClick={onCopy}
          className="flex items-center gap-1 text-[#8b949e] hover:text-white transition-colors"
        >
          {copied ? <Check className="w-3.5 h-3.5 text-[#3fb950]" /> : <Copy className="w-3.5 h-3.5" />}
          <span>{copied ? 'Copied' : 'Copy'}</span>
        </button>
      </div>
      <pre className="p-3.5 overflow-x-auto leading-relaxed text-[#e6edf3]">
        <code>{code}</code>
      </pre>
    </div>
  );
}

function ThinkingTrace({ stepName, duration, expanded, onToggle }: {
  stepName: string;
  duration?: string;
  expanded: boolean;
  onToggle: () => void;
}) {
  return (
    <div 
      className="my-2 rounded-lg overflow-hidden text-xs"
      style={{
        background: 'var(--color-surface-1, #161b22)',
        border: '1px solid var(--color-border-muted, #21262d)'
      }}
    >
      <button 
        className="w-full px-3 py-2 flex items-center justify-between transition hover:bg-surface-2 text-left"
        onClick={onToggle}
      >
        <div className="flex items-center gap-2">
          <Sparkles className="w-3.5 h-3.5 text-[#a78bfa]" />
          <span className="font-mono text-[11px] text-[#e6edf3] font-medium">{stepName}</span>
        </div>
        <div className="flex items-center gap-2 text-[#8b949e] font-mono text-[10px]">
          {duration && <span>{duration}</span>}
          {expanded ? <ChevronDown className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
        </div>
      </button>

      <AnimatePresence>
        {expanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="p-3 text-[11px] font-mono text-[#8b949e] space-y-1.5"
            style={{ borderTop: '1px solid var(--color-border-muted, #21262d)' }}
          >
            <div className="flex items-center gap-2 text-[#3fb950]">
              <CheckCircle2 className="w-3 h-3" />
              <span>Parsed AST symbols in crates/custos-runtime/src/workflow/dispatcher.rs</span>
            </div>
            <div className="flex items-center gap-2 text-[#3fb950]">
              <CheckCircle2 className="w-3 h-3" />
              <span>Checked invariant INV-01: Double-dispatch lease verification required</span>
            </div>
            <div className="flex items-center gap-2 text-[#3fb950]">
              <CheckCircle2 className="w-3 h-3" />
              <span>Generated non-repudiation proof closure: cas://bafy2bzace4v3k99apermitfenced01</span>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export const ChatSection: React.FC<ChatSectionProps> = ({
  session,
  onSendMessage,
  onClearHistory,
  onShowToast
}) => {
  const [inputText, setInputText] = useState('');
  const [isTaskMode, setIsTaskMode] = useState(false);
  const [expandedTraces, setExpanded] = useState<Record<number, boolean>>({ 1: true });
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [session?.messages]);

  useEffect(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = Math.min(ta.scrollHeight, 180) + 'px';
  }, [inputText]);

  const handleSend = () => {
    if (!inputText.trim()) return;
    onSendMessage(inputText.trim());
    setInputText('');
    if (textareaRef.current) textareaRef.current.style.height = 'auto';
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleCopyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopiedId(id);
    onShowToast('Code copied to clipboard');
    setTimeout(() => setCopiedId(null), 2000);
  };

  const exportConversation = () => {
    if (!session) return;
    const md = session.messages
      .map(m => `## ${m.author}\n\n${m.text}`)
      .join('\n\n---\n\n');
    const blob = new Blob([`# ${session.title}\n\n${md}\n`], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${session.title.replace(/[^a-z0-9-_]+/gi, '-').toLowerCase()}.md`;
    a.click();
    URL.revokeObjectURL(url);
    onShowToast('Exported conversation as Markdown');
  };

  const hasMessages = session && session.messages && session.messages.length > 0;

  return (
    <section 
      className="flex-1 flex flex-col h-full overflow-hidden select-text"
      style={{ background: 'var(--color-canvas, #0d1117)' }}
    >
      {/* ── 1. Top Subbar (Claude Desktop header) ── */}
      <div 
        className="h-10 px-4 flex items-center justify-between shrink-0 select-none"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          borderBottom: '1px solid var(--color-border-default, #30363d)',
        }}
      >
        <div className="flex items-center gap-2">
          {/* Claude Model Selector pill */}
          <div 
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium cursor-pointer"
            style={{
              background: 'var(--color-surface-2, #1c2128)',
              border: '1px solid var(--color-border-default, #30363d)',
              color: 'var(--color-editor-fg, #e6edf3)'
            }}
          >
            <Sparkles className="w-3.5 h-3.5 text-[#a78bfa]" />
            <span>Claude 3.7 Sonnet</span>
            <ChevronDown className="w-3 h-3 text-[#6e7681]" />
          </div>

          <span className="text-[#6e7681] text-xs">·</span>
          <span className="text-xs text-[#8b949e] truncate max-w-sm">
            {session?.title || 'New Conversation'}
          </span>
        </div>

        <div className="flex items-center gap-1">
          <button 
            onClick={onClearHistory} 
            className="p-1.5 rounded hover:bg-surface-2 text-[#8b949e] hover:text-white transition"
            title="Reset conversation history"
          >
            <RotateCcw className="w-3.5 h-3.5" />
          </button>
          <button 
            onClick={exportConversation} 
            className="p-1.5 rounded hover:bg-surface-2 text-[#8b949e] hover:text-white transition"
            title="Export as Markdown"
          >
            <Download className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* ── 2. Message Conversation Stream ── */}
      <div className="flex-1 overflow-y-auto px-4 py-6">
        <div className="max-w-3xl mx-auto w-full space-y-6">
          {!hasMessages ? (
            /* Claude Welcome Screen (Empty state) */
            <div className="py-12 flex flex-col items-center text-center space-y-6 select-none">
              <div 
                className="w-14 h-14 rounded-2xl flex items-center justify-center shadow-lg"
                style={{
                  background: 'var(--color-surface-2, #1c2128)',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                <Sparkles className="w-7 h-7 text-[#a78bfa]" />
              </div>

              <div className="space-y-1">
                <h2 className="text-xl font-bold text-white tracking-tight">How can Custos help you today?</h2>
                <p className="text-xs text-[#8b949e]">
                  Supervised agent environment for coding, literature research, and invariant safety verification.
                </p>
              </div>

              {/* Starter Prompt Cards */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3 w-full pt-4">
                {STARTER_PROMPTS.map((starter) => {
                  const Icon = starter.icon;
                  return (
                    <button
                      key={starter.title}
                      onClick={() => {
                        setInputText(starter.prompt);
                        textareaRef.current?.focus();
                      }}
                      className="p-3.5 rounded-xl text-left transition space-y-2 group"
                      style={{
                        background: 'var(--color-surface-1, #161b22)',
                        border: '1px solid var(--color-border-default, #30363d)',
                      }}
                    >
                      <div className="p-1.5 rounded w-fit" style={{ background: 'var(--color-surface-2, #1c2128)' }}>
                        <Icon className="w-4 h-4 text-[#a78bfa] group-hover:text-white transition" />
                      </div>
                      <div>
                        <h4 className="text-xs font-semibold text-white group-hover:text-[#a78bfa] transition">
                          {starter.title}
                        </h4>
                        <p className="text-[11px] text-[#8b949e] line-clamp-2 mt-0.5">
                          {starter.desc}
                        </p>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          ) : (
            /* Conversation Messages List */
            session.messages.map((msg, idx) => {
              const isUser = msg.role === 'user';
              return (
                <motion.div
                  key={idx}
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.16 }}
                  className="flex gap-4 min-w-0"
                >
                  {/* Avatar */}
                  <div className="shrink-0 mt-0.5 select-none">
                    {isUser ? (
                      <div 
                        className="w-7 h-7 rounded-full flex items-center justify-center font-mono text-xs font-bold"
                        style={{
                          background: 'var(--color-surface-2, #1c2128)',
                          border: '1px solid var(--color-border-default, #30363d)',
                          color: '#e6edf3'
                        }}
                      >
                        U
                      </div>
                    ) : (
                      <div 
                        className="w-7 h-7 rounded-full flex items-center justify-center shadow-sm"
                        style={{
                          background: 'rgba(167,139,250,0.15)',
                          border: '1px solid rgba(167,139,250,0.3)',
                          color: '#a78bfa'
                        }}
                      >
                        <Sparkles className="w-4 h-4" />
                      </div>
                    )}
                  </div>

                  {/* Message Content */}
                  <div className="flex-1 min-w-0 space-y-2">
                    <div className="flex items-center gap-2 select-none">
                      <span className="font-semibold text-xs text-white">
                        {isUser ? 'You' : msg.author || 'Claude 3.7 Sonnet'}
                      </span>
                      {msg.badge && (
                        <span 
                          className="font-mono text-[10px] px-1.5 py-0.2 rounded font-bold"
                          style={{
                            background: 'rgba(63,185,80,0.12)',
                            color: '#3fb950',
                            border: '1px solid rgba(63,185,80,0.25)'
                          }}
                        >
                          {msg.badge}
                        </span>
                      )}
                    </div>

                    {/* Thinking Trace Accordion */}
                    {msg.stepName && (
                      <ThinkingTrace
                        stepName={msg.stepName}
                        duration={msg.duration}
                        expanded={!!expandedTraces[idx]}
                        onToggle={() => setExpanded(p => ({ ...p, [idx]: !p[idx] }))}
                      />
                    )}

                    {/* Message Body */}
                    {isUser ? (
                      <div 
                        className="p-3.5 rounded-2xl text-xs leading-relaxed text-white max-w-2xl"
                        style={{
                          background: 'var(--color-surface-2, #1c2128)',
                          border: '1px solid var(--color-border-default, #30363d)'
                        }}
                      >
                        {msg.text}
                      </div>
                    ) : (
                      <div className="markdown-body text-xs leading-relaxed text-[#e6edf3]">
                        <ReactMarkdown
                          remarkPlugins={[remarkGfm]}
                          components={{
                            code({ node, inline, className, children, ...props }: any) {
                              const match = /language-(\w+)/.exec(className || '');
                              const codeStr = String(children).replace(/\n$/, '');
                              const codeId = `code-${idx}-${match?.[1] || 'text'}`;
                              const copied = copiedId === codeId;

                              if (!inline && match) {
                                return (
                                  <CodeBlock
                                    lang={match[1]}
                                    code={codeStr}
                                    onCopy={() => handleCopyCode(codeStr, codeId)}
                                    copied={copied}
                                  />
                                );
                              }
                              return (
                                <code
                                  style={{
                                    fontFamily: 'var(--font-mono, JetBrains Mono, monospace)',
                                    fontSize: '0.75rem',
                                    background: 'rgba(110,118,129,0.15)',
                                    border: '1px solid rgba(110,118,129,0.25)',
                                    padding: '0.1em 0.4em',
                                    borderRadius: '4px',
                                    color: '#e6edf3',
                                  }}
                                  {...props}
                                >
                                  {children}
                                </code>
                              );
                            }
                          }}
                        >
                          {msg.text}
                        </ReactMarkdown>
                      </div>
                    )}
                  </div>
                </motion.div>
              );
            })
          )}
          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* ── 3. Floating Claude Pill Composer ── */}
      <div className="shrink-0 p-4 pt-2">
        <div className="max-w-3xl mx-auto w-full">
          {/* Task Mode Banner */}
          {isTaskMode && (
            <div 
              className="mb-2 flex items-center justify-between px-3 py-1.5 rounded-lg text-xs"
              style={{
                background: 'rgba(88,166,255,0.08)',
                border: '1px solid rgba(88,166,255,0.25)',
                color: '#58a6ff'
              }}
            >
              <div className="flex items-center gap-2">
                <Target className="w-3.5 h-3.5" />
                <span>Task Mode Active: Next prompt will be tracked under SADE Invariant criteria</span>
              </div>
              <button 
                onClick={() => setIsTaskMode(false)}
                className="text-[10px] text-[#8b949e] hover:text-white"
              >
                Cancel
              </button>
            </div>
          )}

          {/* Floating Pill Container */}
          <div 
            className="rounded-2xl shadow-xl transition focus-within:border-accent"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              border: '1px solid var(--color-border-default, #30363d)',
            }}
          >
            <textarea
              ref={textareaRef}
              rows={2}
              value={inputText}
              onChange={e => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={isTaskMode ? "Describe task goal, criteria, and invariant requirements..." : "Reply to Claude or instruct an agent task... (Shift+Enter for newline)"}
              className="w-full px-4 pt-3 pb-1 bg-transparent text-xs text-white placeholder-[#6e7681] focus:outline-none resize-none font-sans"
              style={{ minHeight: '52px', maxHeight: '180px' }}
            />

            {/* Bottom Controls Row inside Composer */}
            <div 
              className="flex items-center justify-between px-3 py-2 text-xs"
              style={{ borderTop: '1px solid var(--color-border-muted, #21262d)' }}
            >
              <div className="flex items-center gap-1.5 select-none">
                <button 
                  className="p-1.5 rounded text-[#8b949e] hover:text-white hover:bg-surface-2 transition"
                  title="Add context file"
                >
                  <Paperclip className="w-3.5 h-3.5" />
                </button>

                <button 
                  onClick={() => setIsTaskMode(!isTaskMode)}
                  className={`px-2 py-1 rounded-md text-[11px] font-medium flex items-center gap-1 transition ${
                    isTaskMode 
                      ? 'bg-blue-500/20 text-[#58a6ff] border border-blue-500/30' 
                      : 'text-[#8b949e] hover:text-white hover:bg-surface-2'
                  }`}
                  title="Turn prompt into a supervised Task"
                >
                  <Target className="w-3 h-3" />
                  <span>Task</span>
                </button>

                {/* Model Pill */}
                <span 
                  className="font-mono text-[10px] px-2 py-0.5 rounded text-[#a78bfa] border border-[#a78bfa]/20 bg-[#a78bfa]/5 hidden sm:inline"
                >
                  Claude 3.7 Sonnet
                </span>

                <span 
                  className="font-mono text-[10px] px-2 py-0.5 rounded text-[#3fb950] border border-[#3fb950]/20 bg-[#3fb950]/5 hidden sm:inline"
                >
                  Thinking: 8k
                </span>
              </div>

              {/* Send Button (Circular Claude-style Arrow) */}
              <button
                onClick={handleSend}
                disabled={!inputText.trim()}
                className="w-7 h-7 rounded-full flex items-center justify-center transition-all disabled:opacity-40"
                style={{
                  background: inputText.trim() ? '#e6edf3' : 'var(--color-surface-2, #1c2128)',
                  color: inputText.trim() ? '#0d1117' : '#6e7681',
                }}
                title="Send message (Enter)"
              >
                <ArrowUp className="w-4 h-4 stroke-[2.5]" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};

export default ChatSection;
