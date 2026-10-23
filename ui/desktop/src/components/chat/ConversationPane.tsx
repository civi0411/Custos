import React, { useRef, useState, useEffect, useMemo, useCallback, useSyncExternalStore } from 'react';
import {
  ArrowUp,
  Compass,
  FileCode,
  Mic,
  Paperclip,
  Plus,
  Square as StopIcon,
  X,
} from 'lucide-react';
import type { Session, ChatMessage } from '@/types';
import type { ActiveWorkbench } from '@/components/shell/AppHeader';
import type { Task, ExecutionWorkspace } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { ModelSelector } from './ModelSelector';
import { AgentActionCard } from './AgentActionCard';
import { AgentWorkingStatus } from './AgentWorkingStatus';
import { ComposerPlusMenu } from './ComposerPlusMenu';
import { ConversationMarkdown } from './ConversationMarkdown';
import { TaskAnchorBanner } from '@/components/shell/TaskAnchorBanner';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import { useLiveRunSnapshot } from '@/context/app/scopedLiveRun';

// Ephemeral presentation drafts only; daemon messages remain the durable record.
const conversationDrafts = new Map<string, string>();
export function readConversationDraft(sessionId: string): string {
  return conversationDrafts.get(sessionId) || '';
}
export function writeConversationDraft(sessionId: string, text: string): void {
  if (!sessionId) return;
  if (text) conversationDrafts.set(sessionId, text); else conversationDrafts.delete(sessionId);
  window.dispatchEvent(new Event('custos:composer-updated'));
}
export function appendConversationDraft(sessionId: string, text: string): void {
  if (!sessionId || !text) return;
  const previous = readConversationDraft(sessionId);
  writeConversationDraft(sessionId, previous ? `${previous}\n\n${text}` : text);
}
export function clearAcceptedConversationDraft(sessionId: string, submitted: string, accepted: boolean): boolean {
  if (!accepted || readConversationDraft(sessionId) !== submitted) return false;
  writeConversationDraft(sessionId, '');
  return true;
}
const subscribeComposer = (listener: () => void) => {
  window.addEventListener('custos:composer-updated', listener);
  return () => window.removeEventListener('custos:composer-updated', listener);
};

export interface ConversationPaneProps {
  session: Partial<Session> & { id: string; title: string; messages: ChatMessage[] };
  boundTask?: Task | null;
  boundWorkspace?: ExecutionWorkspace | null;
  taskAttentionCount?: number;
  mode: ActiveWorkbench;
  onSendMessage: (text: string) => Promise<boolean> | void;
  onOpenTab?: (id: OrcaTabId, url?: string, title?: string) => void;
  onSelectTask?: (id: string) => void;
  onClearTask?: () => void;
  onPromoteToTask?: () => void;
  onOpenSplitCanvas?: () => void;
  onShowToast?: (msg: string) => void;
  isCompact?: boolean;
}

