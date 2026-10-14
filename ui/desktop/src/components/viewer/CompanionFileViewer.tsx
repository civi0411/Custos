import React, { useMemo } from 'react';
import {
  FileText,
  FileCode,
  FileSpreadsheet,
  Image as ImageIcon,
  Globe,
  Binary,
  Braces,
  Save,
} from 'lucide-react';
import { MarkdownViewer } from './MarkdownViewer';
import { TabularDataViewer } from './TabularDataViewer';
import { StructuredDataViewer } from './StructuredDataViewer';
import { VisualMediaViewer } from './VisualMediaViewer';
import { WebPreviewViewer } from './WebPreviewViewer';
import { HexInspector } from './HexInspector';
import { CodeTextViewer } from './CodeTextViewer';
import { formatKeyCombo } from '@/lib/utils';

export interface CompanionFileViewerProps {
  path: string;
  content: string;
  isBinary?: boolean;
  sizeBytes?: number;
  lineCount?: number;
  isDirty?: boolean;
  isReadOnly?: boolean;
  onChangeText?: (newText: string) => void;
  onSave?: () => void;
  saving?: boolean;
  extraHeaderActions?: React.ReactNode;
  className?: string;
}

export type DetectedFileType =
  | 'markdown'
  | 'csv'
  | 'tsv'
  | 'json'
  | 'yaml'
  | 'toml'
  | 'svg'
  | 'image'
  | 'html'
  | 'code'
  | 'binary'
  | 'plain';

export function detectFileType(path: string, isBinary?: boolean): { type: DetectedFileType; label: string; icon: React.ComponentType<{ className?: string }> } {
  const ext = path.split('.').pop()?.toLowerCase() || '';

  if (isBinary) {
    if (['png', 'jpg', 'jpeg', 'webp', 'gif', 'ico', 'bmp'].includes(ext)) {
      return { type: 'image', label: ext.toUpperCase() + ' Image', icon: ImageIcon };
    }
    return { type: 'binary', label: 'Binary File', icon: Binary };
  }

  switch (ext) {
    case 'md':
    case 'markdown':
    case 'mdx':
      return { type: 'markdown', label: 'Markdown Spec', icon: FileText };
    case 'csv':
      return { type: 'csv', label: 'CSV Table', icon: FileSpreadsheet };
    case 'tsv':
      return { type: 'tsv', label: 'TSV Table', icon: FileSpreadsheet };
    case 'json':
    case 'jsonl':
      return { type: 'json', label: 'JSON Data', icon: Braces };
    case 'yaml':
    case 'yml':
      return { type: 'yaml', label: 'YAML Config', icon: Braces };
    case 'toml':
      return { type: 'toml', label: 'TOML Manifest', icon: Braces };
    case 'svg':
      return { type: 'svg', label: 'SVG Graphic', icon: ImageIcon };
    case 'html':
    case 'htm':
      return { type: 'html', label: 'HTML Preview', icon: Globe };
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'webp':
    case 'gif':
    case 'ico':
    case 'bmp':
      return { type: 'image', label: ext.toUpperCase() + ' Image', icon: ImageIcon };
    case 'rs':
      return { type: 'code', label: 'Rust Source', icon: FileCode };
    case 'ts':
    case 'tsx':
      return { type: 'code', label: 'TypeScript', icon: FileCode };
    case 'js':
    case 'jsx':
      return { type: 'code', label: 'JavaScript', icon: FileCode };
    case 'py':
      return { type: 'code', label: 'Python Script', icon: FileCode };
    case 'go':
      return { type: 'code', label: 'Go Source', icon: FileCode };
    case 'c':
    case 'cpp':
    case 'h':
    case 'hpp':
      return { type: 'code', label: 'C / C++', icon: FileCode };
    case 'sh':
    case 'bash':
    case 'zsh':
      return { type: 'code', label: 'Shell Script', icon: FileCode };
    case 'sql':
      return { type: 'code', label: 'SQL Query', icon: FileCode };
    case 'css':
    case 'scss':
      return { type: 'code', label: 'Stylesheet', icon: FileCode };
    default:
      return { type: 'plain', label: ext ? ext.toUpperCase() : 'Plain Text', icon: FileText };
  }
}

