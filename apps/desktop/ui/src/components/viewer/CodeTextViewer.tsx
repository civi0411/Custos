import React, { useState, useMemo } from 'react';
import { Copy, Check, WrapText } from 'lucide-react';

interface CodeTextViewerProps {
  content: string;
  path?: string;
  isReadOnly?: boolean;
  onChangeText?: (text: string) => void;
  className?: string;
  language?: string;
}

export const CodeTextViewer: React.FC<CodeTextViewerProps> = ({
  content,
  isReadOnly = false,
  onChangeText,
  className = '',
  language,
}) => {
  const [wordWrap, setWordWrap] = useState<boolean>(false);
  const [copied, setCopied] = useState<boolean>(false);
  const lineCount = useMemo(() => (content ? content.split('\n').length : 1), [content]);

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex items-center justify-between px-4 py-1.5 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          {language && (
            <span className="uppercase text-[10px] font-mono font-semibold tracking-wider px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-subtle)]">
              {language}
            </span>
          )}
          <span className="text-[11px] font-mono text-[var(--color-fg-subtle)]">
            {lineCount.toLocaleString()} lines • {content.length.toLocaleString()} chars
          </span>
        </div>

        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => setWordWrap(!wordWrap)}
            className={`flex items-center gap-1 px-2 py-1 rounded text-xs transition ${
              wordWrap
                ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
            }`}
            title={wordWrap ? 'Disable Word Wrap' : 'Enable Word Wrap'}
          >
            <WrapText className="h-3 w-3" />
            <span>Wrap</span>
          </button>

          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-1 rounded text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Copy code"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </div>

      {/* Code Text Area with Line Numbers */}
      <div className="flex-1 relative overflow-hidden flex">
        {/* Line Numbers Gutter */}
        <div className="select-none bg-[var(--color-surface-1)]/40 border-r border-[var(--color-border-subtle)]/60 py-4 px-2 text-right font-mono text-xs text-[var(--color-fg-subtle)] opacity-60 overflow-hidden shrink-0 min-w-[42px]">
          {Array.from({ length: Math.min(lineCount, 5000) }).map((_, i) => (
            <div key={i} className="leading-relaxed">
              {i + 1}
            </div>
          ))}
        </div>

        {/* Editor / Viewer textarea */}
        <textarea
          value={content}
          readOnly={isReadOnly}
          onChange={(e) => onChangeText?.(e.target.value)}
          wrap={wordWrap ? 'soft' : 'off'}
          spellCheck={false}
          className={`flex-1 w-full h-full resize-none bg-[var(--color-canvas)] p-4 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] focus:outline-none selection:bg-zinc-800 ${
            wordWrap ? 'whitespace-pre-wrap' : 'whitespace-pre overflow-x-auto'
          }`}
        />
      </div>
    </div>
  );
};
