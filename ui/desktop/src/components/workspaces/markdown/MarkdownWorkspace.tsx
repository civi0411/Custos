import React, { useState } from 'react';
import { 
  FileText, 
  Bold, 
  Italic, 
  Code, 
  List, 
  CheckSquare, 
  Heading1, 
  Heading2, 
  Download 
} from 'lucide-react';

const DEFAULT_MARKDOWN = `# Custos SADE Architecture & Orchestration Guide

Welcome to the Custos **Self-Adapting Development Environment (SADE)** document scratchpad.

## 1. Core Principles
- **Single Task Kernel**: Unified contract between Coding, Research, and Assistant packs.
- **Durable Orchestration**: Transactional run convergence, mailbox delivery, and fencing.
- **Zero-IO Invariant**: Proof closure ensuring zero unintended I/O mutations.

### Key Milestones
- [x] Integrate Orca-class living tab strip with \`+\` popover launcher
- [x] Multi-pane dynamic layout (Single, Split 2, Split 3, 2x2 Grid, Master+4)
- [ ] Cross-pack criterion verification & model cascade

\`\`\`rust
pub struct WorkflowDispatcher {
    permit_gate: Arc<PermitGate>,
    max_inflight_actions: usize,
}
\`\`\`

> Note: Use keyboard shortcut \`⌘S\` to export this document.
`;

export const MarkdownWorkspace: React.FC = () => {
  const [content, setContent] = useState(DEFAULT_MARKDOWN);
  const [viewMode, setViewMode] = useState<'split' | 'edit' | 'preview'>('split');
  const [isCopied, setIsCopied] = useState(false);

  const handleInsert = (before: string, after: string = '') => {
    setContent((prev) => prev + `\n${before}New Entry${after}`);
  };

  const handleExport = () => {
    const blob = new Blob([content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'custos-note.md';
    a.click();
    URL.revokeObjectURL(url);
    setIsCopied(true);
    setTimeout(() => setIsCopied(false), 2000);
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#0c0e14] text-neutral-200 select-none font-sans overflow-hidden">
      {/* 1. Markdown Toolbar */}
      <div className="h-10 px-3 bg-[#090b10] border-b border-[#1c2130] flex items-center justify-between shrink-0">
        <div className="flex items-center gap-1">
          <div className="flex items-center gap-1.5 px-2 py-1 rounded bg-[#141824] border border-[#232938] text-xs font-mono text-neutral-300 mr-2">
            <FileText className="w-3.5 h-3.5 text-brand-blue" />
            <span>custos-note.md</span>
          </div>

          <div className="flex items-center gap-0.5 text-neutral-400">
            <button
              onClick={() => handleInsert('# ')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Heading 1"
            >
              <Heading1 className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => handleInsert('## ')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Heading 2"
            >
              <Heading2 className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => handleInsert('**', '**')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Bold"
            >
              <Bold className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => handleInsert('*', '*')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Italic"
            >
              <Italic className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => handleInsert('```rust\n', '\n```')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Code Block"
            >
              <Code className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => handleInsert('- ')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Bullet List"
            >
              <List className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => handleInsert('- [ ] ')}
              className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition"
              title="Checklist Task"
            >
              <CheckSquare className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* View Mode Switcher & Export */}
        <div className="flex items-center gap-1.5">
          <div className="flex items-center bg-[#141824] border border-[#232938] rounded-lg p-0.5 text-xs">
            <button
              onClick={() => setViewMode('split')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition ${viewMode === 'split' ? 'bg-[#1e2436] text-white' : 'text-neutral-400 hover:text-white'}`}
            >
              Split
            </button>
            <button
              onClick={() => setViewMode('edit')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition ${viewMode === 'edit' ? 'bg-[#1e2436] text-white' : 'text-neutral-400 hover:text-white'}`}
            >
              Source
            </button>
            <button
              onClick={() => setViewMode('preview')}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition ${viewMode === 'preview' ? 'bg-[#1e2436] text-white' : 'text-neutral-400 hover:text-white'}`}
            >
              Preview
            </button>
          </div>

          <button
            onClick={handleExport}
            className="flex items-center gap-1 px-2.5 py-1 rounded-lg bg-[#141824] hover:bg-[#1a2030] border border-[#232938] text-xs text-neutral-300 hover:text-white transition"
            title="Download Markdown"
          >
            <Download className="w-3.5 h-3.5 text-brand-blue" />
            <span>{isCopied ? 'Saved!' : 'Export'}</span>
          </button>
        </div>
      </div>

      {/* 2. Editor & Preview Area */}
      <div className="flex-1 flex overflow-hidden min-h-0">
        {/* Source Textarea */}
        {(viewMode === 'split' || viewMode === 'edit') && (
          <div className="flex-1 flex flex-col min-w-0 border-r border-[#1c2130] bg-[#090b10]">
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              className="flex-1 w-full bg-transparent p-4 font-mono text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none resize-none leading-relaxed select-text"
              placeholder="Write Markdown here..."
            />
          </div>
        )}

        {/* Formatted Preview */}
        {(viewMode === 'split' || viewMode === 'preview') && (
          <div className="flex-1 overflow-y-auto p-6 bg-[#0c0e14] select-text">
            <div className="max-w-2xl mx-auto space-y-4 text-xs leading-relaxed text-neutral-300">
              {content.split('\n\n').map((block, idx) => {
                const trimmed = block.trim();
                if (trimmed.startsWith('# ')) {
                  return <h1 key={idx} className="text-xl font-bold text-white border-b border-[#232938] pb-2">{trimmed.slice(2)}</h1>;
                }
                if (trimmed.startsWith('## ')) {
                  return <h2 key={idx} className="text-base font-semibold text-white mt-4 border-b border-[#232938]/60 pb-1">{trimmed.slice(3)}</h2>;
                }
                if (trimmed.startsWith('### ')) {
                  return <h3 key={idx} className="text-sm font-semibold text-neutral-200 mt-2">{trimmed.slice(4)}</h3>;
                }
                if (trimmed.startsWith('```')) {
                  const lines = trimmed.split('\n');
                  const code = lines.slice(1, -1).join('\n');
                  return (
                    <pre key={idx} className="p-3 rounded-lg bg-[#141824] border border-[#232938] font-mono text-[11px] text-emerald-400 overflow-x-auto">
                      <code>{code}</code>
                    </pre>
                  );
                }
                if (trimmed.startsWith('- [ ]') || trimmed.startsWith('- [x]')) {
                  return (
                    <ul key={idx} className="space-y-1 my-2">
                      {trimmed.split('\n').map((li, i) => {
                        const isDone = li.includes('[x]');
                        return (
                          <li key={i} className="flex items-center gap-2 text-neutral-300">
                            <input type="checkbox" checked={isDone} readOnly className="rounded border-[#232938] accent-brand-blue" />
                            <span className={isDone ? 'line-through text-neutral-500' : ''}>{li.replace(/- \[[ x]\] /, '')}</span>
                          </li>
                        );
                      })}
                    </ul>
                  );
                }
                if (trimmed.startsWith('> ')) {
                  return (
                    <blockquote key={idx} className="border-l-2 border-brand-blue pl-3 italic text-neutral-400 bg-brand-blue/5 py-1 rounded-r">
                      {trimmed.slice(2)}
                    </blockquote>
                  );
                }
                return <p key={idx} className="text-neutral-300 leading-normal">{trimmed}</p>;
              })}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default MarkdownWorkspace;
