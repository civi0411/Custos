import React from 'react';
import {
  ArrowUp,
  Compass,
  FileCode,
  Mic,
  Plus,
  Square as StopIcon,
  X,
} from 'lucide-react';
import type { ChatMessage, Session } from '@/types';
import type { ActiveWorkbench } from '@/types/workbench';
import { AgentWorkingStatus } from './AgentWorkingStatus';
import { ComposerPlusMenu } from './ComposerPlusMenu';
import { ModelSelector } from './ModelSelector';

interface ConversationDockedComposerProps {
  mode: ActiveWorkbench;
  session: Session;
  liveMessage?: ChatMessage;
  selectedModel?: string;
  composerMode: 'chat' | 'task';
  inputText: string;
  setInputText: (value: string) => void;
  attachedFiles: File[];
  onRemoveFile: (index: number) => void;
  isPlusMenuOpen: boolean;
  setIsPlusMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  onTriggerFileInput: () => void;
  onOpenSkills: () => void;
  onOpenPlugins: () => void;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  onKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onSelectModel: (modelId: string, effort?: string, accountId?: string) => void;
  isSubmitting: boolean;
  onSend: () => void;
  onSteer: () => void;
  onStopClick: (runId: string) => void;
}

export const ConversationDockedComposer: React.FC<ConversationDockedComposerProps> = ({
  mode,
  session,
  liveMessage,
  selectedModel,
  composerMode,
  inputText,
  setInputText,
  attachedFiles,
  onRemoveFile,
  isPlusMenuOpen,
  setIsPlusMenuOpen,
  onTriggerFileInput,
  onOpenSkills,
  onOpenPlugins,
  textareaRef,
  onKeyDown,
  onSelectModel,
  isSubmitting,
  onSend,
  onSteer,
  onStopClick,
}) => (
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
                onClick={() => onRemoveFile(idx)}
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
            onAddFilesOrPhotos={onTriggerFileInput}
            onOpenSkills={onOpenSkills}
            onAddPlugins={onOpenPlugins}
            position="top"
          />
        </div>

        <textarea
          ref={textareaRef}
          rows={1}
          value={inputText}
          onChange={(e) => setInputText(e.target.value)}
          onKeyDown={onKeyDown}
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
            onSelectModel={onSelectModel}
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
                  onSteer();
                } else if (liveMessage.runId) {
                  onStopClick(liveMessage.runId);
                }
              } else {
                onSend();
              }
            }}
            disabled={isSubmitting || (!liveMessage && !inputText.trim() && attachedFiles.length === 0)}
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
);
