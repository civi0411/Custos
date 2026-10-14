import React, { useState } from 'react';
import { Globe, Code, RotateCw, Monitor, Tablet, Smartphone, Copy, Check } from 'lucide-react';

interface WebPreviewViewerProps {
  content: string;
  path: string;
  isReadOnly?: boolean;
  onChangeText?: (text: string) => void;
  className?: string;
}

export const WebPreviewViewer: React.FC<WebPreviewViewerProps> = ({
  content,
  path,
  isReadOnly = true,
  onChangeText,
  className = '',
}) => {
  const [viewMode, setViewMode] = useState<'preview' | 'code'>('preview');
  const [deviceWidth, setDeviceWidth] = useState<'100%' | '768px' | '375px'>('100%');
  const [iframeKey, setIframeKey] = useState<number>(0);
  const [copied, setCopied] = useState(false);

  const handleRefresh = () => {
    setIframeKey((k) => k + 1);
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          {/* Mode Switcher */}
          <div className="flex items-center gap-1 bg-[var(--color-surface-2)] p-0.5 rounded-lg border border-[var(--color-border-subtle)]">
            <button
              type="button"
              onClick={() => setViewMode('preview')}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
                viewMode === 'preview'
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <Globe className="h-3 w-3" />
              <span>Interactive Preview</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode('code')}
              className={`flex items-center gap-1 px-2.5 py-1 rounded text-xs font-medium transition ${
                viewMode === 'code'
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <Code className="h-3 w-3" />
              <span>HTML Source</span>
            </button>
          </div>

          {/* Device Width Presets */}
          {viewMode === 'preview' && (
            <div className="flex items-center gap-1 bg-[var(--color-surface-2)] p-0.5 rounded-lg border border-[var(--color-border-subtle)]">
              <button
                type="button"
                onClick={() => setDeviceWidth('100%')}
                title="Full Responsive"
                className={`p-1 rounded text-xs transition ${
                  deviceWidth === '100%'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                <Monitor className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                onClick={() => setDeviceWidth('768px')}
                title="Tablet (768px)"
                className={`p-1 rounded text-xs transition ${
                  deviceWidth === '768px'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                <Tablet className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                onClick={() => setDeviceWidth('375px')}
                title="Mobile (375px)"
                className={`p-1 rounded text-xs transition ${
                  deviceWidth === '375px'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                <Smartphone className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-1.5">
          {viewMode === 'preview' && (
            <button
              type="button"
              onClick={handleRefresh}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Reload preview"
            >
              <RotateCw className="h-3.5 w-3.5" />
            </button>
          )}
          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-1 rounded text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-auto flex justify-center bg-zinc-950/20">
        {viewMode === 'preview' ? (
          <div
            style={{ width: deviceWidth }}
            className="h-full bg-white transition-[width] duration-200 border-x border-[var(--color-border-subtle)] shadow-xl relative"
          >
            <iframe
              key={iframeKey}
              title={path}
              srcDoc={content}
              sandbox="allow-scripts allow-same-origin"
              className="w-full h-full border-none"
            />
          </div>
        ) : (
          <textarea
            value={content}
            readOnly={isReadOnly}
            onChange={(e) => onChangeText?.(e.target.value)}
            spellCheck={false}
            className="w-full h-full resize-none bg-[var(--color-canvas)] p-5 font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] focus:outline-none selection:bg-zinc-800"
          />
        )}
      </div>
    </div>
  );
};
