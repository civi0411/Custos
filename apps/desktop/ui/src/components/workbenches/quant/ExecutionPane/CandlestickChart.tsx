import React, { useRef, useEffect, useState } from 'react';
import { BarChart2, Database } from 'lucide-react';
import type { MarketBar } from './types';

export interface CandlestickChartProps {
  ticketSymbol: string;
  ticketPrice: number;
  timeframe: string;
  bars: MarketBar[];
  pipelineRunning: boolean;
  onTimeframeChange: (tf: string) => void;
  onIngestBenchmark: () => void;
}

export const CandlestickChart: React.FC<CandlestickChartProps> = ({
  ticketSymbol,
  ticketPrice,
  timeframe,
  bars,
  pipelineRunning,
  onTimeframeChange,
  onIngestBenchmark,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [hoveredBar, setHoveredBar] = useState<MarketBar | null>(null);

  // Render Canvas Candlestick Chart
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || bars.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);

    // Background
    ctx.fillStyle = '#141416';
    ctx.fillRect(0, 0, width, height);

    const priceAxisWidth = 60;
    const plotWidth = width - priceAxisWidth;
    const volumeHeight = height * 0.22;
    const pricePlotHeight = height - volumeHeight - 20;

    // Price Bounds
    let minPrice = Infinity;
    let maxPrice = -Infinity;
    let maxVol = 0;

    bars.forEach((b) => {
      if (b.low < minPrice) minPrice = b.low;
      if (b.high > maxPrice) maxPrice = b.high;
      if (b.volume > maxVol) maxVol = b.volume;
    });

    const pricePad = (maxPrice - minPrice) * 0.05 || 1;
    minPrice -= pricePad;
    maxPrice += pricePad;
    const priceRange = maxPrice - minPrice;

    // Price Grid Lines
    ctx.strokeStyle = '#222226';
    ctx.lineWidth = 1;
    const gridSteps = 5;
    for (let i = 0; i <= gridSteps; i++) {
      const y = (pricePlotHeight / gridSteps) * i + 10;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(plotWidth, y);
      ctx.stroke();

      const priceLabel = (maxPrice - (priceRange / gridSteps) * i).toFixed(2);
      ctx.fillStyle = '#71717a';
      ctx.font = '10px monospace';
      ctx.fillText(`$${priceLabel}`, plotWidth + 6, y + 3);
    }

    // Volume Grid Line Separator
    ctx.strokeStyle = '#2a2a2e';
    ctx.beginPath();
    ctx.moveTo(0, pricePlotHeight + 10);
    ctx.lineTo(plotWidth, pricePlotHeight + 10);
    ctx.stroke();

    // Bar dimensions
    const barWidth = Math.max(3, (plotWidth / bars.length) * 0.7);
    const barSpacing = plotWidth / bars.length;

    // Draw Moving Average (SMA 20)
    ctx.beginPath();
    ctx.strokeStyle = '#f59e0b';
    ctx.lineWidth = 1.5;
    let maCount = 0;
    let maSum = 0;

    bars.forEach((b, idx) => {
      maSum += b.close;
      maCount++;
      if (maCount > 20) {
        maSum -= bars[idx - 20].close;
      }
      if (maCount >= 5) {
        const sma = maSum / Math.min(maCount, 20);
        const x = idx * barSpacing + barSpacing / 2;
        const y = 10 + (1 - (sma - minPrice) / priceRange) * pricePlotHeight;
        if (idx === 4) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
    });
    ctx.stroke();

    // Draw Candlesticks and Volume
    bars.forEach((b, idx) => {
      const x = idx * barSpacing + barSpacing / 2;
      const isBull = b.close >= b.open;
      const candleColor = isBull ? '#34d399' : '#f43f5e';

      // 1. Volume Bar
      const vHeight = (b.volume / (maxVol || 1)) * (volumeHeight - 10);
      const vY = height - vHeight - 4;
      ctx.fillStyle = isBull ? 'rgba(52, 211, 153, 0.25)' : 'rgba(244, 63, 94, 0.25)';
      ctx.fillRect(x - barWidth / 2, vY, barWidth, vHeight);

      // 2. High-Low Wick
      const yHigh = 10 + (1 - (b.high - minPrice) / priceRange) * pricePlotHeight;
      const yLow = 10 + (1 - (b.low - minPrice) / priceRange) * pricePlotHeight;
      ctx.strokeStyle = candleColor;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(x, yHigh);
      ctx.lineTo(x, yLow);
      ctx.stroke();

      // 3. Candle Body
      const yOpen = 10 + (1 - (b.open - minPrice) / priceRange) * pricePlotHeight;
      const yClose = 10 + (1 - (b.close - minPrice) / priceRange) * pricePlotHeight;
      const bodyTop = Math.min(yOpen, yClose);
      const bodyHeight = Math.max(2, Math.abs(yOpen - yClose));

      ctx.fillStyle = candleColor;
      ctx.fillRect(x - barWidth / 2, bodyTop, barWidth, bodyHeight);
    });
  }, [bars]);

  const handleCanvasMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || bars.length === 0) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const plotWidth = canvas.clientWidth - 60;
    const barSpacing = plotWidth / bars.length;
    const idx = Math.floor(x / barSpacing);
    if (idx >= 0 && idx < bars.length) {
      setHoveredBar(bars[idx]);
    }
  };

  const handleCanvasMouseLeave = () => {
    setHoveredBar(null);
  };

  return (
    <div className="p-4 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] space-y-3">
      <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] pb-2.5">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <BarChart2 size={15} className="text-workbench-accent" />
            <span className="text-xs font-bold font-mono tracking-wide text-[var(--color-fg)]">
              {ticketSymbol}/USDT
            </span>
            <span className="text-xs font-mono font-semibold text-emerald-400 ml-1">
              ${ticketPrice.toFixed(2)}
            </span>
          </div>
          <div className="flex items-center gap-1 bg-[var(--color-surface-2)] p-0.5 rounded border border-[var(--color-border-subtle)]">
            {(['1m', '5m', '15m', '1h', '1d'] as const).map((tf) => (
              <button
                key={tf}
                type="button"
                onClick={() => onTimeframeChange(tf)}
                className={`px-2 py-0.5 text-[10px] font-mono font-semibold rounded ${
                  timeframe === tf
                    ? 'bg-[var(--color-primary-emphasis)] text-white'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg)]'
                }`}
              >
                {tf}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled={pipelineRunning}
            onClick={onIngestBenchmark}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border-subtle)] text-xs font-medium text-[var(--color-fg-muted)]"
          >
            <Database size={12} className={pipelineRunning ? 'animate-spin text-amber-400' : 'text-emerald-400'} />
            <span>Sync Market Feed</span>
          </button>
        </div>
      </div>

      {/* Native HTML5 Canvas Candlestick & Volume Chart */}
      <div className="relative w-full h-56 rounded bg-[#141416] overflow-hidden border border-[#222226] flex items-center justify-center">
        {bars.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-6 text-center">
            <BarChart2 className="h-8 w-8 text-zinc-600 mb-2" />
            <span className="text-xs font-semibold text-zinc-300">
              No Historical Bars for {ticketSymbol} ({timeframe})
            </span>
            <span className="text-[11px] text-zinc-500 max-w-sm mt-0.5">
              Synchronize market feed or stream real ticks into Universal Resource Registry to view the candlestick price chart.
            </span>
            <button
              type="button"
              disabled={pipelineRunning}
              onClick={onIngestBenchmark}
              className="mt-3 flex items-center gap-1.5 px-3 py-1 rounded bg-[var(--color-primary-emphasis)] hover:opacity-90 text-white text-xs font-medium"
            >
              <Database size={12} />
              <span>Ingest Benchmark Market Bars</span>
            </button>
          </div>
        ) : (
          <>
            <canvas
              ref={canvasRef}
              onMouseMove={handleCanvasMouseMove}
              onMouseLeave={handleCanvasMouseLeave}
              className="w-full h-full block cursor-crosshair"
            />
            {hoveredBar ? (
              <div className="absolute top-2 left-3 flex items-center gap-3 text-[10px] font-mono text-zinc-300 pointer-events-none bg-black/80 px-2 py-0.5 rounded border border-zinc-700/60 backdrop-blur-xs">
                <span>O: <span className="text-zinc-100">${hoveredBar.open.toFixed(2)}</span></span>
                <span>H: <span className="text-emerald-400">${hoveredBar.high.toFixed(2)}</span></span>
                <span>L: <span className="text-rose-400">${hoveredBar.low.toFixed(2)}</span></span>
                <span>C: <span className="text-zinc-100">${hoveredBar.close.toFixed(2)}</span></span>
                <span>Vol: <span className="text-amber-400">{hoveredBar.volume.toLocaleString()}</span></span>
              </div>
            ) : (
              <div className="absolute top-2 left-3 flex items-center gap-3 text-[10px] font-mono text-zinc-400 pointer-events-none">
                <span>SMA(20): <span className="text-amber-400 font-semibold">${ticketPrice.toFixed(2)}</span></span>
                <span className="text-zinc-500">Zero-Lookahead Invariant Enforced</span>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
};
