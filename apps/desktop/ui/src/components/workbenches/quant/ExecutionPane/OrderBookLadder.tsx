import React from 'react';
import { Layers } from 'lucide-react';

export interface OrderBookLadderProps {
  ticketSymbol: string;
  orderBook: {
    asks: Array<{ price: number; size: number; depth: number; depthPct: number }>;
    bids: Array<{ price: number; size: number; depth: number; depthPct: number }>;
    spread: number;
    spreadBps: number;
    microprice: number;
  };
  onSelectPrice: (price: number, direction: 'buy' | 'sell') => void;
}

export const OrderBookLadder: React.FC<OrderBookLadderProps> = ({
  ticketSymbol,
  orderBook,
  onSelectPrice,
}) => {
  return (
    <div className="col-span-5 p-4 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] space-y-2.5">
      <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] pb-2">
        <div className="flex items-center gap-1.5">
          <Layers size={13} className="text-cyan-400" />
          <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg)]">
            L2 Order Book ({ticketSymbol})
          </span>
        </div>
        <span className="text-[10px] font-mono text-[var(--color-fg-muted)]">Click level to quote</span>
      </div>

      {/* Asks (Red) */}
      <div className="space-y-0.5 text-xs font-mono">
        {orderBook.asks.length === 0 && orderBook.bids.length === 0 && (
          <div className="px-2 py-3 text-center text-[11px] text-[var(--color-fg-muted)]">
            No market bars ingested for {ticketSymbol}. Ingest OHLCV data to compute L2 depth.
          </div>
        )}
        {orderBook.asks.map((ask, idx) => (
          <div
            key={`ask-${idx}`}
            onClick={() => onSelectPrice(ask.price, 'buy')}
            className="relative flex items-center justify-between px-2 py-0.5 rounded hover:bg-rose-500/10 cursor-pointer overflow-hidden group"
          >
            <div
              className="absolute right-0 top-0 bottom-0 bg-rose-500/10 pointer-events-none"
              style={{ width: `${ask.depthPct}%` }}
            />
            <span className="relative z-10 text-rose-400 font-semibold">${ask.price.toFixed(2)}</span>
            <span className="relative z-10 text-[var(--color-fg-muted)]">{ask.size}</span>
          </div>
        ))}
      </div>

      {/* Spread Row */}
      <div className="py-1 px-2 border-y border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] flex items-center justify-between text-[11px] font-mono text-[var(--color-fg-muted)]">
        <span>Spread: ${orderBook.spread.toFixed(2)} ({orderBook.spreadBps.toFixed(1)} bps)</span>
        <span className="text-[10px]">Micro: ${orderBook.microprice.toFixed(2)}</span>
      </div>

      {/* Bids (Green) */}
      <div className="space-y-0.5 text-xs font-mono">
        {orderBook.bids.map((bid, idx) => (
          <div
            key={`bid-${idx}`}
            onClick={() => onSelectPrice(bid.price, 'sell')}
            className="relative flex items-center justify-between px-2 py-0.5 rounded hover:bg-emerald-500/10 cursor-pointer overflow-hidden group"
          >
            <div
              className="absolute right-0 top-0 bottom-0 bg-emerald-500/10 pointer-events-none"
              style={{ width: `${bid.depthPct}%` }}
            />
            <span className="relative z-10 text-emerald-400 font-semibold">${bid.price.toFixed(2)}</span>
            <span className="relative z-10 text-[var(--color-fg-muted)]">{bid.size}</span>
          </div>
        ))}
      </div>
    </div>
  );
};
