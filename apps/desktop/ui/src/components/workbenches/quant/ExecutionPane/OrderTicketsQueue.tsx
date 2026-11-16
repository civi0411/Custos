import React from 'react';
import { CheckCircle2 } from 'lucide-react';
import { WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import type { OrderTicketItem } from './types';

export interface OrderTicketsQueueProps {
  orders: OrderTicketItem[];
  onApproveOrder: (id: string) => void;
  onCreateOrder: () => void;
}

export const OrderTicketsQueue: React.FC<OrderTicketsQueueProps> = ({
  orders,
  onApproveOrder,
  onCreateOrder,
}) => {
  return (
    <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] overflow-hidden">
      <div className="px-4 py-2.5 border-b border-[var(--color-border-subtle)] flex items-center justify-between">
        <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
          Governed Order Tickets ({orders.length})
        </span>
        <span className="text-[11px] font-mono text-[var(--color-fg-muted)]">Human-in-the-Loop Gate</span>
      </div>
      {orders.length > 0 ? (
        <div className="divide-y divide-[var(--color-border-subtle)]">
          {orders.map((ord) => (
            <div key={ord.id} className="p-3 flex items-center justify-between hover:bg-[var(--color-surface-2)]">
              <div>
                <div className="flex items-center gap-2">
                  <span className="text-xs font-mono font-semibold text-[var(--color-fg)]">{ord.id}</span>
                  <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-black/20 text-emerald-400">
                    {ord.direction} {ord.quantity} {ord.symbol}
                  </span>
                  <span
                    className={`text-[10px] font-mono uppercase px-1.5 py-0.5 rounded ${
                      ord.status === 'approved'
                        ? 'text-emerald-400 bg-emerald-500/10'
                        : 'text-amber-400 bg-amber-500/10'
                    }`}
                  >
                    {ord.status}
                  </span>
                </div>
                <div className="text-[11px] text-[var(--color-fg-muted)] mt-1 font-mono">
                  Strategy: {ord.strategy_id} · Weight: {(ord.target_weight * 100).toFixed(1)}%
                </div>
              </div>
              {ord.status !== 'approved' && ord.status !== 'cancelled' ? (
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => onApproveOrder(ord.id)}
                    className="flex items-center gap-1 px-2.5 py-1 rounded bg-emerald-500 hover:bg-emerald-600 text-white text-xs font-semibold"
                  >
                    <CheckCircle2 size={12} />
                    <span>Approve Ticket</span>
                  </button>
                </div>
              ) : (
                <span className="text-xs font-mono text-[var(--color-fg-muted)] capitalize">{ord.status}</span>
              )}
            </div>
          ))}
        </div>
      ) : (
        <div className="p-4">
          <WorkbenchEmptyState
            title="No Pending Order Tickets"
            description="Draft order tickets manually above or trigger factor backtest execution to generate tickets."
            action={
              <button
                type="button"
                onClick={onCreateOrder}
                className="px-3 py-1.5 rounded-md bg-emerald-500 text-white text-xs font-semibold hover:bg-emerald-600"
              >
                Draft Sample Order
              </button>
            }
          />
        </div>
      )}
    </div>
  );
};