export const CompanionFileViewer: React.FC<CompanionFileViewerProps> = ({
  path,
  content,
  isBinary = false,
  sizeBytes = 0,
  lineCount = 0,
  isDirty = false,
  isReadOnly = false,
  onChangeText,
  onSave,
  saving = false,
  extraHeaderActions,
  className = '',
}) => {
  const { type, label, icon: TypeIcon } = useMemo(() => detectFileType(path, isBinary), [path, isBinary]);

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden ${className}`}>
      {/* Top Header Bar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
        {/* Left: Path, Type Badge & Dirty State */}
        <div className="flex items-center gap-2.5 min-w-0">
          <TypeIcon className="h-4 w-4 text-[var(--color-fg-muted)] shrink-0" />
          <span className="font-mono text-xs font-semibold text-[var(--color-editor-fg)] truncate max-w-sm">
            {path}
          </span>
          {isDirty && (
            <span className="h-2 w-2 rounded-full bg-amber-400 shrink-0" title="Unsaved changes" />
          )}

          <span className="text-[10px] uppercase font-mono font-medium px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-subtle)] shrink-0">
            {label}
          </span>
          {lineCount > 0 && (
            <span className="text-[10px] text-[var(--color-fg-subtle)] hidden sm:inline">
              ({lineCount.toLocaleString()} lines)
            </span>
          )}
        </div>

        {/* Right: Actions & Save Button */}
        <div className="flex items-center gap-2 shrink-0">
          {extraHeaderActions}

          {onSave && !isReadOnly && (
            <button
              type="button"
              onClick={onSave}
              disabled={saving || !isDirty}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition ${
                isDirty
                  ? 'bg-emerald-600 text-white hover:bg-emerald-500 shadow-sm'
                  : 'border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] opacity-50 cursor-not-allowed'
              }`}
            >
              <Save className="h-3 w-3" />
              <span>{saving ? 'Saving...' : `Save ${formatKeyCombo({ ctrlOrCmd: true, key: 'S' })}`}</span>
            </button>
          )}
        </div>
      </div>

      {/* Viewer Body Router */}
      <div className="flex-1 overflow-hidden relative">
        {type === 'markdown' ? (
          <MarkdownViewer
            content={content}
            isReadOnly={isReadOnly}
            onChangeText={onChangeText}
          />
        ) : type === 'csv' || type === 'tsv' ? (
          <TabularDataViewer
            content={content}
            isTsv={type === 'tsv'}
            isReadOnly={isReadOnly}
            onChangeText={onChangeText}
          />
        ) : type === 'json' || type === 'yaml' || type === 'toml' ? (
          <StructuredDataViewer
            content={content}
            format={type}
            isReadOnly={isReadOnly}
            onChangeText={onChangeText}
          />
        ) : type === 'svg' || type === 'image' ? (
          <VisualMediaViewer
            path={path}
            content={content}
            isBinary={isBinary}
            sizeBytes={sizeBytes}
            isReadOnly={isReadOnly}
            onChangeText={onChangeText}
          />
        ) : type === 'html' ? (
          <WebPreviewViewer
            path={path}
            content={content}
            isReadOnly={isReadOnly}
            onChangeText={onChangeText}
          />
        ) : type === 'binary' ? (
          <HexInspector
            path={path}
            content={content}
            sizeBytes={sizeBytes}
          />
        ) : (
          <CodeTextViewer
            path={path}
            content={content}
            isReadOnly={isReadOnly}
            onChangeText={onChangeText}
            language={label.split(' ')[0].toLowerCase()}
          />
        )}
      </div>
    </div>
  );
};
