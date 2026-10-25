import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Dna, Minus, Plus, RotateCcw } from 'lucide-react';
import {
  formatBp,
  genomeFormat,
  packRows,
  parseGenome,
  type GenomeFeature,
} from '@/lib/genome';

const ROW_H = 16;
const MAX_ROWS = 60; // Cap drawn rows so dense contigs don't blow up the DOM
const RULER_H = 28;
const MIN_SPAN = 20; // Don't zoom past ~20 bp

const CATEGORY_COLORS = [
  'bg-emerald-500/80 border-emerald-400 text-emerald-100',
  'bg-blue-500/80 border-blue-400 text-blue-100',
  'bg-purple-500/80 border-purple-400 text-purple-100',
  'bg-amber-500/80 border-amber-400 text-amber-100',
  'bg-rose-500/80 border-rose-400 text-rose-100',
  'bg-cyan-500/80 border-cyan-400 text-cyan-100',
];

interface GenomeViewProps {
  filename: string;
  text: string;
  className?: string;
}

export const GenomeView: React.FC<GenomeViewProps> = ({ filename, text, className = '' }) => {
  const ext = filename.split('.').pop() || '';
  const format = useMemo(() => genomeFormat(ext), [ext]);
  const data = useMemo(
    () => (format ? parseGenome(text, format) : null),
    [format, text]
  );

  const [contigIdx, setContigIdx] = useState(0);
  const contig = data?.contigs[contigIdx];

  // Features on selected contig
  const { contigFeatures, rows } = useMemo(() => {
    if (!data || !contig) return { contigFeatures: [] as GenomeFeature[], rows: [] as number[] };
    const cf = data.features.filter((f) => f.chrom === contig.name);
    return { contigFeatures: cf, rows: packRows(cf) };
  }, [data, contig]);

  // Unique categories for color mapping
  const categoryMap = useMemo(() => {
    const set = new Set<string>();
    for (const f of contigFeatures) {
      if (f.type) set.add(f.type);
      else if (f.strand) set.add(f.strand === '+' ? 'Forward (+)' : 'Reverse (-)');
    }
    const map = new Map<string, number>();
    let idx = 0;
    for (const cat of set) {
      map.set(cat, idx % CATEGORY_COLORS.length);
      idx++;
    }
    return map;
  }, [contigFeatures]);

  const [view, setView] = useState<{ start: number; end: number } | null>(null);
  const [width, setWidth] = useState(760);
  const [hover, setHover] = useState<{ f: GenomeFeature; x: number; y: number } | null>(null);
  const boxRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{ x: number; startBp: number; span: number } | null>(null);

  // Reset viewport when contig changes
  useEffect(() => {
    if (!contig) {
      setView(null);
      return;
    }
    const pad = Math.max(1, Math.round((contig.max - contig.min) * 0.02));
    setView({ start: Math.max(0, contig.min - pad), end: contig.max + pad });
    setHover(null);
  }, [contig]);

  useLayoutEffect(() => {
    const el = boxRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => setWidth(el.clientWidth || 760));
    ro.observe(el);
    setWidth(el.clientWidth || 760);
    return () => ro.disconnect();
  }, []);

  if (!format) {
    return <div className="p-4 text-xs text-[var(--color-fg-muted)]">Not a recognized genome annotation format.</div>;
  }

  if (!data || data.features.length === 0 || !contig || !view) {
    return (
      <div className="p-8 text-center text-xs text-[var(--color-fg-muted)]">
        No genomic features found in {format.toUpperCase()} annotation file.
      </div>
    );
  }

  const extent = { start: Math.max(0, contig.min - 1), end: contig.max + 1 };
  const span = Math.max(MIN_SPAN, view.end - view.start);
  const xOf = (bp: number) => ((bp - view.start) / span) * width;
  const rowsShown = Math.min(MAX_ROWS, rows.length ? Math.max(...rows) + 1 : 1);
  const bodyH = rowsShown * ROW_H + 8;

  const clampView = (start: number, end: number) => {
    let s = start;
    let e = end;
    const sp = Math.max(MIN_SPAN, Math.min(e - s, extent.end - extent.start));
    if (s < extent.start) {
      s = extent.start;
      e = s + sp;
    }
    if (e > extent.end) {
      e = extent.end;
      s = e - sp;
    }
    return { start: Math.max(extent.start, s), end: e };
  };

  const zoomAt = (factor: number, atX: number) => {
    const frac = Math.max(0, Math.min(1, atX / (width || 1)));
    const anchor = view.start + frac * span;
    const newSpan = span * factor;
    const newStart = Math.round(anchor - frac * newSpan);
    const newEnd = Math.round(newStart + newSpan);
    setView(clampView(newStart, newEnd));
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const rect = boxRef.current?.getBoundingClientRect();
    const atX = rect ? e.clientX - rect.left : width / 2;
    const factor = e.deltaY > 0 ? 1.25 : 0.8;
    zoomAt(factor, atX);
  };

  const handlePointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    dragRef.current = { x: e.clientX, startBp: view.start, span };
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (dragRef.current) {
      const dx = e.clientX - dragRef.current.x;
      const bpShift = Math.round((-dx / (width || 1)) * dragRef.current.span);
      const newStart = dragRef.current.startBp + bpShift;
      setView(clampView(newStart, newStart + dragRef.current.span));
    }
  };

  const handlePointerUp = () => {
    dragRef.current = null;
  };

  // Generate ruler tick marks
  const ticks = (() => {
    const targetTicks = Math.max(3, Math.round(width / 120));
    const rawStep = span / targetTicks;
    const mag = 10 ** Math.floor(Math.log10(rawStep));
    const norm = rawStep / mag;
    const step = (norm < 2 ? 1 : norm < 5 ? 2 : 5) * mag;
    const first = Math.ceil(view.start / step) * step;
    const list: number[] = [];
    for (let bp = first; bp <= view.end; bp += step) {
      list.push(bp);
    }
    return list;
  })();

  return (
    <div className={`flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] select-none ${className}`}>
      {/* Sub-Header Toolbar */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-4 py-2 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs">
        <div className="flex items-center gap-2">
          <Dna className="size-4 text-emerald-400 shrink-0" />
          <span className="font-semibold text-xs text-[var(--color-editor-fg)] truncate max-w-[200px]">
            {filename}
          </span>
          <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border border-[var(--color-border-subtle)]">
            {format.toUpperCase()}
          </span>
          <span className="text-[11px] font-mono text-[var(--color-fg-muted)]">
            {data.features.length.toLocaleString()} features
            {data.truncated && ' (capped)'}
          </span>
        </div>

        {/* Contig selector & Zoom controls */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5 text-xs">
            <span className="text-[var(--color-fg-muted)] font-mono text-[11px]">Contig:</span>
            <select
              value={contigIdx}
              onChange={(e) => setContigIdx(Number(e.target.value))}
              className="bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] border border-[var(--color-border-subtle)] rounded px-2 py-0.5 text-xs font-mono focus:outline-none"
            >
              {data.contigs.map((c, i) => (
                <option key={c.name} value={i}>
                  {c.name} ({c.count.toLocaleString()} feats)
                </option>
              ))}
            </select>
          </div>

          <div className="h-4 w-px bg-[var(--color-border-subtle)] mx-1" />

          {/* Span indicator */}
          <span className="text-[11px] font-mono text-[var(--color-fg-subtle)]">
            {formatBp(span)} ({view.start.toLocaleString()} - {view.end.toLocaleString()})
          </span>

          {/* Zoom In/Out/Reset */}
          <div className="flex items-center gap-0.5 bg-[var(--color-surface-2)] p-0.5 rounded border border-[var(--color-border-subtle)]">
            <button
              type="button"
              onClick={() => zoomAt(0.75, width / 2)}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-white hover:bg-white/10"
              title="Zoom In"
            >
              <Plus className="size-3" />
            </button>
            <button
              type="button"
              onClick={() => zoomAt(1.33, width / 2)}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-white hover:bg-white/10"
              title="Zoom Out"
            >
              <Minus className="size-3" />
            </button>
            <button
              type="button"
              onClick={() => {
                const pad = Math.max(1, Math.round((contig.max - contig.min) * 0.02));
                setView({ start: Math.max(0, contig.min - pad), end: contig.max + pad });
              }}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-white hover:bg-white/10"
              title="Reset View"
            >
              <RotateCcw className="size-3" />
            </button>
          </div>
        </div>
      </div>

      {/* Main Track View Area */}
      <div
        ref={boxRef}
        onWheel={handleWheel}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        className="flex-1 overflow-x-hidden overflow-y-auto relative cursor-grab active:cursor-grabbing bg-black/30 p-4"
      >
        {/* Base-Pair Axis Ruler */}
        <svg width={width} height={RULER_H} className="border-b border-white/10 shrink-0">
          <line x1={0} y1={RULER_H - 1} x2={width} y2={RULER_H - 1} stroke="rgba(255,255,255,0.2)" strokeWidth={1} />
          {ticks.map((bp) => {
            const x = xOf(bp);
            if (x < 0 || x > width) return null;
            return (
              <g key={bp} transform={`translate(${x}, 0)`}>
                <line x1={0} y1={RULER_H - 7} x2={0} y2={RULER_H - 1} stroke="rgba(255,255,255,0.4)" strokeWidth={1} />
                <text
                  x={2}
                  y={RULER_H - 10}
                  fill="rgba(255,255,255,0.5)"
                  fontSize={10}
                  fontFamily="monospace"
                >
                  {formatBp(bp)}
                </text>
              </g>
            );
          })}
        </svg>

        {/* Feature Tracks */}
        <div className="relative mt-2" style={{ height: `${bodyH}px`, width: `${width}px` }}>
          {contigFeatures.map((f, i) => {
            const x1 = Math.max(0, xOf(f.start));
            const x2 = Math.min(width, xOf(f.end));
            const w = Math.max(2, x2 - x1);
            if (x2 < 0 || x1 > width) return null;

            const row = rows[i] || 0;
            if (row >= MAX_ROWS) return null;
            const top = row * ROW_H;

            const category = f.type || (f.strand ? (f.strand === '+' ? 'Forward (+)' : 'Reverse (-)') : 'Feature');
            const colorIdx = categoryMap.get(category) ?? 0;
            const colorClass = CATEGORY_COLORS[colorIdx];

            return (
              <div
                key={i}
                onMouseEnter={(e) => {
                  const rect = boxRef.current?.getBoundingClientRect();
                  setHover({
                    f,
                    x: e.clientX - (rect?.left || 0),
                    y: e.clientY - (rect?.top || 0) + 16,
                  });
                }}
                onMouseLeave={() => setHover(null)}
                style={{
                  position: 'absolute',
                  left: `${x1}px`,
                  top: `${top}px`,
                  width: `${w}px`,
                  height: `${ROW_H - 3}px`,
                }}
                className={`rounded border text-[9px] font-mono px-1 flex items-center overflow-hidden cursor-pointer transition-opacity hover:opacity-100 ${colorClass}`}
              >
                {w > 30 && <span className="truncate">{f.name || f.type || `${f.start}`}</span>}
              </div>
            );
          })}
        </div>

        {/* Hover Tooltip */}
        {hover && (
          <div
            style={{ left: `${Math.min(width - 220, hover.x)}px`, top: `${hover.y}px` }}
            className="absolute z-50 pointer-events-none rounded-lg border border-white/20 bg-[#1e1e22]/95 backdrop-blur-md p-2.5 shadow-2xl text-[11px] font-mono text-white/90 space-y-1 w-56"
          >
            <div className="font-semibold text-emerald-300 truncate">
              {hover.f.name || hover.f.type || 'Genomic Feature'}
            </div>
            <div className="text-white/60">
              Locus: {hover.f.chrom}:{hover.f.start.toLocaleString()} - {hover.f.end.toLocaleString()}
            </div>
            <div className="text-white/60">
              Length: {formatBp(hover.f.end - hover.f.start + 1)}
            </div>
            {hover.f.type && <div>Type: <span className="text-white/80">{hover.f.type}</span></div>}
            {hover.f.strand && <div>Strand: <span className="text-white/80">{hover.f.strand}</span></div>}
            {hover.f.score !== undefined && <div>Score: <span className="text-white/80">{hover.f.score}</span></div>}
          </div>
        )}
      </div>
    </div>
  );
};
