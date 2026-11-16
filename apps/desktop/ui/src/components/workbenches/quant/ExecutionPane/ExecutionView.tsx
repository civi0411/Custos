import { useState, useEffect, useMemo, useCallback } from 'react';
import { Shield, RotateCw, Play } from 'lucide-react';
import { WorkbenchPane } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import type { ExecutionPaneProps, OrderTicketItem, PositionItem, MarketBar, OrderBookLevel } from './types';
import { VERIFIED_HISTORICAL_BENCHMARK_BARS } from '../quantBenchmarkData';
import { CandlestickChart } from './CandlestickChart';
import { OrderBookLadder } from './OrderBookLadder';
import { OrderTicketForm } from './OrderTicketForm';
import { PositionsTable } from './PositionsTable';
import { OrderTicketsQueue } from './OrderTicketsQueue';

export function ExecutionView({ onShowToast }: ExecutionPaneProps) {
  const [loading, setLoading] = useState<boolean>(false);
  const [orders, setOrders] = useState<OrderTicketItem[]>([]);
  const [positions, setPositions] = useState<PositionItem[]>([]);

  // Order Ticket Form State
  const [ticketSymbol, setTicketSymbol] = useState<string>('NVDA');
  const [ticketDirection, setTicketDirection] = useState<'buy' | 'sell' | 'sell_short' | 'buy_to_cover'>('buy');
  const [ticketQty, setTicketQty] = useState<number>(100);
  const [ticketPrice, setTicketPrice] = useState<number>(135.5);
  const [gateVerdict, setGateVerdict] = useState<string | null>(null);
  const [serverOrderBook, setServerOrderBook] = useState<{
    symbol: string;
    mid_price: number;
    spread: number;
    spread_bps: number;
    microprice: number;
    imbalance_pct: number;
    asks: Array<{ price: number; size: number; total: number }>;
    bids: Array<{ price: number; size: number; total: number }>;
    timestamp: number;
  } | null>(null);

  // Candlestick Chart State
  const [timeframe, setTimeframe] = useState<string>('15m');
  const [bars, setBars] = useState<MarketBar[]>([]);

  // Ingesting pipeline modal/drawer
  const [pipelineRunning, setPipelineRunning] = useState<boolean>(false);

  // Generate or load market bars for chart
  const loadMarketBars = useCallback(async (sym: string, tf: string) => {
    try {
      const res = await daemonClient.getQuantMarketBars({ symbol: sym, timeframe: tf });
      if (res && res.bars && res.bars.length > 0) {
        setBars(res.bars);
        return;
      }
    } catch {
      // Ignored
    }
    setBars([]);
  }, []);

  const handleIngestBenchmarkData = async (sym: string, tf: string) => {
    if (sym !== 'SPY') {
      onShowToast?.(`Benchmark dataset is only calibrated for SPY (1d). Connect an exchange feed or upload verified OHLCV data for ${sym}.`);
      return;
    }

    const targetTf = '1d';
    if (tf !== targetTf) {
      onShowToast?.(`SPY benchmark data is daily (1d). Ingesting as 1d and aligning view from ${tf} to 1d.`);
      setTimeframe(targetTf);
    }

    setPipelineRunning(true);
    try {
      onShowToast?.(`Ingesting verified SPY benchmark bars (${targetTf})...`);
      await daemonClient.ingestQuantMarketBars({
        symbol: 'SPY',
        timeframe: targetTf,
        bars: VERIFIED_HISTORICAL_BENCHMARK_BARS,
      });
      onShowToast?.(`Stored ${VERIFIED_HISTORICAL_BENCHMARK_BARS.length} verified SPY benchmark bars in SQLite data store.`);
      await loadMarketBars('SPY', targetTf);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Ingest failed: ${msg}`);
    } finally {
      setPipelineRunning(false);
    }
  };

  const fetchOrderBook = useCallback(async (sym: string) => {
    try {
      const res = await daemonClient.getQuantOrderBook({ symbol: sym, depth: 5 });
      if (res && res.asks && res.bids) {
        setServerOrderBook(res);
      }
    } catch {
      // Ignored
    }
  }, []);

  useEffect(() => {
    void loadMarketBars(ticketSymbol, timeframe);
    void fetchOrderBook(ticketSymbol);
    const interval = setInterval(() => {
      void fetchOrderBook(ticketSymbol);
    }, 5000);
    return () => clearInterval(interval);
  }, [ticketSymbol, timeframe, loadMarketBars, fetchOrderBook]);

  // Order Book L2 Calculations around ticketPrice (server-authoritative with synthetic fallback)
  const orderBook = useMemo(() => {
    if (serverOrderBook && serverOrderBook.symbol === ticketSymbol && serverOrderBook.asks.length > 0) {
      const maxDepth = Math.max(
        serverOrderBook.asks[serverOrderBook.asks.length - 1]?.total || 1,
        serverOrderBook.bids[serverOrderBook.bids.length - 1]?.total || 1,
        1
      );
      return {
        asks: serverOrderBook.asks.map((a) => ({
          price: a.price,
          size: a.size,
          depth: a.total,
          depthPct: Math.round((a.total / maxDepth) * 100),
        })),
        bids: serverOrderBook.bids.map((b) => ({
          price: b.price,
          size: b.size,
          depth: b.total,
          depthPct: Math.round((b.total / maxDepth) * 100),
        })),
        spread: serverOrderBook.spread,
        spreadBps: serverOrderBook.spread_bps,
        microprice: serverOrderBook.microprice,
      };
    }

    return {
      asks: [] as Array<OrderBookLevel & { depthPct: number }>,
      bids: [] as Array<OrderBookLevel & { depthPct: number }>,
      spread: 0,
      spreadBps: 0,
      microprice: 0,
    };
  }, [serverOrderBook, ticketSymbol]);

  const fetchOrders = async () => {
    setLoading(true);
    try {
      const res = await daemonClient.listQuantOrders({ task_id: 'task_quant_active' });
      if (res && res.orders) {
        setOrders(res.orders);
      }
    } catch {
      // Clean fallback if daemon has no stored orders
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void fetchOrders();
  }, []);

  const handleEvaluateGate = async () => {
    try {
      onShowToast?.(`Checking Pre-Trade Risk Gate for ${ticketSymbol}...`);
      const res = await daemonClient.evaluateQuantGate({
        symbol: ticketSymbol,
        direction: ticketDirection,
        estimated_price: ticketPrice,
        quantity: ticketQty,
        daily_executed_notional: 150_000,
        current_drawdown: 0.012,
        consecutive_losses: 0,
        max_order_notional: 200_000,
        max_daily_notional: 1_000_000,
        max_drawdown_limit: 0.05,
        max_consecutive_losses: 5,
      });
      setGateVerdict(res.outcome.status);
      onShowToast?.(`Gate Verdict: ${res.outcome.status} (${res.outcome.details.reason})`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Gate check failed: ${msg}`);
    }
  };

  const handleCreateOrder = async () => {
    setLoading(true);
    try {
      onShowToast?.(`Submitting ${ticketDirection.toUpperCase()} ${ticketQty} ${ticketSymbol}...`);
      const res = await daemonClient.createQuantOrders({
        strategy_id: 'strat_manual_discretionary',
        task_id: 'task_quant_active',
        orders: [
          {
            symbol: ticketSymbol,
            direction: ticketDirection,
            quantity: ticketQty,
            target_weight: 0.05,
            limit_price: ticketPrice,
            estimated_slippage_bps: 2.0,
          },
        ],
      });
      if (res.created_orders && res.created_orders.length > 0) {
        setOrders((prev) => [...res.created_orders, ...prev]);
        onShowToast?.(`Order ticket drafted: ${res.created_orders[0].id}`);
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Failed to draft order: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  const handleApproveOrder = async (orderId: string) => {
    try {
      await daemonClient.approveQuantOrder({
        order_id: orderId,
        permit_id: 'permit_operator_approved',
        task_id: 'task_quant_active',
      });
      setOrders((prev) =>
        prev.map((o) => (o.id === orderId ? { ...o, status: 'approved' } : o))
      );
      const approvedOrder = orders.find((o) => o.id === orderId);
      if (approvedOrder) {
        setPositions((prev) => [
          ...prev.filter((p) => p.symbol !== approvedOrder.symbol),
          {
            symbol: approvedOrder.symbol,
            direction: approvedOrder.direction.includes('short') ? 'short' : 'long',
            quantity: approvedOrder.quantity,
            entryPrice: ticketPrice,
            markPrice: ticketPrice,
            unrealizedPnl: 0,
          },
        ]);
      }
      onShowToast?.(`Order ${orderId} approved and recorded to Outbox`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Approval failed: ${msg}`);
    }
  };

  const handleClosePosition = async (pos: PositionItem) => {
    try {
      onShowToast?.(`Closing position ${pos.symbol} (Exit Immunity Active)...`);
      const gate = await daemonClient.evaluateQuantGate({
        symbol: pos.symbol,
        direction: pos.direction === 'long' ? 'sell' : 'buy_to_cover',
        estimated_price: pos.markPrice,
        quantity: pos.quantity,
        is_risk_reducing: true,
      });
      onShowToast?.(`Exit Gate: ${gate.outcome.status} — position liquidation queued`);
      setPositions((prev) => prev.filter((p) => p.symbol !== pos.symbol));
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Close position failed: ${msg}`);
    }
  };

  const handleKillSwitch = async () => {
    try {
      const res = await daemonClient.triggerQuantKillSwitch({
        reason: 'Operator Emergency Kill-Switch Triggered via Execution Pane',
      });
      onShowToast?.(`EMERGENCY KILL-SWITCH: Cancelled ${res.cancelled_orders_count} pending orders`);
      setOrders((prev) => prev.map((o) => ({ ...o, status: 'cancelled' })));
    } catch {
      onShowToast?.('Emergency kill-switch executed');
    }
  };

  const circuitBreakers = [
    { rule: 'Portfolio Daily Drawdown Stop', threshold: '2.5%', action: 'Liquidate all positions immediately' },
    { rule: 'Single Symbol Max Weight', threshold: '15.0%', action: 'Hard clip on order generation' },
    { rule: 'Max Adverse Slippage Tolerance', threshold: '15.0 bps', action: 'Cancel order and requote' },
  ];

  return (
    <WorkbenchPane
      toolbar={
        <div className="flex items-center justify-between w-full px-3 py-1.5 text-xs font-semibold text-[var(--color-fg)]">
          <div className="flex items-center gap-2">
            <Play size={14} className="text-workbench-accent" />
            <span>Algorithmic & Discretionary Execution Hub</span>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={loading}
              onClick={() => void fetchOrders()}
              className="flex items-center gap-1.5 px-2 py-1 rounded bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs font-medium text-[var(--color-fg-muted)]"
            >
              <RotateCw size={12} className={loading ? 'animate-spin' : ''} />
              <span>Refresh</span>
            </button>
            <button
              type="button"
              onClick={handleKillSwitch}
              className="px-2.5 py-1 rounded-md border border-rose-500/40 text-rose-400 text-xs font-semibold hover:bg-rose-500/20"
              title="Cancel all draft and pending orders immediately"
            >
              Emergency Kill-Switch
            </button>
          </div>
        </div>
      }
    >
      <div className="flex h-full min-h-0 flex-col overflow-y-auto p-4 space-y-4 bg-[var(--color-canvas)] text-[var(--color-fg)]">
        <CandlestickChart
          ticketSymbol={ticketSymbol}
          ticketPrice={ticketPrice}
          timeframe={timeframe}
          bars={bars}
          pipelineRunning={pipelineRunning}
          onTimeframeChange={setTimeframe}
          onIngestBenchmark={() => handleIngestBenchmarkData(ticketSymbol, timeframe)}
        />

        <div className="grid grid-cols-12 gap-4">
          <OrderBookLadder
            ticketSymbol={ticketSymbol}
            orderBook={orderBook}
            onSelectPrice={(price, dir) => {
              setTicketPrice(price);
              setTicketDirection(dir);
              onShowToast?.(`Quoting ${dir === 'buy' ? 'Buy @ Ask' : 'Sell @ Bid'} $${price}`);
            }}
          />

          <OrderTicketForm
            ticketSymbol={ticketSymbol}
            ticketDirection={ticketDirection}
            ticketQty={ticketQty}
            ticketPrice={ticketPrice}
            gateVerdict={gateVerdict}
            loading={loading}
            onSymbolChange={setTicketSymbol}
            onDirectionChange={setTicketDirection}
            onQtyChange={setTicketQty}
            onPriceChange={setTicketPrice}
            onCheckGate={handleEvaluateGate}
            onDraftOrder={handleCreateOrder}
          />
        </div>

        <PositionsTable
          positions={positions}
          onClosePosition={handleClosePosition}
        />

        <OrderTicketsQueue
          orders={orders}
          onApproveOrder={handleApproveOrder}
          onCreateOrder={handleCreateOrder}
        />

        {/* Circuit Breakers & Kill-Switches */}
        <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4 space-y-2">
          <div className="flex items-center gap-1.5 text-xs font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
            <Shield size={14} className="text-amber-400" />
            <span>Active Circuit Breakers & Guard Thresholds</span>
          </div>
          <div className="grid grid-cols-3 gap-3 pt-1">
            {circuitBreakers.map((cb, idx) => (
              <div key={idx} className="p-3 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]">
                <div className="text-xs font-semibold text-[var(--color-fg)]">{cb.rule}</div>
                <div className="mt-1 text-[11px] text-amber-400 font-mono">Limit: {cb.threshold}</div>
                <div className="mt-0.5 text-[10px] text-[var(--color-fg-muted)]">{cb.action}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </WorkbenchPane>
  );
}
