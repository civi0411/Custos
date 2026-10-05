import React, { useState, useRef, useEffect } from 'react';
import { 
  RotateCcw, 
  Download, 
  ArrowUp, 
  Sparkles, 
  CheckCircle2, 
  ChevronDown, 
  ChevronUp, 
  Copy, 
  Check, 
  Code2, 
  ShieldCheck, 
  Zap, 
  Bot
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

const QUICK_STARTERS = {
  engineering: [
    { label: 'Explain changes', icon: Code2, prompt: 'Explain the changes linked to this task and any remaining risks.' },
    { label: 'Plan tests', icon: ShieldCheck, prompt: 'Suggest tests for this task and explain what they would verify.' },
  ],
  research: [
    { label: 'Compare sources', icon: Sparkles, prompt: 'Compare the available sources and identify where they disagree.' },
    { label: 'Find gaps', icon: ShieldCheck, prompt: 'Which claims still need evidence or a better source?' },
  ],
  assistant: [
    { label: 'Draft response', icon: Sparkles, prompt: 'Draft a response for my review.' },
    { label: 'Next steps', icon: Zap, prompt: 'Summarize the next steps for this task.' },
  ],
  general: [
    { label: 'Summarize', icon: Sparkles, prompt: 'Summarize the current task and its open questions.' },
    { label: 'Next steps', icon: Zap, prompt: 'What are the next useful steps?' },
  ],
};

export const ChatSection: React.FC<ChatSectionProps> = ({
  session,
  onSendMessage,
  onClearHistory,
  onShowToast
}) => {
  const [inputText, setInputText] = useState('');
  const [expandedThoughts, setExpandedThoughts] = useState<Record<number, boolean>>({});
  const [copiedCodeIndex, setCopiedCodeIndex] = useState<string | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [session?.messages]);

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

  const handleTextChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    setInputText(val);
  };

  const toggleThought = (index: number) => {
    setExpandedThoughts((prev) => ({ ...prev, [index]: !prev[index] }));
  };

  const handleCopyCode = (code: string, id: string) => {
    navigator.clipboard.writeText(code);
    setCopiedCodeIndex(id);
    onShowToast('Copied code to clipboard');
    setTimeout(() => setCopiedCodeIndex(null), 2000);
  };

  const exportConversation = () => {
    if (!session) return;
    const markdown = session.messages.map((message) => `## ${message.author}\n\n${message.text}`).join('\n\n');
    const url = URL.createObjectURL(new Blob([`# ${session.title}\n\n${markdown}\n`], { type: 'text/markdown' }));
    const link = document.createElement('a');
    link.href = url;
    link.download = `${session.title.replace(/[^a-z0-9-_]+/gi, '-').toLowerCase() || 'conversation'}.md`;
    link.click();
    URL.revokeObjectURL(url);
    onShowToast('Conversation downloaded as Markdown');
  };

  if (!session) {
    return (
      <section className="flex-1 flex flex-col items-center justify-center bg-canvas text-neutral-500 text-xs font-sans">
        <Sparkles className="w-8 h-8 mb-2 opacity-40 text-brand-blue" />
        <p>Select a session or start a new conversation</p>
      </section>
    );
  }

  return (
    <section className="flex-1 flex flex-col bg-canvas overflow-hidden min-w-0 relative border-r border-surface-border font-sans">
      {/* 1. Assistant Header with Model Selector */}
      <div className="h-11 border-b border-surface-border px-3 sm:px-4 flex items-center justify-between bg-surface text-xs shrink-0 min-w-0">
        <div className="flex items-center gap-2 truncate min-w-0">
          <div className="w-5 h-5 rounded-md bg-amber-500/10 border border-amber-500/20 flex items-center justify-center shrink-0">
            <Bot className="w-3.5 h-3.5 text-amber-400" />
          </div>
          <span className="font-semibold text-white truncate text-xs">
            {session.title}
          </span>

          <span className="rounded border border-surface-border px-2 py-0.5 text-[11px] text-neutral-400">{session.model || 'Model not reported'}</span>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-1 shrink-0">
          <button 
            onClick={onClearHistory} 
            className="p-1 hover:bg-surface-elevated rounded text-neutral-500 hover:text-neutral-300 transition" 
            title="Reset Conversation"
          >
            <RotateCcw className="w-3.5 h-3.5" />
          </button>
          <button 
            onClick={exportConversation}
            className="p-1 hover:bg-surface-elevated rounded text-neutral-500 hover:text-neutral-300 transition" 
            title="Export session"
          >
            <Download className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* 2. Messages Scrollable Area */}
      <div className="flex-1 bg-surface overflow-y-auto p-3 sm:p-4 space-y-4 min-w-0 select-text">
        {session.messages.map((msg, index) => {
          const isUser = msg.role === 'user';
          const isThoughtExpanded = expandedThoughts[index] ?? false;

          return (
            <motion.div 
              key={index} 
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.2 }}
              className="flex gap-2.5 sm:gap-3 max-w-3xl min-w-0 select-text"
            >
              {isUser ? (
                <>
                  <div className="w-6 h-6 rounded-lg bg-neutral-800 border border-surface-border flex items-center justify-center text-[10px] font-mono shrink-0 text-neutral-300 select-none mt-0.5 font-bold shadow-sm">
                    U
                  </div>
                  <div className="space-y-1 flex-1 min-w-0">
                    <div className="text-[11px] text-neutral-400 font-medium select-none">You</div>
                    <div className="text-[13px] text-neutral-100 bg-[#151822] border border-[#232838] rounded-xl p-3 leading-relaxed break-words select-text shadow-sm">
                      {msg.text}
                    </div>
                  </div>
                </>
              ) : (
                <>
                  <div className="w-6 h-6 rounded-lg bg-surface-elevated border border-surface-border flex items-center justify-center shrink-0 overflow-hidden shadow-sm select-none mt-0.5">
                    <img src="/assets/custos-owl.png" alt="Custos" className="w-4 h-4 object-contain" />
                  </div>
                  <div className="space-y-2 flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-[11px] text-neutral-200 font-semibold select-none">{msg.author}</span>
                      {msg.badge && (
                        <span className="text-[10px] text-neutral-400 font-mono bg-surface-elevated px-1.5 py-0.5 rounded border border-surface-border select-none">
                          {msg.badge}
                        </span>
                      )}
                    </div>

                    {/* Collapsible Chain of Thought (Thinking Process) */}
                    {msg.stepName && (
                      <div className="rounded-lg border border-surface-border/80 bg-[#12141c] overflow-hidden text-xs">
                        <button
                          onClick={() => toggleThought(index)}
                          className="w-full px-2.5 py-1.5 flex items-center justify-between text-neutral-400 hover:text-neutral-200 transition text-[11px] font-mono"
                        >
                          <div className="flex items-center gap-2 truncate">
                            <CheckCircle2 className="w-3 h-3 text-emerald-400 shrink-0" />
                            <span className="truncate">Thinking Process: {msg.stepName}</span>
                          </div>
                          <div className="flex items-center gap-1.5 shrink-0 ml-2">
                            {msg.duration && (
                              <span className="text-neutral-500 text-[10px]">{msg.duration}</span>
                            )}
                            {isThoughtExpanded ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
                          </div>
                        </button>

                        <AnimatePresence>
                          {isThoughtExpanded && (
                            <motion.div
                              initial={{ height: 0, opacity: 0 }}
                              animate={{ height: 'auto', opacity: 1 }}
                              exit={{ height: 0, opacity: 0 }}
                              className="px-3 py-2 border-t border-surface-border/60 bg-[#0d0f14] text-[11px] font-mono text-neutral-400 space-y-1"
                            >
                              <div className="flex items-center gap-1.5 text-neutral-300">
                                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                                <span>Checked AST symbols & invariants</span>
                              </div>
                              <div className="flex items-center gap-1.5 text-neutral-300">
                                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                                <span>Bounded memory limits and structural context</span>
                              </div>
                              <div className="flex items-center gap-1.5 text-neutral-300">
                                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                                <span>Synthesized diff with zero-IO proof-closure</span>
                              </div>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    )}

                    {/* Markdown Message Body with Custom Code Blocks */}
                    <div className="text-[13px] text-neutral-300 leading-relaxed p-0.5 select-text markdown-body prose-sm prose-invert max-w-none">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                          code({ node, inline, className, children, ...props }: any) {
                            const match = /language-(\w+)/.exec(className || '');
                            const codeString = String(children).replace(/\n$/, '');
                            const codeId = `code-${index}-${match?.[1] || 'text'}`;
                            const isCopied = copiedCodeIndex === codeId;

                            if (!inline && match) {
                              return (
                                <div className="my-2 rounded-xl border border-[#232838] bg-[#0d0f14] overflow-hidden shadow-md">
                                  <div className="flex items-center justify-between px-3 py-1.5 bg-[#141720] border-b border-[#232838] text-[11px] font-mono text-neutral-400">
                                    <span>{match[1]}</span>
                                    <button
                                      onClick={() => handleCopyCode(codeString, codeId)}
                                      className="flex items-center gap-1 hover:text-white transition px-1.5 py-0.5 rounded bg-surface-elevated hover:bg-neutral-700"
                                      title="Copy code"
                                    >
                                      {isCopied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
                                      <span className="text-[10px]">{isCopied ? 'Copied!' : 'Copy'}</span>
                                    </button>
                                  </div>
                                  <pre className="p-3 text-[12px] font-mono overflow-x-auto text-neutral-200 leading-snug">
                                    <code className={className} {...props}>
                                      {children}
                                    </code>
                                  </pre>
                                </div>
                              );
                            }
                            return (
                              <code className="bg-[#181b24] text-neutral-200 px-1.5 py-0.5 rounded text-[11.5px] font-mono border border-[#262c3a]" {...props}>
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
                </>
              )}
            </motion.div>
          );
        })}
        <div ref={messagesEndRef} />
      </div>

      {/* 3. Quick Prompt Starter Chips */}
      <div className="px-3 pt-2 pb-1 bg-surface border-t border-surface-border/60 flex items-center gap-1.5 overflow-x-auto no-scrollbar">
        {(session.pack === 'engineering' ? QUICK_STARTERS.engineering : session.pack === 'research' ? QUICK_STARTERS.research : session.pack === 'assistant' ? QUICK_STARTERS.assistant : QUICK_STARTERS.general).map((starter) => {
          const Icon = starter.icon;
          return (
            <button
              key={starter.label}
              onClick={() => {
                setInputText(starter.prompt);
                textareaRef.current?.focus();
              }}
              className="px-2.5 py-1 rounded-full bg-surface-card hover:bg-surface-elevated border border-surface-border/80 hover:border-brand-blue/50 text-[11px] text-neutral-300 hover:text-white transition flex items-center gap-1.5 shrink-0 shadow-sm"
            >
              <Icon className="w-3 h-3 text-brand-blue" />
              <span>{starter.label}</span>
            </button>
          );
        })}
      </div>

      {/* Chat input */}
      <div className="p-2.5 sm:p-3 bg-surface shrink-0 relative">

        <div className="bg-[#12141c] border border-[#232838] rounded-xl p-2 focus-within:border-brand-blue shadow-lg transition">
          <textarea 
            ref={textareaRef}
            rows={2}
            value={inputText}
            onChange={handleTextChange}
            onKeyDown={handleKeyDown}
            placeholder="Ask about this task or describe what to do next..." 
            className="w-full bg-transparent resize-none text-[12px] placeholder:text-[12px] text-white placeholder-neutral-500 focus:outline-none px-1.5 py-0.5 leading-relaxed font-sans"
          />
          
          <div className="flex items-center justify-between pt-2 border-t border-[#1e222d] text-xs">
            <div className="flex items-center gap-1.5">
              <span className="text-[10px] text-neutral-500">Ask, then open a workbench to review resources.</span>
            </div>

            <div className="flex items-center gap-2">
              <span className="text-[10px] text-neutral-500 font-mono hidden md:inline">↵ to send</span>
              <button 
                onClick={handleSend}
                className="w-7 h-7 bg-brand-blue hover:bg-blue-600 rounded-lg flex items-center justify-center text-white transition shadow-md shadow-brand-blueGlow"
                title="Send message"
              >
                <ArrowUp className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};

export default ChatSection;
