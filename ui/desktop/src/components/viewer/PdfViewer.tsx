import React, { useEffect, useState } from 'react';
import {
  FileText,
  Download,
  ExternalLink,
  Globe,
  AlertCircle,
  ZoomIn,
  ZoomOut,
  RotateCcw,
} from 'lucide-react';

interface PdfViewerProps {
  path: string;
  content: string; // Base64-encoded PDF data from backend
  sizeBytes?: number;
  onOpenInBrowser?: (url: string) => void;
  className?: string;
}

export function decodePdfBytes(content: string): Uint8Array {
  if (content.startsWith('%PDF-')) throw new Error('The daemon returned PDF bytes as text. Refresh the daemon before reopening this file.');
  let binary: string;
  try { binary = atob(content.replace(/^data:application\/pdf;base64,/, '')); }
  catch { throw new Error('The PDF response is not valid base64. No partial download was created.'); }
  if (!binary.startsWith('%PDF-')) throw new Error('The response does not contain a PDF header.');
  return Uint8Array.from(binary, character => character.charCodeAt(0));
}

export const PdfViewer: React.FC<PdfViewerProps> = ({
  path,
  content,
  sizeBytes = 0,
  onOpenInBrowser,
  className = '',
}) => {
  const [loadError, setLoadError] = useState<string | null>(null);
  const [zoomScale, setZoomScale] = useState<number>(100);

  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  useEffect(() => {
    setLoadError(null); setBlobUrl(null);
    let url: string | null = null;
    try {
      const bytes = decodePdfBytes(content);
      url = URL.createObjectURL(new Blob([bytes.buffer as ArrayBuffer], { type: 'application/pdf' }));
      setBlobUrl(url);
    } catch (reason) { setLoadError(reason instanceof Error ? reason.message : 'Invalid PDF bytes'); }
    return () => { if (url) URL.revokeObjectURL(url); };
  }, [content]);

  const filename = path.split('/').pop() || 'document.pdf';

  const handleDownload = () => {
    if (!blobUrl) return;
    const a = document.createElement('a');
    a.href = blobUrl;
    a.download = filename;
    a.click();
  };

  const handleOpenExternal = () => {
    if (!blobUrl) return;
    window.open(blobUrl, '_blank');
  };

  const handleOpenInBrowserWorkbench = () => {
    if (blobUrl && onOpenInBrowser) {
      onOpenInBrowser(blobUrl);
    } else if (blobUrl) {
      window.open(blobUrl, '_blank');
    }
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-1.5 font-semibold text-xs text-[var(--color-editor-fg)]">
            <FileText className="h-4 w-4 text-rose-400" />
            <span className="truncate max-w-[200px]">{filename}</span>
          </span>
          {sizeBytes > 0 && (
            <span className="text-[11px] font-mono text-[var(--color-fg-muted)] px-2 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)]">
              {(sizeBytes / 1024).toFixed(1)} KB
            </span>
          )}
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-1.5">
          {/* Zoom controls for iframe container */}
          <div className="hidden sm:flex items-center gap-1 bg-[var(--color-surface-2)] px-1 py-0.5 rounded border border-[var(--color-border-subtle)]">
            <button
              type="button"
              onClick={() => setZoomScale((z) => Math.max(50, z - 15))}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Zoom out"
            >
              <ZoomOut className="h-3 w-3" />
            </button>
            <span className="w-10 text-center font-mono text-[10px] text-[var(--color-fg-muted)]">
              {zoomScale}%
            </span>
            <button
              type="button"
              onClick={() => setZoomScale((z) => Math.min(200, z + 15))}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Zoom in"
            >
              <ZoomIn className="h-3 w-3" />
            </button>
            <button
              type="button"
              onClick={() => setZoomScale(100)}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Reset Zoom"
            >
              <RotateCcw className="h-3 w-3" />
            </button>
          </div>

          {/* Open in Custos Browser Workbench */}
          <button
            type="button"
            onClick={handleOpenInBrowserWorkbench}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-3)] text-xs text-sky-400 font-medium transition"
            title="Open in Browser Workbench tab"
          >
            <Globe className="h-3.5 w-3.5" />
            <span>Open in Browser</span>
          </button>

          {/* External Window */}
          <button
            type="button"
            onClick={handleOpenExternal}
            className="p-1.5 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Open in external viewer window"
          >
            <ExternalLink className="h-3.5 w-3.5" />
          </button>

          {/* Download PDF */}
          <button
            type="button"
            onClick={handleDownload}
            className="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-3)] text-xs transition"
            title="Download PDF"
          >
            <Download className="h-3.5 w-3.5" />
            <span>Download</span>
          </button>
        </div>
      </div>

      {/* Main PDF Viewport */}
      <div className="flex-1 relative overflow-hidden bg-zinc-950/40 flex items-center justify-center p-2">
        {loadError ? (
          <div className="flex flex-col items-center justify-center p-8 text-center text-[var(--color-fg-muted)] max-w-md">
            <AlertCircle className="h-10 w-10 text-rose-400 mb-3" />
            <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">Unable to render PDF</h3>
            <p className="mt-1 text-xs text-[var(--color-fg-muted)] leading-relaxed">
              {loadError}
            </p>
            <button
              type="button"
              onClick={handleDownload}
              disabled={!blobUrl}
              className="mt-4 flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 text-white text-xs font-medium hover:bg-emerald-500"
            >
              <Download className="h-3.5 w-3.5" />
              <span>Download File Directly</span>
            </button>
          </div>
        ) : blobUrl ? (
          <div
            className="w-full h-full flex items-center justify-center transition-transform duration-150 origin-top"
            style={{
              transform: zoomScale !== 100 ? `scale(${zoomScale / 100})` : undefined,
              height: zoomScale !== 100 ? `${(100 / zoomScale) * 100}%` : '100%',
              width: zoomScale !== 100 ? `${(100 / zoomScale) * 100}%` : '100%',
            }}
          >
            <iframe
              src={blobUrl}
              title={path}
              className="w-full h-full border-none rounded-sm shadow-xl bg-white"
            />
          </div>
        ) : (
          <div className="flex items-center justify-center text-xs text-[var(--color-fg-muted)]">
            Loading PDF document...
          </div>
        )}
      </div>
    </div>
  );
};
