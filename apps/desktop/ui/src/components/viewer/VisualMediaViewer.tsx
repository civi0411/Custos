import React, { useState, useRef, useMemo } from 'react';
import { ZoomIn, ZoomOut, RotateCcw, Maximize2, Image as ImageIcon, Code, Copy, Check } from 'lucide-react';

interface VisualMediaViewerProps {
  path: string;
  content: string; // Base64 string for binary images, or SVG XML string
  isBinary?: boolean;
  sizeBytes?: number;
  className?: string;
  isReadOnly?: boolean;
  onChangeText?: (text: string) => void;
}

export const VisualMediaViewer: React.FC<VisualMediaViewerProps> = ({
  path,
  content,
  sizeBytes = 0,
  className = '',
  isReadOnly = true,
  onChangeText,
}) => {
  const isSvg = path.toLowerCase().endsWith('.svg');
  const [viewMode, setViewMode] = useState<'preview' | 'code'>(isSvg ? 'preview' : 'preview');
  const [zoomLevel, setZoomLevel] = useState<number>(100);
  const [naturalSize, setNaturalSize] = useState<{ width: number; height: number } | null>(null);
  const [copied, setCopied] = useState(false);
  const imgRef = useRef<HTMLImageElement | null>(null);

  // Compute image source
  const imageSrc = useMemo(() => {
    if (isSvg) {
      if (content.startsWith('data:')) return content;
      // If SVG XML text
      const cleanSvg = content.trim();
      return `data:image/svg+xml;utf8,${encodeURIComponent(cleanSvg)}`;
    }

    if (content.startsWith('data:')) {
      return content;
    }

    // Determine mime type from extension
    const ext = path.split('.').pop()?.toLowerCase();
    let mime = 'image/png';
    if (ext === 'jpg' || ext === 'jpeg') mime = 'image/jpeg';
    else if (ext === 'webp') mime = 'image/webp';
    else if (ext === 'gif') mime = 'image/gif';
    else if (ext === 'ico') mime = 'image/x-icon';
    else if (ext === 'bmp') mime = 'image/bmp';

    return `data:${mime};base64,${content}`;
  }, [path, content, isSvg]);

  const handleImageLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    setNaturalSize({ width: img.naturalWidth, height: img.naturalHeight });
  };

  const handleZoomIn = () => setZoomLevel((z) => Math.min(500, z + 25));
  const handleZoomOut = () => setZoomLevel((z) => Math.max(25, z - 25));
  const handleResetZoom = () => setZoomLevel(100);
  const handleFit = () => setZoomLevel(75);

  const handleCopyCode = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          {/* SVG Mode Switcher */}
          {isSvg && (
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
                <ImageIcon className="h-3 w-3" />
                <span>Vector Graphic</span>
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
                <span>SVG XML</span>
              </button>
            </div>
          )}

          {/* Image Dimensions & Size Badge */}
          {naturalSize && viewMode === 'preview' && (
            <span className="text-[11px] font-mono text-[var(--color-fg-muted)] px-2 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)]">
              {naturalSize.width} × {naturalSize.height} px • {(sizeBytes / 1024).toFixed(1)} KB
            </span>
          )}
        </div>

        {/* Zoom Controls */}
        {viewMode === 'preview' ? (
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={handleZoomOut}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Zoom out"
            >
              <ZoomOut className="h-3.5 w-3.5" />
            </button>
            <span className="w-12 text-center font-mono text-xs text-[var(--color-fg-muted)]">
              {zoomLevel}%
            </span>
            <button
              type="button"
              onClick={handleZoomIn}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Zoom in"
            >
              <ZoomIn className="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              onClick={handleResetZoom}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Reset 100%"
            >
              <RotateCcw className="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              onClick={handleFit}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Fit to view (75%)"
            >
              <Maximize2 className="h-3.5 w-3.5" />
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={handleCopyCode}
            className="flex items-center gap-1 px-2 py-1 rounded text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
            <span>{copied ? 'Copied' : 'Copy XML'}</span>
          </button>
        )}
      </div>

      {/* Viewport Area */}
      <div className="flex-1 overflow-auto relative">
        {viewMode === 'preview' ? (
          <div
            className="min-h-full min-w-full flex items-center justify-center p-8 overflow-auto"
            style={{
              backgroundImage: `
                linear-gradient(45deg, var(--color-surface-2) 25%, transparent 25%),
                linear-gradient(-45deg, var(--color-surface-2) 25%, transparent 25%),
                linear-gradient(45deg, transparent 75%, var(--color-surface-2) 75%),
                linear-gradient(-45deg, transparent 75%, var(--color-surface-2) 75%)
              `,
              backgroundSize: '20px 20px',
              backgroundPosition: '0 0, 0 10px, 10px -10px, -10px 0px',
            }}
          >
            <img
              ref={imgRef}
              src={imageSrc}
              alt={path}
              onLoad={handleImageLoad}
              style={{
                width: `${zoomLevel}%`,
                maxWidth: 'none',
                transition: 'width 0.15s ease-out',
                imageRendering: zoomLevel > 150 ? 'pixelated' : 'auto',
              }}
              className="rounded shadow-2xl drop-shadow-md select-none border border-[var(--color-border-subtle)]/50"
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
