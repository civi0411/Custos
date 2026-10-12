interface MathNode {
  type: string;
  value?: string;
  children?: MathNode[];
  position?: { start: { offset?: number }; end: { offset?: number } };
  data?: { hProperties?: Record<string, unknown>; [key: string]: unknown };
}

export function remarkNormalizeChatMath() {
  return (tree: MathNode, file: { value: unknown }) => {
    const source = String(file.value);
    const visit = (node: MathNode) => {
      if (node.type === 'math' || node.type === 'inlineMath') {
        // Model Markdown sometimes escapes subscripts; preserve literal text macros.
        node.value = node.value?.replace(
          /\\(?:text|textrm|textsf|texttt|operatorname)\{(?:[^{}]|\{[^{}]*\})*\}|\\_(?=\{|[a-zA-Z0-9])/g,
          token => token === '\\_' ? '_' : token,
        );
        if (node.data) {
          const text = { type: 'text', value: node.value || '' };
          node.data.hChildren = node.type === 'math'
            ? [{ type: 'element', tagName: 'code', properties: { className: ['language-math', 'math-display'] }, children: [text] }]
            : [text];
        }
        const start = node.position?.start.offset;
        const end = node.position?.end.offset;
        if (node.type === 'inlineMath' && start !== undefined && end !== undefined &&
          source.slice(start, end).startsWith('$$')) {
          node.data = {
            ...node.data,
            hProperties: { ...node.data?.hProperties, className: ['language-math', 'math-display'] },
          };
        }
      }
      node.children?.forEach(visit);
    };
    visit(tree);
  };
}
