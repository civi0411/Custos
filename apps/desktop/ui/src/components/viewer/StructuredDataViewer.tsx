import React, { useState, useMemo } from 'react';
import { ChevronRight, ChevronDown, Copy, Check, Search, Braces, FileText } from 'lucide-react';

interface StructuredDataViewerProps {
  content: string;
  isReadOnly?: boolean;
  onChangeText?: (text: string) => void;
  className?: string;
  format?: 'json' | 'yaml' | 'toml';
}

interface TreeNodeProps {
  name: string;
  value: any;
  path: string;
  depth: number;
  searchFilter: string;
}

const TreeNode: React.FC<TreeNodeProps> = ({ name, value, path, depth, searchFilter }) => {
  const isObject = value !== null && typeof value === 'object';
  const isArray = Array.isArray(value);
  const [expanded, setExpanded] = useState<boolean>(depth < 2);
  const [copiedPath, setCopiedPath] = useState(false);

  const keys = useMemo(() => {
    if (!isObject) return [];
    return Object.keys(value);
  }, [isObject, value]);

  const matchesFilter = useMemo(() => {
    if (!searchFilter.trim()) return true;
    const q = searchFilter.toLowerCase();
    if (name.toLowerCase().includes(q) || path.toLowerCase().includes(q)) return true;
    if (!isObject && String(value).toLowerCase().includes(q)) return true;
    return false;
  }, [searchFilter, name, path, isObject, value]);

  const copyPath = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(path);
    setCopiedPath(true);
    setTimeout(() => setCopiedPath(false), 1200);
  };

  const renderValue = () => {
    if (value === null) return <span className="text-zinc-500 italic">null</span>;
    if (typeof value === 'boolean') return <span className="text-purple-400 font-semibold">{String(value)}</span>;
    if (typeof value === 'number') return <span className="text-amber-400 font-mono">{value}</span>;
    if (typeof value === 'string') return <span className="text-emerald-400 select-text">&quot;{value}&quot;</span>;
    return null;
  };

  if (!matchesFilter && !isObject) return null;

  return (
    <div className="font-mono text-xs select-none">
      <div
        onClick={() => isObject && setExpanded(!expanded)}
        className={`flex items-center gap-1.5 py-0.5 px-2 rounded hover:bg-[var(--color-surface-2)] transition group cursor-pointer ${
          depth > 0 ? 'ml-4' : ''
        }`}
      >
        {isObject ? (
          <span className="text-[var(--color-fg-muted)] group-hover:text-[var(--color-editor-fg)]">
            {expanded ? <ChevronDown className="h-3 w-3 shrink-0" /> : <ChevronRight className="h-3 w-3 shrink-0" />}
          </span>
        ) : (
          <span className="w-3" />
        )}

        <span className="font-semibold text-sky-400 shrink-0">{name}</span>
        <span className="text-[var(--color-fg-subtle)]">:</span>

        {isObject ? (
          <span className="text-[var(--color-fg-muted)] text-[11px]">
            {isArray ? `Array(${keys.length})` : `Object{${keys.length}}`}
          </span>
        ) : (
          <span className="truncate flex-1">{renderValue()}</span>
        )}

        {/* Action button to copy JSON path */}
        <button
          type="button"
          onClick={copyPath}
          title={`Copy path: ${path}`}
          className="opacity-0 group-hover:opacity-100 ml-auto px-1.5 py-0.2 rounded text-[10px] text-[var(--color-fg-muted)] hover:text-white bg-[var(--color-surface-3)] transition"
        >
          {copiedPath ? 'Copied' : path}
        </button>
      </div>

      {isObject && expanded && (
        <div className="border-l border-[var(--color-border-subtle)]/40 ml-3.5 my-0.5">
          {keys.map((k) => (
            <TreeNode
              key={k}
              name={k}
              value={value[k]}
              path={isArray ? `${path}[${k}]` : `${path}.${k}`}
              depth={depth + 1}
              searchFilter={searchFilter}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const StructuredDataViewer: React.FC<StructuredDataViewerProps> = ({
  content,
  isReadOnly = true,
  onChangeText,
  className = '',
}) => {
  const [viewMode, setViewMode] = useState<'tree' | 'formatted'>('tree');
  const [searchFilter, setSearchFilter] = useState('');
  const [copied, setCopied] = useState(false);

  const { parsedJson, parseError } = useMemo(() => {
    try {
      const parsed = JSON.parse(content);
      return { parsedJson: parsed, parseError: null };
    } catch (err: any) {
      return { parsedJson: null, parseError: err?.message || 'Invalid JSON format' };
    }
  }, [content]);

  const formattedContent = useMemo(() => {
    if (parsedJson) {
      try {
        return JSON.stringify(parsedJson, null, 2);
      } catch {
        return content;
      }
    }
    return content;
  }, [parsedJson, content]);

  const handleCopy = () => {
    navigator.clipboard.writeText(formattedContent);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 bg-[var(--color-surface-2)] p-0.5 rounded-lg border border-[var(--color-border-subtle)]">
            <button
              type="button"
              onClick={() => setViewMode('tree')}
              disabled={!!parseError}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
                viewMode === 'tree' && !parseError
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] disabled:opacity-40'
              }`}
            >
              <Braces className="h-3 w-3" />
              <span>Interactive Tree</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode('formatted')}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
                viewMode === 'formatted' || !!parseError
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <FileText className="h-3 w-3" />
              <span>Formatted Code</span>
            </button>
          </div>

          {parseError && (
            <span className="text-[11px] text-amber-400 bg-amber-400/10 px-2 py-0.5 rounded border border-amber-400/20">
              Raw view (Syntax error in JSON)
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {viewMode === 'tree' && !parseError && (
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
              <input
                type="text"
                placeholder="Filter keys..."
                value={searchFilter}
                onChange={(e) => setSearchFilter(e.target.value)}
                className="pl-8 pr-3 py-1 bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] rounded text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-blue-500 w-40"
              />
            </div>
          )}

          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-1 rounded text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Copy structured content"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-auto p-4">
        {viewMode === 'tree' && !parseError && parsedJson !== null ? (
          <div className="p-2 space-y-1 max-w-4xl">
            {typeof parsedJson === 'object' ? (
              Object.keys(parsedJson).map((key) => (
                <TreeNode
                  key={key}
                  name={key}
                  value={parsedJson[key]}
                  path={key}
                  depth={0}
                  searchFilter={searchFilter}
                />
              ))
            ) : (
              <div className="font-mono text-xs">{String(parsedJson)}</div>
            )}
          </div>
        ) : (
          <textarea
            value={formattedContent}
            readOnly={isReadOnly}
            onChange={(e) => onChangeText?.(e.target.value)}
            spellCheck={false}
            className="w-full h-full resize-none bg-[var(--color-canvas)] p-2 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] focus:outline-none selection:bg-zinc-800"
          />
        )}
      </div>
    </div>
  );
};
