import React from 'react';
import { FileDiff, Check, FileCode, Copy, Sparkles } from 'lucide-react';
import { Session } from '../types';

interface DiffSectionProps {
  session: Session | null;
  onAcceptAndRun: () => void;
  onRejectDiff: () => void;
  onCopyDiff: () => void;
}

export const DiffSection: React.FC<DiffSectionProps> = ({
  session,
  onAcceptAndRun,
  onRejectDiff,
  onCopyDiff
}) => {
  if (!session) {
    return (
      <section className="flex-1 flex flex-col items-center justify-center bg-surface text-neutral-500 text-xs">
        <Sparkles className="w-8 h-8 mb-2 opacity-40 text-brand-cyan" />
        <p>No diff selected</p>
      </section>
    );
  }

  return (
    <section className="flex-1 flex flex-col bg-surface overflow-hidden min-w-0 transition-all duration-150">
      {/* Changes Header & File Tabs */}
      <div className="h-11 border-b border-surface-border px-3 sm:px-4 flex items-center justify-between bg-surface-card/60 text-xs shrink-0 min-w-0">
        <div className="flex items-center gap-2 min-w-0 truncate">
          <span className="text-xs font-semibold text-neutral-300 flex items-center gap-1.5 truncate">
            <FileDiff className="w-3.5 h-3.5 text-brand-cyan shrink-0" />
            <span className="truncate">Code Changes</span>
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono shrink-0">
            {session.diffLinesCount}
          </span>
        </div>

        {/* Action Buttons: Accept & Run, Reject */}
        <div className="flex items-center gap-1.5 sm:gap-2 shrink-0">
          <button 
            onClick={onAcceptAndRun} 
            className="px-2 sm:px-2.5 py-1 bg-emerald-600/20 hover:bg-emerald-600/30 text-emerald-400 border border-emerald-500/30 rounded-md text-[11px] font-medium transition flex items-center gap-1"
          >
            <Check className="w-3 h-3" />
            <span>Accept & Run</span>
          </button>
          <button 
            onClick={onRejectDiff} 
            className="px-2 py-1 bg-surface-elevated hover:bg-surface-hover text-neutral-400 rounded-md text-[11px] transition hidden sm:inline-block"
          >
            Reject
          </button>
        </div>
      </div>

      {/* File Tab Selector Bar */}
      <div className="h-9 border-b border-surface-border px-3 flex items-center gap-2 bg-surface text-xs overflow-x-auto shrink-0 font-mono min-w-0">
        <div className="px-2.5 py-1 bg-surface-card border-t-2 border-brand-blue text-white rounded-t flex items-center gap-1.5 text-[11px] truncate shrink-0">
          <FileCode className="w-3 h-3 text-neutral-400 shrink-0" />
          <span className="truncate">{session.fileName}</span>
          <span className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0"></span>
        </div>
        <span className="text-neutral-500 text-[10px] shrink-0">Modified file</span>
      </div>

      {/* Code Diff Viewer Body */}
      <div className="flex-1 overflow-y-auto p-3 sm:p-4 font-mono text-[11px] leading-5 text-neutral-300 bg-black/60 min-w-0">
        <div className="border border-surface-border rounded-xl overflow-hidden bg-black shadow-inner min-w-0">
          <div className="px-3 py-1.5 bg-surface-card border-b border-surface-border flex items-center justify-between text-neutral-400 text-[10px] shrink-0">
            <span className="truncate font-mono">{session.diffHunk}</span>
            <button onClick={onCopyDiff} className="hover:text-white flex items-center gap-1 shrink-0 ml-2">
              <Copy className="w-3 h-3" />
              <span className="hidden sm:inline">Copy Diff</span>
            </button>
          </div>
          
          <pre className="p-3 overflow-x-auto text-[11px] space-y-0.5 leading-relaxed font-mono">
            {session.diffCode.map((line, idx) => {
              if (line.type === 'add') {
                return (
                  <div key={idx} className="text-emerald-400 bg-emerald-500/10 px-1 rounded">
                    {line.text}
                  </div>
                );
              }
              if (line.type === 'del') {
                return (
                  <div key={idx} className="text-red-400 bg-red-500/10 px-1 rounded line-through opacity-80">
                    {line.text}
                  </div>
                );
              }
              return (
                <div key={idx} className="text-neutral-400 px-1">
                  {line.text}
                </div>
              );
            })}
          </pre>
        </div>

        {/* Diff Summary / AST inspection */}
        <div className="mt-4 p-3 rounded-xl border border-surface-border bg-surface-card text-xs">
          <div className="flex items-center justify-between text-neutral-300 font-sans mb-1.5">
            <span className="font-medium">Diff Analysis & Inspection</span>
            <span className="text-[10px] text-emerald-400 font-mono">Clean AST Patch</span>
          </div>
          <p className="text-neutral-400 text-[11px] leading-relaxed font-sans">
            {session.summary}
          </p>
        </div>
      </div>
    </section>
  );
};
