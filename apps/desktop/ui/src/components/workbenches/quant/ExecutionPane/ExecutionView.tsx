import { useState, useEffect, useMemo, useCallback } from 'react';
import { Shield, RotateCw, Play } from 'lucide-react';
import { WorkbenchPane } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import { quantSelectionStore, useQuantSelection } from '../quantSelectionStore';
import type { ExecutionPaneProps, OrderTicketItem, PositionItem, MarketBar, OrderBookLevel } from './types';
import { VERIFIED_HISTORICAL_BENCHMARK_BARS } from '../quantBenchmarkData';
import { CandlestickChart } from './CandlestickChart';
import { OrderBookLadder } from './OrderBookLadder';
import { OrderTicketForm } from './OrderTicketForm';
import { PositionsTable } from './PositionsTable';
import { OrderTicketsQueue } from './OrderTicketsQueue';

export function ExecutionView({ sessionId, onShowToast }: ExecutionPaneProps) {
  const selection = useQuantSelection();
  const activeTaskId = sessionId || 'quant_session_active';
  const activeStrategyId = selection.strategyId || 'strat_manual_discretionary';
  const [loading, setLoading] = useState<boolean>(false);
  const [orders, setOrders] = useState<OrderTicketItem[]>([]);
  const [positions, setPositions] = useState<PositionItem[]>([]);

  // Order Ticket Form State
  const [ticketSymbol, setTicketSymbol] = useState<string>(selection.symbol || 'SPY');
  const [ticketDirection, setTicketDirection] = useState<'buy' | 'sell' | 'sell_short' | 'buy_to_cover'>('buy');
  const [ticketQty, setTicketQty] = useState<number>(10);
  const [ticketPrice, setTicketPrice] = useState<number>(500.0);
  const [gateVerdict, setGateVerdict] = useState<string | null>(null);
  const [serverOrderBook, setServerOrderBook] = useState<{
    symbol: string;
    mid_price: number;
    spread: number;
    spread_bps: number;
    microprice: number;
    imbalance_pct: number;
    data_mode?: string;
    asks: Array<{ price: number; size: number; total: number }>;
    bids: Array<{ price: number; size: number; total: number }>;
    timestamp: number;
  } | null>(null);

  // Candlestick Chart State
  const [timeframe, setTimeframe] = useState<string>(selection.timeframe || '1d');
  const [bars, setBars] = useState<MarketBar[]>([]);

  // Ingesting pipeline modal/drawer
  const [pipelineRunning, setPipelineRunning] = useState<boolean>(false);

  // Generate or load market bars for chart
  const loadMarketBars = useCallback(async (sym: string, tf: string) => {
    try {
      const res = await daemonClient.getQuantMarketBars({ symbol: sym, timeframe: tf });
      if (res && res.bars && res.bars.length > 0) {
        setBars(res.bars);
        const lastClose = res.bars[res.bars.length - 1]?.close;
        if (typeof lastClose === 'number' && Number.isFinite(lastClose) && lastClose > 0) {
          setTicketPrice(Number(lastClose.toFixed(2)));
        }
        return;
      }
    } catch {
      // Ignored
    }
    setBars([]);
  }, []);

  useEffect(() => {
    if (selection.symbol && selection.symbol !== ticketSymbol) {
      setTicketSymbol(selection.symbol);
    }
    if (selection.timeframe && selection.timeframe !== timeframe) {
      setTimeframe(selection.timeframe);
    }
  }, [selection.symbol, selection.timeframe]);

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
      if (!document.hidden) {
        void fetchOrderBook(ticketSymbol);
      }
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

  const latestBarClose = bars.length > 0 ? bars[bars.length - 1]?.close : undefined;

  const fetchOrdersAndPositions = useCallback(async () => {
    setLoading(true);
    try {
      const [ordersRes, posRes] = await Promise.all([
        daemonClient.listQuantOrders({}),
        daemonClient.listQuantPositions({}),
      ]);
      if (ordersRes?.orders) {
        setOrders(ordersRes.orders);
      }
      if (posRes?.positions) {
        setPositions(
          posRes.positions.map((p) => {
            const markPrice =
              p.symbol === ticketSymbol && typeof latestBarClose === 'number' && latestBarClose > 0
                ? latestBarClose
                : p.avg_entry_price;
            const sign = p.direction === 'short' ? -1 : 1;
            const unrealizedPnl = (markPrice - p.avg_entry_price) * p.quantity * sign;
            return {
              symbol: p.symbol,
              direction: p.direction === 'short' ? 'short' : 'long',
              quantity: p.quantity,
              entryPrice: p.avg_entry_price,
              markPrice,
              unrealizedPnl,
            };
          })
        );
      }
    } catch {
      // Clean fallback if daemon has no stored orders/positions yet
    } finally {
      setLoading(false);
    }
  }, [ticketSymbol, latestBarClose]);

  useEffect(() => {
    void fetchOrdersAndPositions();
  }, [fetchOrdersAndPositions, selection.revision]);

  const handleEvaluateGate = async () => {
    try {
      onShowToast?.(`Checking Pre-Trade Risk Gate for ${ticketSymbol}...`);
      const res = await daemonClient.evaluateQuantGate({
        symbol: ticketSymbol,
        direction: ticketDirection,
        estimated_price: ticketPrice,
        quantity: ticketQty,
        daily_executed_notional: 0,
        current_drawdown: 0,
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
        strategy_id: activeStrategyId,
        task_id: activeTaskId,
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
        await fetchOrdersAndPositions();
        quantSelectionStore.notifyMutation();
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
      const approvedOrder = orders.find((o) => o.id === orderId);
      const execPrice =
        approvedOrder && approvedOrder.symbol === ticketSymbol && typeof latestBarClose === 'number'
          ? latestBarClose
          : ticketPrice;
      const res = await daemonClient.approveQuantOrder({
        order_id: orderId,
        permit_id: 'auto_issue',
        task_id: activeTaskId,
        execute_paper: true,
        execution_price: execPrice,
      });
      await fetchOrdersAndPositions();
      quantSelectionStore.notifyMutation();
      onShowToast?.(`Order ${orderId} settled (${res.status}) with permit ${res.permit_id}`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Approval failed: ${msg}`);
    }
  };

  const handleClosePosition = async (pos: PositionItem) => {
    try {
      onShowToast?.(`Closing position ${pos.symbol} (Exit Immunity Active)...`);
      const res = await daemonClient.closeQuantPosition({
        strategy_id: activeStrategyId,
        task_id: activeTaskId,
        symbol: pos.symbol,
        direction: pos.direction,
        quantity: pos.quantity,
        mark_price: pos.markPrice,
        permit_id: 'auto_issue',
        execute_paper: true,
      });
      await fetchOrdersAndPositions();
      quantSelectionStore.notifyMutation();
      onShowToast?.(
        `Closed ${pos.symbol} (${res.gate_outcome?.status ?? 'BypassExitImmunity'}) — order ${res.ticket?.id ?? 'queued'} (${res.ticket?.status ?? 'filled'})`
      );
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
      await fetchOrdersAndPositions();
      quantSelectionStore.notifyMutation();
      onShowToast?.(`EMERGENCY KILL-SWITCH: Cancelled ${res.cancelled_orders_count} pending orders`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Emergency kill-switch failed: ${msg}`);
    }
  };

  const circuitBreakers = [
    { rule: 'Max Single Order Notional', threshold: '$200,000', action: 'Enforced by PreTradeDecisionGate' },
    { rule: 'Max Daily Executed Notional', threshold: '$1,000,000', action: 'Summed from live SQLite fills' },
    { rule: 'Portfolio Max Drawdown Stop', threshold: '5.0%', action: 'Reject new entries when breached' },
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
              onClick={() => void fetchOrdersAndPositions()}
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
            dataMode={serverOrderBook?.data_mode}
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
