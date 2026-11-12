import React from 'react';
import { Plus } from 'lucide-react';

export interface OrderTicketFormProps {
  ticketSymbol: string;
  ticketDirection: 'buy' | 'sell' | 'sell_short' | 'buy_to_cover';
  ticketQty: number;
  ticketPrice: number;
  gateVerdict: string | null;
  loading: boolean;
  onSymbolChange: (v: string) => void;
  onDirectionChange: (v: 'buy' | 'sell' | 'sell_short' | 'buy_to_cover') => void;
  onQtyChange: (v: number) => void;
  onPriceChange: (v: number) => void;
  onCheckGate: () => void;
  onDraftOrder: () => void;
}

export const OrderTicketForm: React.FC<OrderTicketFormProps> = ({
  ticketSymbol,
  ticketDirection,
  ticketQty,
  ticketPrice,
  gateVerdict,
  loading,
  onSymbolChange,
  onDirectionChange,
  onQtyChange,
  onPriceChange,
  onCheckGate,
  onDraftOrder,
}) => {
  return (
    <div className="col-span-7 p-4 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] space-y-3">
      <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] pb-2">
        <div className="flex items-center gap-2">
          <Plus size={14} className="text-emerald-400" />
          <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg)]">
            Order Ticket Entry (Discretionary & Algo)
          </span>
        </div>
        {gateVerdict && (
          <span
            className={`text-[10px] font-mono font-semibold px-2 py-0.5 rounded border ${
              gateVerdict === 'Pass' || gateVerdict === 'BypassExitImmunity'
                ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'
                : 'border-rose-500/30 bg-rose-500/10 text-rose-400'
            }`}
          >
            Gate: {gateVerdict}
          </span>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-[10px] font-semibold text-[var(--color-fg-muted)] block mb-1">Symbol</label>
          <input
            type="text"
            value={ticketSymbol}
            onChange={(e) => onSymbolChange(e.target.value.toUpperCase())}
            className="w-full px-2.5 py-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-xs font-mono font-medium text-[var(--color-fg)] focus:outline-none focus:border-emerald-500"
          />
        </div>
        <div>
          <label className="text-[10px] font-semibold text-[var(--color-fg-muted)] block mb-1">Direction</label>
          <select
            value={ticketDirection}
            onChange={(e) => onDirectionChange(e.target.value as any)}
            className="w-full px-2.5 py-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-xs font-medium text-[var(--color-fg)] focus:outline-none focus:border-emerald-500"
          >
            <option value="buy">Buy (Long)</option>
            <option value="sell">Sell (Close Long)</option>
            <option value="sell_short">Sell Short</option>
            <option value="buy_to_cover">Buy to Cover</option>
          </select>
        </div>
        <div>
          <label className="text-[10px] font-semibold text-[var(--color-fg-muted)] block mb-1">Quantity</label>
          <input
            type="number"
            value={ticketQty}
            onChange={(e) => onQtyChange(Number(e.target.value))}
            className="w-full px-2.5 py-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-xs font-mono font-medium text-[var(--color-fg)] focus:outline-none focus:border-emerald-500"
          />
        </div>
        <div>
          <label className="text-[10px] font-semibold text-[var(--color-fg-muted)] block mb-1">Limit Price ($)</label>
          <input
            type="number"
            step="0.01"
            value={ticketPrice}
            onChange={(e) => onPriceChange(Number(e.target.value))}
            className="w-full px-2.5 py-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-subtle)] text-xs font-mono font-medium text-[var(--color-fg)] focus:outline-none focus:border-emerald-500"
          />
        </div>
      </div>

      <div className="flex items-center justify-between pt-2">
        <span className="text-xs font-mono text-[var(--color-fg-muted)]">
          Est. Notional: ${(ticketQty * ticketPrice).toLocaleString()} · Algorithm: Limit-Then-Market
        </span>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onCheckGate}
            className="px-3 py-1.5 rounded-md border border-[var(--color-border-subtle)] text-xs font-medium text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"
          >
            Check Gate
          </button>
          <button
            type="button"
            disabled={loading}
            onClick={onDraftOrder}
            className="px-3 py-1.5 rounded-md bg-emerald-500 hover:bg-emerald-600 text-white text-xs font-semibold shadow-xs"
          >
            Draft Order Ticket
          </button>
        </div>
      </div>
    </div>
  );
};
