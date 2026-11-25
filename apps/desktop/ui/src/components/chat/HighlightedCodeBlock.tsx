import { memo, useMemo } from 'react';
import hljs from 'highlight.js';

export const HighlightedCodeBlock = memo(function HighlightedCodeBlock({
  code,
  language,
}: { code: string; language?: string }) {
  const highlighted = useMemo(() => {
    // Bound synchronous work while a streamed response is growing.
    if (code.length > 100_000 || /^(text|txt|plaintext|plain|nohighlight)$/i.test(language || '')) return null;
    try {
      if (language) {
        return hljs.getLanguage(language)
          ? hljs.highlight(code, { language, ignoreIllegals: true })
          : null;
      }
      return hljs.highlightAuto(code, ['javascript', 'typescript', 'python', 'rust', 'go', 'json', 'bash', 'sql', 'xml', 'css']);
    } catch {
      return null;
    }
  }, [code, language]);

  return (
    <div className="my-3 min-w-0 rounded-xl overflow-hidden bg-surface-0 border border-border-default font-mono text-[12px]">
      <div className="px-3 py-1.5 bg-surface-1 text-[10px] text-fg-muted">
        {language || highlighted?.language || 'text'}
      </div>
      <pre className="p-3.5 overflow-x-auto text-fg-editor">
        {highlighted
          ? <code className="chat-highlight hljs" dangerouslySetInnerHTML={{ __html: highlighted.value }} />
          : <code>{code}</code>}
      </pre>
    </div>
  );
});
