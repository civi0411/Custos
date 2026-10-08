import React, { useMemo } from 'react';
import { FileDiff, Check, FileCode, Copy, Sparkles } from 'lucide-react';
import { Highlight, themes } from 'prism-react-renderer';
import { Session } from '@/types';
import { Tooltip } from '@/components/Tooltip';

interface DiffSectionProps {
  session: Session | null;
  onAcceptAndRun: () => void;
  onRejectDiff: () => void;
  onCopyDiff: () => void;
}

const getLanguageFromFileName = (fileName: string): string => {
  const ext = fileName.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'ts':
    case 'tsx':
      return 'typescript';
    case 'js':
    case 'jsx':
      return 'javascript';
    case 'rs':
      return 'rust';
    case 'py':
      return 'python';
    case 'json':
      return 'json';
    case 'css':
      return 'css';
    case 'html':
      return 'html';
    case 'go':
      return 'go';
    case 'md':
      return 'markdown';
    default:
      return 'typescript';
  }
};

const parseHunk = (hunk: string) => {
  const match = hunk.match(/@@\s*-(\d+)(?:,\d+)?\s*\+(\d+)(?:,\d+)?\s*@@/);
  if (match) {
    return {
      oldStart: parseInt(match[1], 10),
      newStart: parseInt(match[2], 10),
    };
  }
  return { oldStart: 1, newStart: 1 };
};

