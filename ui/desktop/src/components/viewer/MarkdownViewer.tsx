import React, { useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import 'katex/dist/katex.min.css';
import { remarkNormalizeChatMath } from '@/components/chat/markdownMath';
import { HighlightedCodeBlock } from '@/components/chat/HighlightedCodeBlock';
import { Eye, Code, Copy, Check } from 'lucide-react';

interface MarkdownViewerProps {
  content: string;
  isReadOnly?: boolean;
  onChangeText?: (text: string) => void;
  className?: string;
  embedded?: boolean;
}

export const MarkdownViewer: React.FC<MarkdownViewerProps> = ({
  content,
  isReadOnly = true,
  onChangeText,
  className = '',
  embedded = false,
}) => {
  const [viewMode, setViewMode] = useState<'preview' | 'raw'>('preview');
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Viewer Sub-Header */}
      {!embedded && <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-1.5 bg-[var(--color-surface-2)] p-0.5 rounded-lg border border-[var(--color-border-subtle)]">
          <button
            type="button"
            onClick={() => setViewMode('preview')}
            className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
              viewMode === 'preview'
                ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
            }`}
          >
            <Eye className="h-3 w-3" />
            <span>Rendered Preview</span>
          </button>
          <button
            type="button"
            onClick={() => setViewMode('raw')}
            className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
              viewMode === 'raw'
                ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
            }`}
          >
            <Code className="h-3 w-3" />
            <span>Raw Source</span>
          </button>
        </div>

        <button
          type="button"
          onClick={handleCopy}
          className="flex items-center gap-1 px-2 py-1 rounded text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
          title="Copy markdown content"
        >
          {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
          <span>{copied ? 'Copied' : 'Copy'}</span>
        </button>
      </div>}

      {/* Content Body */}
      <div className="flex-1 overflow-auto">
        {viewMode === 'preview' ? (
          <div className={`max-w-4xl mx-auto ${embedded ? 'p-3' : 'p-6 md:p-8'} markdown-body conversation-markdown text-sm leading-relaxed text-[var(--color-editor-fg)] select-text`}>
            <ReactMarkdown
              remarkPlugins={[remarkGfm, remarkMath, remarkNormalizeChatMath]}
              rehypePlugins={[[rehypeKatex, { trust: false, throwOnError: false, strict: false, maxExpand: 1000, maxSize: 20 }]]}
              components={{
                pre({ children }) {
                  const child = React.Children.toArray(children)[0];
                  if (!React.isValidElement<{ className?: string; children?: React.ReactNode }>(child)) {
                    return <pre className="p-3 bg-[var(--color-surface-2)] rounded-lg font-mono text-xs overflow-x-auto my-3">{children}</pre>;
                  }
                  const language = /(?:^|\s)language-([^\s]+)/.exec(child.props.className || '')?.[1];
                  const codeString = String(child.props.children ?? '').replace(/\n$/, '');
                  return <HighlightedCodeBlock code={codeString} language={language} />;
                },
                code({ children, className }) {
                  return (
                    <code className={className || 'px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] font-mono text-xs'}>
                      {children}
                    </code>
                  );
                },
                table({ children }) {
                  return (
                    <div className="my-4 overflow-x-auto rounded-lg border border-[var(--color-border-subtle)]">
                      <table className="min-w-full divide-y divide-[var(--color-border-subtle)] text-xs">
                        {children}
                      </table>
                    </div>
                  );
                },
                thead({ children }) {
                  return <thead className="bg-[var(--color-surface-2)] font-semibold text-[var(--color-editor-fg)]">{children}</thead>;
                },
                th({ children }) {
                  return <th className="px-3 py-2 text-left text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">{children}</th>;
                },
                td({ children }) {
                  return <td className="px-3 py-2 border-t border-[var(--color-border-subtle)] text-xs text-[var(--color-editor-fg)]">{children}</td>;
                },
                blockquote({ children }) {
                  return (
                    <blockquote className="my-3 border-l-4 border-blue-500/60 bg-blue-500/5 px-4 py-2 text-xs italic text-[var(--color-fg-muted)] rounded-r-md">
                      {children}
                    </blockquote>
                  );
                },
                h1({ children }) {
                  return <h1 className="text-xl font-bold tracking-tight text-[var(--color-editor-fg)] pb-2 mb-4 border-b border-[var(--color-border-subtle)]">{children}</h1>;
                },
                h2({ children }) {
                  return <h2 className="text-base font-semibold text-[var(--color-editor-fg)] mt-6 mb-3 pb-1 border-b border-[var(--color-border-subtle)]/50">{children}</h2>;
                },
                h3({ children }) {
                  return <h3 className="text-sm font-semibold text-[var(--color-editor-fg)] mt-4 mb-2">{children}</h3>;
                },
                ul({ children }) {
                  return <ul className="list-disc pl-5 my-2 space-y-1 text-xs">{children}</ul>;
                },
                ol({ children }) {
                  return <ol className="list-decimal pl-5 my-2 space-y-1 text-xs">{children}</ol>;
                },
              }}
            >
              {content || '_Empty document_'}
            </ReactMarkdown>
          </div>
        ) : (
          <div className="h-full flex flex-col">
            <textarea
              value={content}
              readOnly={isReadOnly}
              onChange={(e) => onChangeText?.(e.target.value)}
              spellCheck={false}
              className="flex-1 w-full resize-none bg-[var(--color-canvas)] p-5 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] focus:outline-none selection:bg-zinc-800"
            />
          </div>
        )}
      </div>
    </div>
  );
};