export function ConversationPane({
  session,
  boundTask,
  boundWorkspace,
  taskAttentionCount = 0,
  mode,
  onSendMessage,
  onOpenTab,
  onSelectTask,
  onClearTask: _onClearTask,
  onPromoteToTask,
  onOpenSplitCanvas,
  onShowToast,
  isCompact = false,
}: ConversationPaneProps) {
  const inputText = useSyncExternalStore(subscribeComposer, () => readConversationDraft(session.id), () => '');
  const setInputText = (value: React.SetStateAction<string>) => writeConversationDraft(session.id,
    typeof value === 'function' ? value(readConversationDraft(session.id)) : value);
  const submitting = useRef(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [composerMode, setComposerMode] = useState<'chat' | 'task'>('chat');
  const [attachedFiles, setAttachedFiles] = useState<File[]>([]);
  const [isPlusMenuOpen, setIsPlusMenuOpen] = useState(false);
  const [scrollEdges, setScrollEdges] = useState({ top: false, bottom: false });

  const fileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const { selectedModel, handleSelectModel, handleStopRun, openSkills, openPlugins } = useAppContext();
  useEffect(() => {
    setAttachedFiles([]);
    setIsPlusMenuOpen(false);
  }, [session.id]);

  const hasMessages = session?.messages && session.messages.length > 0;
  const liveMessage = useMemo(() => {
    if (!session?.messages || session.messages.length === 0) return undefined;
    const reversed = [...session.messages].reverse();
    return reversed.find((m: ChatMessage) => m.role === 'assistant' && m.runStatus && !['completed', 'failed', 'cancelled', 'interrupted'].includes(m.runStatus));
  }, [session?.messages]);

  const liveSnapshot = useLiveRunSnapshot(liveMessage?.runId);

  // Handle scroll shadows
  const handleScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const isTop = el.scrollTop > 8;
    const isBottom = el.scrollHeight - el.scrollTop - el.clientHeight > 8;
    setScrollEdges({ top: isTop, bottom: isBottom });
  }, []);

  useEffect(() => {
    handleScroll();
  }, [session?.messages, handleScroll]);

  // Auto scroll on new message or live streaming token
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [session?.messages?.length, liveMessage?.text, liveSnapshot?.text]);

  const handleTriggerFileInput = () => {
    if (submitting.current) return;
    fileInputRef.current?.click();
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (submitting.current) return;
    if (e.target.files && e.target.files.length > 0) {
      const newFiles = Array.from(e.target.files);
      setAttachedFiles((prev) => [...prev, ...newFiles]);
    }
  };

  const handleRemoveFile = (index: number) => {
    if (submitting.current) return;
    setAttachedFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const handleSend = async () => {
    if (submitting.current) return;
    const submitted = inputText;
    const sessionId = session.id;
    const submittedFiles = attachedFiles;
    const text = submitted.trim();
    if (!sessionId || (!text && attachedFiles.length === 0)) return;
    submitting.current = true; setIsSubmitting(true);
    try {
      const contents = await Promise.all(attachedFiles.map(async file => {
        if (file.size > 100000) throw new Error(`File ${file.name} exceeds the text attachment limit.`);
        if (!/^(text\/|application\/(json|xml|javascript))/.test(file.type) && !/\.(txt|md|csv|ts|tsx|js|json|py|rs|yaml|yml|toml|log|xml|html|css)$/i.test(file.name)) {
          throw new Error(`Binary attachment ${file.name} is not supported by this text composer.`);
        }
        return `--- Attached File: ${file.name} ---\n${await file.text()}`;
      }));
      const prompt = [text, ...contents].filter(Boolean).join('\n\n');
      const accepted = await onSendMessage(prompt);
      clearAcceptedConversationDraft(sessionId, submitted, accepted === true);
      if (accepted === true) setAttachedFiles(current => current === submittedFiles ? [] : current);
    } catch (error) { onShowToast?.(`Message was not accepted: ${String(error)}`); }
    finally { submitting.current = false; setIsSubmitting(false); }
  };

  const handleSteer = async () => {
    const submitted = inputText;
    const text = submitted.trim();
    const sessionId = session.id;
    if (!text || !liveMessage?.runId || submitting.current) return;
    submitting.current = true; setIsSubmitting(true);
    try {
      await daemonClient.steerHarness(session.nativeHarnessId || 'claude-code', liveMessage.runId, text);
      clearAcceptedConversationDraft(sessionId, submitted, true);
      onShowToast?.('Additional guidance accepted by the agent.');
    } catch (error) { onShowToast?.(`Guidance was not accepted: ${String(error)}`); }
    finally { submitting.current = false; setIsSubmitting(false); }
  };

  const onStopClick = (runId: string) => {
    if (handleStopRun) {
      handleStopRun(runId);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (liveMessage) {
        if (inputText.trim()) handleSteer();
      } else {
        handleSend();
      }
    }
  };

  return (
    <div className={`flex flex-col h-full overflow-hidden bg-canvas relative select-text min-w-0 ${isCompact ? 'text-[12.5px]' : ''}`}>
      {/* Hidden file input */}
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileChange}
        className="hidden"
        multiple
      />

      {/* Main Dialogue Scroll Container */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto px-4 py-4 min-h-0 relative select-text"
      >
        {/* Task Anchor Banner (Decoupled Task & Session Anchor) */}
        <TaskAnchorBanner
          session={session as Session}
          task={boundTask}
          boundWorkspace={boundWorkspace}
          attentionCount={taskAttentionCount}
          hasMessages={Boolean(hasMessages)}
          onViewTask={() => {
            if (session.taskId && onSelectTask) {
              onSelectTask(session.taskId);
            }
          }}
          onOpenSplitCanvas={onOpenSplitCanvas}
          onPromoteToTask={onPromoteToTask || (() => {})}
        />

        {/* Empty State / Welcome Starters */}
        {!hasMessages ? (
          <div className="flex flex-col items-center justify-center min-h-[70vh] max-w-2xl mx-auto px-4 select-none">
            <div className="size-11 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center mb-4 text-white/80 shadow-inner">
              <Compass className="size-5 text-workbench-accent animate-pulse-subtle" />
            </div>
            <h2 className="text-xl font-semibold text-white tracking-tight mb-2">
              {mode === 'research'
                ? 'Research Copilot'
                : mode === 'copilot'
                ? 'Personal Copilot'
                : 'Engineering Copilot'}
            </h2>
            <p className="text-xs text-white/50 text-center max-w-md mb-8 leading-relaxed">
              {mode === 'research'
                ? 'Explore literature, extract scientific claims, and run empirical tests in notebooks.'
                : mode === 'copilot'
                ? 'Coordinate personal tasks, draft communications, and organize cowork activities.'
                : 'Engineering discussion, assign agent tasks, and modify code in isolated worktrees.'}
            </p>

            {/* Centered Large Composer Box */}
            <div className="w-full bg-[#18181b]/95 border border-white/15 rounded-2xl p-3 shadow-2xl transition-all focus-within:border-workbench-accent/50 focus-within:ring-1 focus-within:ring-workbench-accent/20">
              {/* Attached Files Preview */}
              {attachedFiles.length > 0 && (
                <div className="flex flex-wrap gap-1.5 pb-2 border-b border-white/10 mb-2">
                  {attachedFiles.map((file, idx) => (
                    <div
                      key={idx}
                      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-white/10 text-white/80 text-[11px] font-mono"
                    >
                      <Paperclip className="size-3 text-white/50" />
                      <span className="truncate max-w-[140px]">{file.name}</span>
                      <button
                        type="button"
                        onClick={() => handleRemoveFile(idx)}
                        className="text-white/40 hover:text-white/90 transition-colors ml-0.5 cursor-pointer"
                      >
                        <X className="size-3" />
                      </button>
                    </div>
                  ))}
                </div>
              )}

              {/* Textarea */}
              <textarea
                ref={textareaRef}
                rows={3}
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={
                  composerMode === 'chat' && !session.taskId
                    ? 'Ask Custos anything (Q&A, code analysis, explanation)...'
                    : 'Ask Custos or assign task: Plan, analyze code, execute actions...'
                }
                className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-white placeholder-white/35 text-[14px] resize-none leading-relaxed min-h-[64px]"
                autoFocus
              />

              {/* Bottom Toolbar inside the Box */}
              <div className="flex items-center justify-between pt-2 mt-1">
                <div className="flex items-center gap-2">
                  <div className="relative">
                    <button
                      type="button"
                      onClick={() => setIsPlusMenuOpen((prev) => !prev)}
                      className="size-8 rounded-full hover:bg-white/10 text-white/70 hover:text-white flex items-center justify-center transition cursor-pointer"
                      title="Add files, skills, plugins (⌘U)"
                    >
                      <Plus className="size-4" />
                    </button>
                    <ComposerPlusMenu
                      isOpen={isPlusMenuOpen}
                      onClose={() => setIsPlusMenuOpen(false)}
                      onAddFilesOrPhotos={handleTriggerFileInput}
                      onOpenSkills={openSkills}
                      onAddPlugins={openPlugins}
                      position="top"
                    />
                  </div>
                </div>

                <div className="flex items-center gap-1.5">
                  <ModelSelector
                    currentModel={session?.model || selectedModel}
                    onSelectModel={handleSelectModel}
                  />

                  <button
                    type="button"
                    onClick={handleSend}
                    disabled={isSubmitting || (!inputText.trim() && attachedFiles.length === 0)}
                    className={`size-8 rounded-full flex items-center justify-center transition ${
                      inputText.trim() || attachedFiles.length > 0
                        ? 'bg-[#2563eb] hover:bg-[#1d4ed8] text-white shadow-xs cursor-pointer'
                        : 'bg-white/10 text-white/30 cursor-not-allowed'
                    }`}
                  >
                    <ArrowUp className="size-4" />
                  </button>
                </div>
              </div>
            </div>

            {/* Chat vs Task Mode Selector Buttons */}
            <div className="mt-4 flex items-center gap-1.5 bg-white/5 border border-white/10 p-1 rounded-full shadow-inner">
              <button
                type="button"
                onClick={() => setComposerMode('chat')}
                className={`px-4 py-1 rounded-full text-xs font-medium transition cursor-pointer ${
                  composerMode === 'chat' && !session.taskId
                    ? 'bg-zinc-700/90 text-white shadow-xs font-semibold'
                    : 'text-white/50 hover:text-white/80'
                }`}
              >
                Chat
              </button>
              <button
                type="button"
                onClick={() => setComposerMode('task')}
                className={`px-4 py-1 rounded-full text-xs font-medium transition cursor-pointer ${
                  composerMode === 'task' || Boolean(session.taskId)
                    ? 'bg-zinc-700/90 text-white shadow-xs font-semibold'
                    : 'text-white/50 hover:text-white/80'
                }`}
              >
                Task
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-6 max-w-3xl mx-auto w-full">
            {session.messages.map((msg, i) => {
              const isUser = msg.role === 'user';
              const isLive = Boolean(liveMessage?.runId && msg.runId === liveMessage.runId);
              const text = isLive && liveSnapshot?.text !== undefined ? liveSnapshot.text : msg.text;
              const runStatus = isLive && liveSnapshot?.status ? liveSnapshot.status : msg.runStatus;
              const activities = isLive && liveSnapshot?.activities ? liveSnapshot.activities : msg.activities;
              const liveError = isLive && liveSnapshot?.error ? liveSnapshot.error : msg.liveError;

              return (
                <div
                  key={msg.id || i}
                  aria-label={isUser ? "User message" : "Agent message"}
                  className={isUser ? "conversation-user-message space-y-2 mb-4" : "space-y-2.5 pb-6 border-b border-border-muted/40 last:border-b-0 min-w-0"}
                >
                  <div className="conversation-markdown text-[13px] leading-relaxed text-fg-editor space-y-3 min-w-0 break-words [overflow-wrap:anywhere]">
                    <ConversationMarkdown text={text} />
                    {!text && msg.runId && !['completed', 'failed', 'cancelled', 'interrupted'].includes(runStatus || '') && (
                      <p role="status" className="text-xs text-fg-muted">
                        {runStatus === 'thinking' ? 'Thinking...' : runStatus === 'suspended' ? 'Suspended (Awaiting approval)...' : 'Running...'}
                      </p>
                    )}
                    {liveError && <p role="alert" className="mt-2 text-xs text-amber-400">{liveError}</p>}
                    {activities?.map((activity, index) => (
                      <AgentActionCard
                        key={`${msg.runId || i}-activity-${index}`}
                        activity={activity}
                        onOpenTab={onOpenTab}
                        onApproveAction={(actionId) =>
                          daemonClient.approveOutbox(actionId)
                            .then(() => onShowToast?.('Action approved'))
                            .catch((err) => onShowToast?.(String(err)))
                        }
                        onHandoffToCoding={(claimId, statement) =>
                          daemonClient.handoffClaimToCoding(claimId, statement)
                            .then(() => onShowToast?.('Scientific claim handed off to Engineering'))
                            .catch((err) => onShowToast?.(String(err)))
                        }
                        onSendAnswer={(answer) => {
                          if (liveMessage?.runId) {
                            const harnessId = session.nativeHarnessId || 'claude-code';
                            daemonClient.steerHarness(harnessId, liveMessage.runId, answer)
                              .then(() => onShowToast?.('Answer forwarded to Agent'))
                              .catch((err) => {
                                onShowToast?.(`Failed to forward answer to Agent: ${err?.message || err}`);
                              });
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

      {/* Scroll blur cues */}
      {hasMessages && (
        <>
          <div aria-hidden="true" className={`conversation-scroll-blur conversation-scroll-blur-top ${scrollEdges.top ? 'is-visible' : ''}`} />
          <div aria-hidden="true" className={`conversation-scroll-blur conversation-scroll-blur-bottom ${scrollEdges.bottom ? 'is-visible' : ''}`} />
        </>
      )}

      {/* DOCKED BOTTOM COMPOSER - When hasMessages */}
      {hasMessages && (
        <div className="shrink-0 px-4 py-3 bg-canvas border-t border-white/5">
          {liveMessage && (
            <div className="max-w-3xl mx-auto mb-2.5">
              <AgentWorkingStatus
                status={liveMessage.runStatus}
                runId={liveMessage.runId}
                harnessName={
                  mode === 'research'
                    ? 'Research Copilot'
                    : mode === 'copilot'
                    ? 'Personal Copilot'
                    : session.nativeHarnessId
                    ? `Coding Agent (${session.nativeHarnessId})`
                    : 'Coding Assistant'
                }
                currentActivity={liveMessage.activities?.[liveMessage.activities.length - 1]}
                onStop={() => liveMessage.runId && onStopClick(liveMessage.runId)}
                onFocusSteer={() => textareaRef.current?.focus()}
              />
            </div>
          )}

          <div className="max-w-3xl mx-auto">
            {attachedFiles.length > 0 && (
              <div className="flex flex-wrap gap-1.5 pb-2 mb-1.5">
                {attachedFiles.map((file, idx) => (
                  <div
                    key={idx}
                    className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-white/10 text-white/80 text-[11px] font-mono"
                  >
                    <FileCode className="size-3 text-white/50" />
                    <span className="truncate max-w-[140px]">{file.name}</span>
                    <button
                      type="button"
                      onClick={() => handleRemoveFile(idx)}
                      className="text-white/40 hover:text-white/90 transition-colors ml-0.5 cursor-pointer"
                    >
                      <X className="size-3" />
                    </button>
                  </div>
                ))}
              </div>
            )}

            <div className="flex items-center gap-2 bg-[#18181b] border border-white/15 rounded-2xl px-3 py-1.5 shadow-xl transition-all focus-within:border-white/30">
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setIsPlusMenuOpen((prev) => !prev)}
                  className="size-8 rounded-full hover:bg-white/10 text-white/70 hover:text-white flex items-center justify-center transition cursor-pointer"
                  title="Add files, skills, plugins (⌘U)"
                >
                  <Plus className="size-4" />
                </button>
                <ComposerPlusMenu
                  isOpen={isPlusMenuOpen}
                  onClose={() => setIsPlusMenuOpen(false)}
                  onAddFilesOrPhotos={handleTriggerFileInput}
                  onOpenSkills={openSkills}
                  onAddPlugins={openPlugins}
                  position="top"
                />
              </div>

              <textarea
                ref={textareaRef}
                rows={1}
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={
                  liveMessage
                    ? 'Agent is working… Type guidance here to steer agent in-flight (or press Stop/Esc)...'
                    : composerMode === 'chat' && !session.taskId
                    ? 'Ask Custos...'
                    : 'Ask Custos or assign task...'
                }
                className="flex-1 bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-white placeholder-white/40 text-[13.5px] resize-none leading-relaxed py-1.5 max-h-32 min-h-[28px]"
              />

              <div className="flex items-center gap-1.5 shrink-0">
                {session.nativeHarnessId && (
                  <span className="rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[11px] text-white/80 shrink-0 font-mono">
                    {session.nativeHarnessId}
                  </span>
                )}

                <ModelSelector
                  currentModel={session?.model || selectedModel}
                  onSelectModel={handleSelectModel}
                />

                <button
                  type="button"
                  className="size-8 rounded-full hover:bg-white/10 text-white/60 hover:text-white flex items-center justify-center transition cursor-pointer"
                  title="Voice input is not configured"
                  disabled
                >
                  <Mic className="size-4" />
                </button>

                <button
                  type="button"
                  onClick={(event) => {
                    if (event.detail > 1 || isSubmitting) return;
                    if (liveMessage) {
                      if (inputText.trim()) {
                        handleSteer();
                      } else if (liveMessage.runId) {
                        onStopClick(liveMessage.runId);
                      }
                    } else {
                      handleSend();
                    }
                  }}
                  disabled={isSubmitting || (!liveMessage && (!inputText.trim() && attachedFiles.length === 0))}
                  className={`size-8 rounded-full flex items-center justify-center transition cursor-pointer ${
                    liveMessage
                      ? inputText.trim()
                        ? 'bg-amber-600 hover:bg-amber-500 text-white'
                        : 'bg-rose-600 hover:bg-rose-500 text-white'
                      : inputText.trim() || attachedFiles.length > 0
                      ? 'bg-[#2563eb] hover:bg-[#1d4ed8] text-white'
                      : 'bg-white/10 text-white/30 cursor-not-allowed'
                  }`}
                >
                  {liveMessage ? (
                    inputText.trim() ? (
                      <Compass className="size-4 animate-spin-slow" aria-hidden="true" />
                    ) : (
                      <StopIcon className="size-3.5 fill-current" aria-hidden="true" />
                    )
                  ) : (
                    <ArrowUp className="size-4" aria-hidden="true" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