export const DiffSection: React.FC<DiffSectionProps> = ({
  session,
  onAcceptAndRun,
  onRejectDiff,
  onCopyDiff
}) => {
  const { computedLines, fullCodeString, language } = useMemo(() => {
    if (!session) {
      return { computedLines: [], fullCodeString: '', language: 'typescript' };
    }

    const { oldStart, newStart } = parseHunk(session.diffHunk);
    let oldLine = oldStart;
    let newLine = newStart;

    const lines = session.diffCode.map((item) => {
      // Strip only leading diff indicator (+, -, or space) to preserve original code indentation
      const cleanText =
        item.text.startsWith('+') || item.text.startsWith('-') || item.text.startsWith(' ')
          ? item.text.slice(1)
          : item.text;

      let oldNum: number | string = '';
      let newNum: number | string = '';
      let sign = ' ';

      if (item.type === 'del') {
        oldNum = oldLine++;
        sign = '-';
      } else if (item.type === 'add') {
        newNum = newLine++;
        sign = '+';
      } else {
        oldNum = oldLine++;
        newNum = newLine++;
        sign = ' ';
      }

      return {
        type: item.type,
        cleanText,
        oldNum,
        newNum,
        sign,
      };
    });

    const fullCode = lines.map((l) => l.cleanText).join('\n');
    const lang = getLanguageFromFileName(session.fileName);

    return { computedLines: lines, fullCodeString: fullCode, language: lang };
  }, [session]);

  if (!session) {
    return (
      <section className="flex-1 flex flex-col items-center justify-center bg-canvas text-fg-subtle text-xs">
        <Sparkles className="w-8 h-8 mb-2 opacity-40 workbench-accent" />
        <p>No diff selected</p>
      </section>
    );
  }

  return (
    <section className="flex-1 flex flex-col bg-canvas overflow-hidden min-w-0 transition-all duration-150 select-text">
      {/* Changes Header & File Tabs */}
      <div className="h-11 border-b border-border-muted px-3 sm:px-4 flex items-center justify-between bg-surface-1 text-xs shrink-0 min-w-0">
        <div className="flex items-center gap-2 min-w-0 truncate">
          <span className="text-xs font-semibold text-fg-editor flex items-center gap-1.5 truncate">
            <FileDiff className="w-3.5 h-3.5 shrink-0" />
            <span className="truncate">Code Changes</span>
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 font-mono shrink-0">
            {session.diffLinesCount}
          </span>
        </div>

        {/* Action Buttons: Accept & Run, Reject */}
        <div className="flex items-center gap-1.5 sm:gap-2 shrink-0">
          <button 
            onClick={onAcceptAndRun} 
            className="px-2 sm:px-2.5 py-0.5 sm:py-1 bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-500 border border-emerald-500/30 rounded-md text-[10px] font-medium transition flex items-center gap-1"
          >
            <Check className="w-2.5 h-2.5 sm:w-3 sm:h-3" />
            <span>Accept & Run</span>
          </button>
          <button 
            onClick={onRejectDiff} 
            className="px-2 py-0.5 sm:py-1 bg-surface-2 hover:bg-surface-3 text-fg-muted hover:text-fg-editor border border-border-default rounded-md text-[10px] font-medium transition hidden sm:inline-block"
          >
            Reject
          </button>
        </div>
      </div>

      {/* File Tab Selector Bar */}
      <div className="h-9 border-b border-border-muted px-3 flex items-center gap-2 bg-surface-1 text-xs overflow-x-auto shrink-0 font-mono min-w-0">
        <div className="px-2.5 py-1 bg-surface-2 border-t-2 border-[var(--workbench-accent)] text-fg-editor rounded-t flex items-center gap-1.5 text-[11px] truncate shrink-0">
          <FileCode className="w-3 h-3 text-fg-muted shrink-0" />
          <span className="truncate">{session.fileName}</span>
          <span className="w-1.5 h-1.5 rounded-full bg-border-default shrink-0"></span>
        </div>
        <span className="text-fg-subtle text-[10px] shrink-0">Modified file</span>
      </div>

      {/* Code Diff Viewer Body */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden p-3 sm:p-5 font-mono text-[11px] leading-5 text-fg-editor bg-canvas min-w-0 select-text">
        <div className="max-w-5xl w-full mx-auto space-y-4">
          <div className="border border-border-default rounded-xl overflow-hidden bg-surface-1 shadow-inner min-w-0 w-full">
            {/* Header Bar */}
            <div className="relative z-20 px-3 py-1.5 bg-surface-2 border-b border-border-default flex items-center justify-between text-fg-muted text-[10px] shrink-0 select-none">
              <div className="flex items-center gap-2 min-w-0">
                <span className="truncate font-mono text-fg-editor">{session.diffHunk}</span>
                <span className="text-[9px] uppercase px-1.5 py-0.5 rounded bg-surface-3 font-mono text-fg-muted border border-border-default shrink-0">
                  {language}
                </span>
              </div>
              <Tooltip content="Copy diff to clipboard" position="bottom" align="end">
                <button 
                  onClick={onCopyDiff} 
                  className="hover:text-fg-editor flex items-center gap-1 shrink-0 ml-2 cursor-pointer transition text-fg-muted"
                >
                  <Copy className="w-3 h-3" />
                  <span className="hidden sm:inline">Copy Diff</span>
                </button>
              </Tooltip>
            </div>
            
            {/* Syntax Highlighted Diff Lines with Gutter Line Numbers */}
            <Highlight theme={themes.vsDark} code={fullCodeString} language={language}>
              {({ tokens, getTokenProps }) => (
                <div className="py-2 font-mono text-[11px] leading-relaxed select-text w-full overflow-x-hidden">
                  {computedLines.map((line, idx) => {
                    const isAdd = line.type === 'add';
                    const isDel = line.type === 'del';

                    const rowBg = isAdd
                      ? 'bg-emerald-500/10 hover:bg-emerald-500/15'
                      : isDel
                      ? 'bg-rose-500/10 hover:bg-rose-500/15'
                      : 'hover:bg-surface-2/40';

                    const signColor = isAdd
                      ? 'text-emerald-500 font-bold'
                      : isDel
                      ? 'text-rose-500 font-bold'
                      : 'text-transparent';

                    const lineTokens = tokens[idx] || [];

                    return (
                      <div
                        key={idx}
                        className={`flex items-stretch w-full min-w-0 transition-colors duration-100 ${rowBg}`}
                      >
                        {/* Gutter: Old Line Number */}
                        <span className="w-9 sm:w-10 text-right pr-2 text-fg-subtle select-none shrink-0 font-mono text-[10.5px] tabular-nums py-0.5">
                          {line.oldNum}
                        </span>

                        {/* Gutter: New Line Number */}
                        <span className="w-9 sm:w-10 text-right pr-2 text-fg-subtle select-none shrink-0 font-mono text-[10.5px] tabular-nums py-0.5 border-r border-border-muted">
                          {line.newNum}
                        </span>

                        {/* Sign indicator (+ / - / space) */}
                        <span className={`w-5 text-center select-none shrink-0 font-mono text-xs py-0.5 ${signColor}`}>
                          {line.sign}
                        </span>

                        {/* Code Content highlighted by prism-react-renderer */}
                        <div className="flex-1 px-2 py-0.5 whitespace-pre-wrap break-words [overflow-wrap:anywhere] [word-break:break-word] font-mono select-text text-fg-editor min-w-0">
                          {lineTokens.length > 0 ? (
                            lineTokens.map((token, key) => (
                              <span key={key} {...getTokenProps({ token })} />
                            ))
                          ) : (
                            <span>{line.cleanText || ' '}</span>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </Highlight>
          </div>

          {/* Diff Summary / AST inspection */}
          <div className="p-3 rounded-xl border border-border-muted bg-surface-1 text-xs select-text">
            <div className="flex items-center justify-between text-fg-editor font-sans mb-1.5 select-none">
              <span className="font-medium">Diff Analysis & Inspection</span>
              <span className="text-[10px] text-emerald-500 font-mono">Clean AST Patch</span>
            </div>
            <p className="text-fg-muted text-[11px] leading-relaxed font-sans select-text whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
              {session.summary}
            </p>
          </div>
        </div>
      </div>
    </section>
  );
};
