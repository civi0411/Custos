import React, { useState, useRef, useEffect } from 'react';
import { RotateCcw, Download, Paperclip, Terminal, ArrowUp, Sparkles, CheckCircle2 } from 'lucide-react';
import { Session } from '../types';
import { Tooltip } from './Tooltip';

interface ChatSectionProps {
  session: Session | null;
  onSendMessage: (text: string) => void;
  onClearHistory: () => void;
  onShowToast: (msg: string) => void;
}

export const ChatSection: React.FC<ChatSectionProps> = ({
  session,
  onSendMessage,
  onClearHistory,
  onShowToast
}) => {
  const [inputText, setInputText] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

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

  if (!session) {
    return (
      <section className="flex-1 flex flex-col items-center justify-center bg-canvas text-neutral-500 text-xs">
        <Sparkles className="w-8 h-8 mb-2 opacity-40 text-brand-blue" />
        <p>Select a session to view conversation</p>
      </section>
    );
  }

  return (
    <section className="flex-1 flex flex-col bg-surface overflow-hidden min-w-0 relative border-r border-surface-border">
      {/* Chat Header */}
      <div className="h-11 px-3 sm:px-4 flex items-center justify-between bg-surface/95 backdrop-blur-sm text-xs shrink-0 min-w-0 z-20">
        <div className="flex items-center gap-2 truncate min-w-0">
          <span className="w-2 h-2 rounded-full bg-emerald-400 shrink-0"></span>
          <span className="font-semibold text-white truncate">{session.title}</span>
          <span className="text-neutral-600 hidden sm:inline">/</span>
          <span className="text-[10px] font-mono text-neutral-400 bg-surface-card px-1.5 py-0.5 rounded border border-surface-border shrink-0 hidden sm:inline">
            {session.model}
          </span>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <Tooltip content="Clear conversation" position="bottom">
            <button 
              onClick={onClearHistory} 
              className="p-1 hover:bg-surface-elevated rounded text-neutral-500 hover:text-neutral-300 transition" 
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
          </Tooltip>
          <Tooltip content="Export session" position="bottom" align="end">
            <button 
              onClick={() => onShowToast('Exporting session context...')} 
              className="p-1 hover:bg-surface-elevated rounded text-neutral-500 hover:text-neutral-300 transition" 
            >
              <Download className="w-3.5 h-3.5" />
            </button>
          </Tooltip>
        </div>
      </div>

      {/* Chat Messages Container with Soft Edge Fade */}
      <div 
        className="flex-1 bg-surface overflow-y-auto overflow-x-hidden px-3 sm:px-5 pt-8 pb-8 min-w-0 select-text chat-scroll-mask"
        style={{
          maskImage: 'linear-gradient(to bottom, transparent 0px, rgba(0, 0, 0, 0.08) 6px, rgba(0, 0, 0, 0.35) 16px, rgba(0, 0, 0, 0.8) 28px, black 40px, black calc(100% - 40px), rgba(0, 0, 0, 0.8) calc(100% - 28px), rgba(0, 0, 0, 0.35) calc(100% - 16px), rgba(0, 0, 0, 0.08) calc(100% - 6px), transparent 100%)',
          WebkitMaskImage: 'linear-gradient(to bottom, transparent 0px, rgba(0, 0, 0, 0.08) 6px, rgba(0, 0, 0, 0.35) 16px, rgba(0, 0, 0, 0.8) 28px, black 40px, black calc(100% - 40px), rgba(0, 0, 0, 0.8) calc(100% - 28px), rgba(0, 0, 0, 0.35) calc(100% - 16px), rgba(0, 0, 0, 0.08) calc(100% - 6px), transparent 100%)'
        }}
      >
        <div className="max-w-3xl w-full mx-auto space-y-4">
          {session.messages.map((msg, index) => {
            const isUser = msg.role === 'user';
            return (
              <div key={index} className="flex gap-2.5 sm:gap-3 w-full min-w-0 select-text">
                {isUser ? (
                  <>
                    <div className="w-6 h-6 rounded bg-neutral-800 border border-surface-border flex items-center justify-center text-[10px] font-mono shrink-0 text-neutral-300 select-none">
                      U
                    </div>
                    <div className="space-y-1 flex-1 min-w-0">
                      <div className="text-[11px] text-neutral-400 font-medium select-none">{msg.author}</div>
                      <div className="text-xs text-neutral-200 bg-surface-card border border-surface-border rounded-xl p-3 leading-relaxed break-words [overflow-wrap:anywhere] [word-break:break-word] whitespace-pre-wrap select-text min-w-0">
                        {msg.text}
                      </div>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="w-6 h-6 rounded-lg bg-surface-elevated border border-surface-border flex items-center justify-center shrink-0 overflow-hidden shadow-sm select-none">
                      <img src="/assets/custos-owl.png" alt="Custos" className="w-4 h-4 object-contain" />
                    </div>
                    <div className="space-y-2 flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap min-w-0">
                        <span className="text-[11px] text-neutral-200 font-semibold select-none">{msg.author}</span>
                        {msg.badge && (
                          <span className="text-[10px] text-neutral-400 font-mono bg-surface-elevated px-1.5 py-0.5 rounded border border-surface-border select-none max-w-full truncate">
                            {msg.badge}
                          </span>
                        )}
                      </div>

                      {msg.stepName && (
                        <div className="flex items-center gap-2 text-[11px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2.5 py-1 rounded-lg font-mono select-none min-w-0 max-w-full overflow-hidden">
                          <CheckCircle2 className="w-3 h-3 shrink-0" />
                          <span className="truncate">{msg.stepName}</span>
                          {msg.duration && (
                            <span className="text-neutral-500 text-[10px] ml-auto shrink-0">{msg.duration}</span>
                          )}
                        </div>
                      )}

                      <div className="text-xs text-neutral-300 leading-relaxed bg-surface-card/60 border border-surface-border/80 rounded-xl p-3 select-text whitespace-pre-wrap break-words [overflow-wrap:anywhere] [word-break:break-word] min-w-0">
                        {msg.text}
                      </div>
                    </div>
                  </>
                )}
              </div>
            );
          })}
          <div ref={messagesEndRef} />
        </div>
      </div>

      {/* Chat Input Bar */}
      <div className="p-2.5 sm:p-4 bg-surface/95 backdrop-blur-sm shrink-0 z-10">
        <div className="max-w-3xl w-full mx-auto">
          <div className="bg-surface-card border border-surface-border rounded-xl p-2 focus-within:border-brand-blue shadow-lg transition">
            <textarea 
              rows={2}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask Custos to refactor, debug, or generate code diffs..." 
              className="w-full bg-transparent resize-none text-[11px] placeholder:text-[11px] text-white placeholder-neutral-500 focus:outline-none px-1.5 py-0.5 leading-relaxed font-sans max-h-36 overflow-y-auto"
            />
            
            <div className="flex items-center justify-between pt-2 border-t border-surface-border/50 text-xs">
              <div className="flex items-center gap-1.5">
                <button 
                  onClick={() => onShowToast('File attachment opened')} 
                  className="px-2 py-0.5 bg-surface-elevated hover:bg-surface-hover rounded text-neutral-400 hover:text-white flex items-center gap-1 text-[10px] font-medium border border-surface-border transition"
                >
                  <Paperclip className="w-2.5 h-2.5" />
                  <span className="hidden sm:inline">Attach</span>
                </button>
                <button 
                  onClick={() => onShowToast('Executing code block in terminal...')} 
                  className="px-2 py-0.5 bg-surface-elevated hover:bg-surface-hover rounded text-neutral-400 hover:text-white flex items-center gap-1 text-[10px] font-medium border border-surface-border transition"
                >
                  <Terminal className="w-2.5 h-2.5" />
                  <span className="hidden sm:inline">Terminal</span>
                </button>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-[10px] text-neutral-500 font-mono hidden md:inline">↵ to send</span>
                <button 
                  onClick={handleSend}
                  className="w-7 h-7 bg-brand-blue hover:bg-blue-600 rounded-lg flex items-center justify-center text-white transition shadow-md shadow-brand-blueGlow"
                >
                  <ArrowUp className="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
};
