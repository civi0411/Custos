import React, { useMemo, useState } from 'react';
import { ParsedTableData, ChartType } from '@/types/research';

const SERIES_COLORS = [
  '#58a6ff', // blue
  '#3fb950', // green
  '#d29922', // amber
  '#a371f7', // purple
  '#f85149', // red
  '#79c0ff', // light blue
  '#56d364', // light green
  '#e3b341', // yellow
];

interface ColumnInfo {
  index: number;
  name: string;
  numeric: boolean;
  values: (number | null)[];
}

export const TableChart: React.FC<{ table: ParsedTableData }> = ({ table }) => {
  const cols = useMemo<ColumnInfo[]>(() => {
    return table.headers.map((name, index) => {
      let numCount = 0;
      const values: (number | null)[] = [];
      for (const row of table.rows) {
        const valStr = row[index]?.trim();
        const num = valStr ? Number(valStr) : NaN;
        if (!isNaN(num)) {
          numCount++;
          values.push(num);
        } else {
          values.push(null);
        }
      }
      return {
        index,
        name,
        numeric: numCount > table.rows.length * 0.5 && table.rows.length > 0,
        values,
      };
    });
  }, [table]);

  const numericCols = useMemo(() => cols.filter((c) => c.numeric), [cols]);

  const [type, setType] = useState<ChartType>('line');
  const [xIndex, setXIndex] = useState<number>(-1); // -1 = Row Index
  const [yIndexes, setYIndexes] = useState<number[]>(() => {
    return numericCols.slice(0, 3).map((c) => c.index);
  });

  if (numericCols.length === 0) {
    return (
      <div className="p-6 text-center text-xs text-[#8b949e]">
        No numeric columns found in this dataset for charting.
      </div>
    );
  }

  const n = table.rows.length;
  const xIsValue = type !== 'bar' && xIndex >= 0 && (cols[xIndex]?.numeric ?? false);

  // Compute X values
  const xVals: number[] = [];
  const xLabels: string[] = [];
  for (let i = 0; i < n; i++) {
    if (xIsValue) {
      xVals.push(cols[xIndex].values[i] ?? NaN);
    } else {
      xVals.push(i);
    }
    xLabels.push(xIndex >= 0 ? table.rows[i][xIndex] ?? '' : String(i + 1));
  }

  const validXVals = xVals.filter((v) => Number.isFinite(v));
  const xMin = xIsValue ? (validXVals.length > 0 ? Math.min(...validXVals) : 0) : 0;
  const xMax = xIsValue ? (validXVals.length > 0 ? Math.max(...validXVals) : 1) : Math.max(1, n - 1);
  const xSpan = xMax - xMin || 1;

  // Compute Y range across selected series
  let yMin = type === 'bar' ? 0 : Infinity;
  let yMax = -Infinity;
  for (const yIdx of yIndexes) {
    const col = cols[yIdx];
    if (!col) continue;
    for (const v of col.values) {
      if (v === null || !Number.isFinite(v)) continue;
      if (v < yMin) yMin = v;
      if (v > yMax) yMax = v;
    }
  }

  if (!Number.isFinite(yMin)) yMin = 0;
  if (!Number.isFinite(yMax)) yMax = 100;
  if (yMin === yMax) yMax = yMin + 1;
  const ySpan = yMax - yMin;

  const W = 640;
  const H = 300;
  const pad = { l: 50, r: 20, t: 20, b: 40 };
  const plotW = W - pad.l - pad.r;
  const plotH = H - pad.t - pad.b;

  const xAt = (i: number) => {
    if (xIsValue) {
      const val = xVals[i];
      if (isNaN(val)) return pad.l;
      return pad.l + ((val - xMin) / xSpan) * plotW;
    }
    return pad.l + (n <= 1 ? plotW / 2 : (i / (n - 1)) * plotW);
  };

  const yAt = (v: number) => {
    return pad.t + ((yMax - v) / ySpan) * plotH;
  };

  const toggleY = (idx: number) => {
    setYIndexes((prev) =>
      prev.includes(idx) ? prev.filter((y) => y !== idx) : [...prev, idx].slice(0, 5)
    );
  };

  return (
    <div className="flex flex-col h-full bg-[#0d1117] text-[#c9d1d9] select-none">
      {/* Control Toolbar */}
      <div className="flex flex-wrap items-center gap-3 px-4 py-2.5 border-b border-[#21262d] bg-[#161b22] text-[11px]">
        {/* Chart Type Picker */}
        <div className="flex items-center rounded-lg border border-[#30363d] bg-[#0d1117] p-0.5">
          {(['line', 'bar', 'scatter'] as const).map((t) => (
            <button
              key={t}
              onClick={() => setType(t)}
              className={`px-2 py-0.5 rounded-md font-medium uppercase text-[10px] transition ${
                type === t ? 'bg-[#a371f7] text-white shadow-xs' : 'text-[#8b949e] hover:text-white'
              }`}
            >
              {t}
            </button>
          ))}
        </div>

        {/* X Axis Selector */}
        <div className="flex items-center gap-1.5 text-[#8b949e]">
          <span>X:</span>
          <select
            value={xIndex}
            onChange={(e) => setXIndex(Number(e.target.value))}
            className="rounded-md border border-[#30363d] bg-[#0d1117] px-2 py-0.5 text-[11px] text-[#c9d1d9] outline-none"
          >
            <option value={-1}>Row Index (1..{n})</option>
            {cols.map((c) => (
              <option key={c.index} value={c.index}>
                {c.name} {c.numeric ? '(num)' : ''}
              </option>
            ))}
          </select>
        </div>

        {/* Y Axis Series Chips */}
        <div className="flex flex-wrap items-center gap-1.5 ml-auto">
          <span className="text-[#8b949e] mr-1">Series:</span>
          {numericCols.map((c, i) => {
            const active = yIndexes.includes(c.index);
            const color = SERIES_COLORS[i % SERIES_COLORS.length];
            return (
              <button
                key={c.index}
                onClick={() => toggleY(c.index)}
                className={`flex items-center gap-1 px-2 py-0.5 rounded-md border text-[10.5px] transition ${
                  active
                    ? 'border-[#a371f7]/40 bg-[#161b22] text-white font-medium'
                    : 'border-transparent text-[#8b949e] hover:bg-[#21262d] hover:text-white'
                }`}
              >
                <span
                  className="w-2 h-2 rounded-full inline-block"
                  style={{ backgroundColor: active ? color : '#484f58' }}
                />
                <span className="truncate max-w-[100px]">{c.name}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* SVG Canvas Area */}
      <div className="flex-1 p-4 flex items-center justify-center overflow-hidden">
        <svg
          viewBox={`0 0 ${W} ${H}`}
          className="w-full h-full max-h-[360px] overflow-visible font-mono"
        >
          {/* Horizontal Gridlines */}
          {[0, 0.25, 0.5, 0.75, 1].map((pct) => {
            const y = pad.t + pct * plotH;
            const val = yMax - pct * ySpan;
            return (
              <g key={pct}>
                <line
                  x1={pad.l}
                  y1={y}
                  x2={pad.l + plotW}
                  y2={y}
                  stroke="#21262d"
                  strokeDasharray="3 3"
                />
                <text
                  x={pad.l - 8}
                  y={y + 3}
                  textAnchor="end"
                  fontSize="9.5"
                  fill="#8b949e"
                >
                  {val.toFixed(val > 100 || val === 0 ? 0 : 2)}
                </text>
              </g>
            );
          })}

          {/* Plot Series */}
          {yIndexes.map((yIdx, sIdx) => {
            const col = cols[yIdx];
            if (!col) return null;
            const color = SERIES_COLORS[sIdx % SERIES_COLORS.length];

            if (type === 'bar') {
              const barWidth = Math.max(2, (plotW / n) * 0.75);
              return (
                <g key={yIdx}>
                  {col.values.map((v, i) => {
                    if (v === null) return null;
                    const x = xAt(i) - barWidth / 2;
                    const y = yAt(v);
                    const h = pad.t + plotH - y;
                    return (
                      <rect
                        key={i}
                        x={x}
                        y={y}
                        width={barWidth}
                        height={Math.max(0, h)}
                        fill={color}
                        opacity={0.8}
                        rx={1}
                      />
                    );
                  })}
                </g>
              );
            }

            if (type === 'scatter') {
              return (
                <g key={yIdx}>
                  {col.values.map((v, i) => {
                    if (v === null) return null;
                    return (
                      <circle
                        key={i}
                        cx={xAt(i)}
                        cy={yAt(v)}
                        r={3}
                        fill={color}
                        opacity={0.85}
                      />
                    );
                  })}
                </g>
              );
            }

            // Line chart (with smooth path)
            const points: string[] = [];
            col.values.forEach((v, i) => {
              if (v !== null && Number.isFinite(v)) {
                points.push(`${xAt(i).toFixed(1)},${yAt(v).toFixed(1)}`);
              }
            });

            return (
              <g key={yIdx}>
                <polyline
                  points={points.join(' ')}
                  fill="none"
                  stroke={color}
                  strokeWidth="2"
                  strokeLinejoin="round"
                />
                {n <= 40 &&
                  col.values.map((v, i) => {
                    if (v === null) return null;
                    return (
                      <circle
                        key={i}
                        cx={xAt(i)}
                        cy={yAt(v)}
                        r={2.5}
                        fill="#0d1117"
                        stroke={color}
                        strokeWidth="1.5"
                      />
                    );
                  })}
              </g>
            );
          })}

          {/* X Axis Ticks (up to 7 samples) */}
          {[0, 0.2, 0.4, 0.6, 0.8, 1].map((pct) => {
            const idx = Math.min(n - 1, Math.round(pct * (n - 1)));
            const x = xAt(idx);
            const label = xLabels[idx] ?? '';
            return (
              <text
                key={pct}
                x={x}
                y={pad.t + plotH + 16}
                textAnchor="middle"
                fontSize="9.5"
                fill="#8b949e"
              >
                {label.length > 10 ? `${label.slice(0, 8)}…` : label}
              </text>
            );
          })}
        </svg>
      </div>
    </div>
  );
};
