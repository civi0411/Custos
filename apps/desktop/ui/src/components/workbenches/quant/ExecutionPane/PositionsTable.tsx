import React from 'react';
import type { PositionItem } from './types';

export interface PositionsTableProps {
  positions: PositionItem[];
  onClosePosition: (pos: PositionItem) => void;
}

export const PositionsTable: React.FC<PositionsTableProps> = ({
  positions,
  onClosePosition,
}) => {
  return (
    <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] overflow-hidden">
      <div className="px-4 py-2.5 border-b border-[var(--color-border-subtle)] flex items-center justify-between">
        <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
          Active Positions Ledger ({positions.length})
        </span>
        <span className="text-[11px] font-mono text-emerald-400">
          Total Unrealized PnL: +${positions.reduce((acc, p) => acc + p.unrealizedPnl, 0).toLocaleString()}
        </span>
      </div>
      {positions.length > 0 ? (
        <table className="w-full text-left text-xs">
          <thead className="border-b border-[var(--color-border-subtle)] text-[var(--color-fg-muted)]">
            <tr>
              <th className="px-4 py-2 font-medium">Symbol</th>
              <th className="px-4 py-2 font-medium">Side</th>
              <th className="px-4 py-2 font-medium">Quantity</th>
              <th className="px-4 py-2 font-medium">Entry Price</th>
              <th className="px-4 py-2 font-medium">Mark Price</th>
              <th className="px-4 py-2 font-medium">Unrealized PnL</th>
              <th className="px-4 py-2 font-medium text-right">Action</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[var(--color-border-subtle)]">
            {positions.map((p) => (
              <tr key={p.symbol} className="hover:bg-[var(--color-surface-2)]">
                <td className="px-4 py-2 font-mono font-semibold">{p.symbol}</td>
                <td className="px-4 py-2">
                  <span
                    className={`text-[10px] font-mono uppercase px-1.5 py-0.5 rounded ${
                      p.direction === 'long'
                        ? 'text-emerald-400 bg-emerald-500/10'
                        : 'text-rose-400 bg-rose-500/10'
                    }`}
                  >
                    {p.direction}
                  </span>
                </td>
                <td className="px-4 py-2 font-mono">{p.quantity}</td>
                <td className="px-4 py-2 font-mono">${p.entryPrice.toFixed(2)}</td>
                <td className="px-4 py-2 font-mono">${p.markPrice.toFixed(2)}</td>
                <td
                  className={`px-4 py-2 font-mono font-semibold ${
                    p.unrealizedPnl >= 0 ? 'text-emerald-400' : 'text-rose-400'
                  }`}
                >
                  {p.unrealizedPnl >= 0
                    ? `+$${p.unrealizedPnl.toFixed(2)}`
                    : `-$${Math.abs(p.unrealizedPnl).toFixed(2)}`}
                </td>
                <td className="px-4 py-2 text-right">
                  <button
                    type="button"
                    onClick={() => onClosePosition(p)}
                    className="px-2 py-1 rounded bg-rose-500/10 border border-rose-500/30 text-rose-400 hover:bg-rose-500/20 text-[11px] font-medium"
                  >
                    Close Market
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div className="p-4 text-center text-xs text-[var(--color-fg-muted)]">
          No open positions active.
        </div>
      )}
    </div>
  );
};
