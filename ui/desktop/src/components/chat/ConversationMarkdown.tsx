import React from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import 'katex/dist/katex.min.css';
import { HighlightedCodeBlock } from './HighlightedCodeBlock';
import { remarkNormalizeChatMath } from './markdownMath';

interface ConversationMarkdownProps {
  text: string;
}

export const ConversationMarkdown = React.memo(function ConversationMarkdown({ text }: ConversationMarkdownProps) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm, remarkMath, remarkNormalizeChatMath]}
      rehypePlugins={[[rehypeKatex, { trust: false, throwOnError: false, strict: false, maxExpand: 1000, maxSize: 20 }]]}
      components={{
        pre({ children }) {
          const child = React.Children.toArray(children)[0];
          if (!React.isValidElement<{ className?: string; children?: React.ReactNode }>(child)) {
            return <pre>{children}</pre>;
          }
          const language = /(?:^|\s)language-([^\s]+)/.exec(child.props.className || '')?.[1];
          return <HighlightedCodeBlock code={String(child.props.children ?? '').replace(/\n$/, '')} language={language} />;
        },
        code({ children, className }) {
          return <code className={className || 'px-1.5 py-0.5 rounded bg-surface-1 text-fg-editor border border-border-muted font-mono text-[12px]'}>{children}</code>;
        }
      }}
    >
      {text}
    </ReactMarkdown>
  );
});
