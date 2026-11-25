import React, { useMemo, useState } from 'react';
import { AlertCircle, ChevronDown, ChevronRight } from 'lucide-react';
import type { RunStepSpan, StepSpanKind } from '@/types/workflow_telemetry';

interface RunWaterfallTimelineProps {
  spans: RunStepSpan[];
  onSelectSpan?: (span: RunStepSpan | null) => void;
  selectedSpanId?: string | null;
}

interface TreeRow {
  span: RunStepSpan;
  depth: number;
  hasChildren: boolean;
}

function buildHierarchyRows(spans: RunStepSpan[], collapsed: Record<string, boolean>): TreeRow[] {
  const byId = new Set(spans.map((s) => s.id));
  const childrenMap = new Map<string | null, RunStepSpan[]>();
  for (const s of spans) {
    const parent = s.parent_id && byId.has(s.parent_id) ? s.parent_id : null;
    const list = childrenMap.get(parent) ?? [];
    list.push(s);
    childrenMap.set(parent, list);
  }
  for (const list of childrenMap.values()) {
    list.sort((a, b) => a.started_at_ms - b.started_at_ms);
  }

  const rows: TreeRow[] = [];
  const visited = new Set<string>();
  const walk = (parentId: string | null, depth: number) => {
    for (const span of childrenMap.get(parentId) ?? []) {
      if (visited.has(span.id)) continue;
      visited.add(span.id);
      const children = childrenMap.get(span.id) ?? [];
      rows.push({ span, depth, hasChildren: children.length > 0 });
      if (!collapsed[span.id]) {
        walk(span.id, depth + 1);
      }
    }
  };
  walk(null, 0);
  return rows;
}

export const RunWaterfallTimeline: React.FC<RunWaterfallTimelineProps> = ({
  spans,
  onSelectSpan,
  selectedSpanId,
}) => {
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const { minStart, totalDuration } = useMemo(() => {
    if (spans.length === 0) return { minStart: 0, totalDuration: 1 };
    let min = spans[0].started_at_ms;
    let max = spans[0].ended_at_ms ?? spans[0].started_at_ms + 100;
    for (const s of spans) {
      if (s.started_at_ms < min) min = s.started_at_ms;
      const end = s.ended_at_ms ?? s.started_at_ms + 100;
      if (end > max) max = end;
    }
    return { minStart: min, totalDuration: Math.max(1, max - min) };
  }, [spans]);

  const rows = useMemo(() => buildHierarchyRows(spans, collapsed), [spans, collapsed]);

  const kindBadge = (kind: StepSpanKind) => {
    switch (kind) {
      case 'llm':
        return 'text-violet-300 bg-violet-500/15 border-violet-500/30';
      case 'tool':
        return 'text-sky-300 bg-sky-500/15 border-sky-500/30';
      case 'agent':
        return 'text-amber-300 bg-amber-500/15 border-amber-500/30';
      case 'error':
        return 'text-rose-300 bg-rose-500/15 border-rose-500/30';
      case 'verification':
        return 'text-emerald-300 bg-emerald-500/15 border-emerald-500/30';
      case 'session':
        return 'text-purple-300 bg-purple-500/15 border-purple-500/30';
      case 'harness':
      default:
        return 'text-white/60 bg-white/5 border-white/10';
    }
  };

  const kindBarColor = (kind: StepSpanKind, status: string) => {
    if (status === 'error') return 'bg-rose-500';
    switch (kind) {
      case 'llm':
        return 'bg-violet-500';
      case 'tool':
        return 'bg-sky-500';
      case 'agent':
        return 'bg-amber-500';
      case 'verification':
        return 'bg-emerald-500';
      case 'session':
        return 'bg-purple-500';
      case 'harness':
      default:
        return 'bg-white/40';
    }
  };

  if (spans.length === 0) {
    return (
      <div className="p-6 text-center text-xs text-white/50">
        No execution step spans recorded for this run.
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-white/10 bg-[#0d0e11]">
      <div className="flex items-center justify-between border-b border-white/8 bg-white/[0.02] px-3 py-2 text-[10px] font-mono text-white/40">
        <span className="w-1/2">Step / Span Hierarchy ({spans.length})</span>
        <div className="flex w-1/2 items-center justify-between pl-4">
          <span>0ms</span>
          <span>{Math.round(totalDuration / 2)}ms</span>
          <span>{totalDuration}ms</span>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-2 space-y-1">
        {rows.map(({ span, depth, hasChildren }) => {
          const isSelected = selectedSpanId === span.id;
          const leftPercent = Math.max(
            0,
            Math.min(100, ((span.started_at_ms - minStart) / totalDuration) * 100)
          );
          const endMs = span.ended_at_ms ?? span.started_at_ms + 50;
          const widthPercent = Math.max(
            1.5,
            Math.min(100 - leftPercent, ((endMs - span.started_at_ms) / totalDuration) * 100)
          );
          const durMs = span.duration_ms ?? Math.max(0, endMs - span.started_at_ms);

          return (
            <div
              key={span.id}
              onClick={() => onSelectSpan?.(isSelected ? null : span)}
              className={`group flex items-center justify-between rounded px-2 py-1.5 transition cursor-pointer text-xs ${
                isSelected
                  ? 'bg-white/[0.12] border border-white/20'
                  : 'hover:bg-white/[0.04] border border-transparent'
              }`}
            >
              <div
                className="flex w-1/2 min-w-0 items-center gap-1.5 pr-2"
                style={{ paddingLeft: `${Math.min(depth, 8) * 14}px` }}
              >
                {hasChildren ? (
                  <button
                    type="button"
                    aria-label={collapsed[span.id] ? 'Expand span branch' : 'Collapse span branch'}
                    onClick={(e) => {
                      e.stopPropagation();
                      setCollapsed((prev) => ({ ...prev, [span.id]: !prev[span.id] }));
                    }}
                    className="shrink-0 rounded p-0.5 text-white/50 hover:bg-white/10 hover:text-white"
                  >
                    {collapsed[span.id] ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
                  </button>
                ) : (
                  <span className="w-4 shrink-0" />
                )}
                <span
                  className={`shrink-0 rounded px-1.5 py-0.5 font-mono text-[9px] uppercase font-semibold border ${kindBadge(
                    span.kind
                  )}`}
                >
                  {span.kind}
                </span>
                <span className="truncate font-mono text-[11px] text-white/90 group-hover:text-white">
                  {span.name}
                </span>
                {span.status === 'error' && (
                  <AlertCircle size={12} className="shrink-0 text-rose-400" />
                )}
                <span className="ml-auto shrink-0 font-mono text-[10px] text-white/40">
                  {durMs >= 1000 ? `${(durMs / 1000).toFixed(2)}s` : `${durMs}ms`}
                </span>
              </div>

              <div className="relative flex h-3 w-1/2 items-center rounded bg-white/[0.03] pl-4">
                <div
                  className={`absolute h-2 rounded transition-all ${kindBarColor(
                    span.kind,
                    span.status
                  )} ${span.status === 'running' ? 'animate-pulse' : ''}`}
                  style={{
                    left: `${leftPercent}%`,
                    width: `${widthPercent}%`,
                  }}
                  title={`${span.name}: ${durMs}ms (${leftPercent.toFixed(1)}% offset)`}
                />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
