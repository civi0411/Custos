import React, { useMemo } from 'react';
import { Binary, Download, File } from 'lucide-react';

interface HexInspectorProps {
  path: string;
  content: string; // Base64 encoded string from backend
  sizeBytes?: number;
  className?: string;
}

interface HexRow {
  offset: string;
  hexBytes: string[];
  ascii: string;
}

export const HexInspector: React.FC<HexInspectorProps> = ({
  path,
  content,
  sizeBytes = 0,
  className = '',
}) => {

  const hexRows: HexRow[] = useMemo(() => {
    let bytes: Uint8Array;
    try {
      // Decode base64 string
      const binaryString = atob(content);
      const len = Math.min(binaryString.length, 16384); // Show first 16KB for high performance
      bytes = new Uint8Array(len);
      for (let i = 0; i < len; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
    } catch {
      // Fallback if not base64
      const enc = new TextEncoder();
      bytes = enc.encode(content.slice(0, 16384));
    }

    const rows: HexRow[] = [];
    for (let i = 0; i < bytes.length; i += 16) {
      const chunk = bytes.slice(i, i + 16);
      const hexBytes: string[] = [];
      let ascii = '';

      for (let j = 0; j < 16; j++) {
        if (j < chunk.length) {
          const b = chunk[j];
          hexBytes.push(b.toString(16).padStart(2, '0'));
          ascii += b >= 32 && b <= 126 ? String.fromCharCode(b) : '.';
        } else {
          hexBytes.push('  ');
          ascii += ' ';
        }
      }

      rows.push({
        offset: i.toString(16).padStart(8, '0'),
        hexBytes,
        ascii,
      });
    }

    return rows;
  }, [content]);

  const handleDownload = () => {
    try {
      const binaryString = atob(content);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
      const blob = new Blob([bytes], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = path.split('/').pop() || 'binary.bin';
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      // Fallback
    }
  };

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs select-none">
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-1.5 font-semibold text-xs text-[var(--color-editor-fg)]">
            <Binary className="h-3.5 w-3.5 text-purple-400" />
            <span>Hex Inspection</span>
          </span>
          <span className="text-[11px] font-mono text-[var(--color-fg-muted)] px-2 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)]">
            {(sizeBytes / 1024).toFixed(1)} KB • Showing first {hexRows.length * 16} bytes
          </span>
        </div>

        <button
          type="button"
          onClick={handleDownload}
          className="flex items-center gap-1 px-2.5 py-1 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-3)] text-xs transition"
        >
          <Download className="h-3.5 w-3.5" />
          <span>Download Binary</span>
        </button>
      </div>

      {/* Hex Dump Viewport */}
      <div className="flex-1 overflow-auto p-5 font-mono text-xs leading-relaxed select-text">
        {hexRows.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-12 text-center text-[var(--color-fg-muted)]">
            <File className="h-10 w-10 opacity-30 mb-2" />
            <p className="text-xs">No binary data available for this file.</p>
          </div>
        ) : (
          <div className="inline-block min-w-full space-y-0.5">
            {hexRows.map((row, idx) => (
              <div key={idx} className="flex items-center gap-4 hover:bg-[var(--color-surface-1)] py-0.5 px-2 rounded">
                {/* Offset */}
                <span className="text-[var(--color-fg-subtle)] select-none w-20 shrink-0">
                  {row.offset}
                </span>

                {/* Hex Bytes in two 8-byte groups */}
                <div className="flex items-center gap-2 text-zinc-300 shrink-0 font-medium">
                  <div className="flex gap-1.5">
                    {row.hexBytes.slice(0, 8).map((b, bi) => (
                      <span key={bi} className={b === '00' ? 'text-zinc-600' : 'text-sky-300'}>
                        {b}
                      </span>
                    ))}
                  </div>
                  <span className="text-[var(--color-fg-subtle)] select-none">|</span>
                  <div className="flex gap-1.5">
                    {row.hexBytes.slice(8, 16).map((b, bi) => (
                      <span key={bi} className={b === '00' ? 'text-zinc-600' : 'text-sky-300'}>
                        {b}
                      </span>
                    ))}
                  </div>
                </div>

                {/* ASCII Column */}
                <span className="text-emerald-400 font-mono tracking-widest pl-2 border-l border-[var(--color-border-subtle)] select-text">
                  {row.ascii}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
