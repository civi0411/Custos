import { useState, useEffect, useCallback } from 'react';
import { ShieldCheck, CheckCircle2, AlertTriangle, XCircle, RotateCw } from 'lucide-react';
import { WorkbenchPane, WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import { useQuantSelection } from './quantSelectionStore';
import type { OrcaTabId } from '../workbenchResources';

interface RiskAuditPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

interface GateOutcomeView {
  is_allowed: boolean;
  outcome: {
    status: 'Pass' | 'Reject' | 'BypassExitImmunity' | 'FailOpen';
    details: {
      reason: string;
      order_notional?: number;
      breach_type?: string;
    };
  };
  order_notional: number;
  symbol: string;
}

interface ReviewReportView {
  strategy_id: string;
  symbol: string;
  total_trades: number;
  gross_pnl: number;
  total_commission: number;
  net_pnl: number;
  profit_factor: number;
  win_loss_ratio: number;
  holding_cohorts: Array<{
    label: string;
    trade_count: number;
    win_count: number;
    win_rate: number;
    gross_pnl: number;
    net_pnl: number;
    total_commission: number;
  }>;
  exit_cohorts: Array<{
    reason: string;
    count: number;
    net_pnl: number;
  }>;
  diagnostics: Array<{
    code: string;
    severity: string;
    summary: string;
    recommendation: string;
  }>;
}

interface PboResultView {
  available: boolean;
  probability: number;
  samples: number;
  median_logit: number;
  is_overfitted: boolean;
  status: string;
}

export function RiskAuditPane({ onShowToast, onOpenTab }: RiskAuditPaneProps) {
  const selection = useQuantSelection();
  const [loading, setLoading] = useState<boolean>(false);
  const [gateResult, setGateResult] = useState<GateOutcomeView | null>(null);
  const [reviewReport, setReviewReport] = useState<ReviewReportView | null>(null);
  const [pboResult, setPboResult] = useState<PboResultView | null>(null);
  const [pboNotice, setPboNotice] = useState<string>('');

  const runFullAudit = useCallback(async () => {
    setLoading(true);
    try {
      const sym = selection.symbol || 'SPY';
      const tf = selection.timeframe || '1d';
      const stratId = selection.strategyId || '';

      // 1. Pre-trade decision gate using real latest ingested bar price + real SQLite daily notional
      try {
        const barsRes = await daemonClient.getQuantMarketBars({ symbol: sym, timeframe: tf, limit: 1 });
        const latestPrice = barsRes?.bars?.[barsRes.bars.length - 1]?.close;
        if (latestPrice && latestPrice > 0) {
          const gate = await daemonClient.evaluateQuantGate({
            symbol: sym,
            direction: 'buy',
            estimated_price: latestPrice,
            quantity: 10,
            daily_executed_notional: 0, // Daemon resolves real daily executed notional from quant_fills
            current_drawdown: 0,
            consecutive_losses: 0,
          });
          setGateResult(gate);
        } else {
          setGateResult(null);
        }
      } catch {
        setGateResult(null);
      }

      // 2. Post-trade FIFO review strictly from persisted quant_fills in SQLite (Zero-Mock)
      if (stratId) {
        try {
          const review = await daemonClient.computeQuantReview({
            strategy_id: stratId,
            symbol: 'ALL',
            fills: [],
          });
          setReviewReport(review);
        } catch {
          setReviewReport(null);
        }
      } else {
        setReviewReport(null);
      }

      // 3. Real CSCV PBO computed from persisted backtest equity curves across registered strategies
      try {
        const list = await daemonClient.listQuantStrategies();
        const strategies = list?.strategies ?? [];
        const returnRows: number[][] = [];
        for (const s of strategies.slice(0, 8)) {
          try {
            const sheet = await daemonClient.getQuantTearsheet(s.id);
            const curve = sheet?.equity_curve ?? [];
            if (curve.length >= 6) {
              const rets: number[] = [];
              for (let i = 1; i < curve.length; i++) {
                const prev = curve[i - 1].equity;
                rets.push(prev > 0 ? (curve[i].equity - prev) / prev : 0);
              }
              returnRows.push(rets);
            }
          } catch {
            // skip strategies without tearsheet
          }
        }
        if (returnRows.length >= 2) {
          const minCols = Math.min(...returnRows.map((r) => r.length));
          const alignedCols = Math.min(minCols - (minCols % 2), 16);
          if (alignedCols >= 4) {
            // Transpose to T_periods x N_strategies score matrix (even rows >= 4)
            const matrix: number[][] = [];
            for (let t = 0; t < alignedCols; t++) {
              matrix.push(returnRows.map((row) => row[t]));
            }
            const pbo = await daemonClient.computeQuantPbo({
              score_matrix: matrix,
              max_combinations: 500,
            });
            setPboResult(pbo);
            setPboNotice('');
          } else {
            setPboResult(null);
            setPboNotice('Need >= 4 aligned return periods across >= 2 backtested strategies to compute CSCV PBO.');
          }
        } else {
          setPboResult(null);
          setPboNotice('Run at least 2 strategy backtests in Alpha Studio to compute empirical CSCV PBO (no synthetic matrices).');
        }
      } catch {
        setPboResult(null);
      }
    } finally {
      setLoading(false);
    }
  }, [selection.strategyId, selection.symbol, selection.timeframe]);

  useEffect(() => {
    void runFullAudit();
  }, [runFullAudit, selection.revision]);

  return (
    <WorkbenchPane
      toolbar={
        <div className="flex items-center justify-between w-full px-3 py-1.5 text-xs font-semibold text-[var(--color-fg)]">
          <div className="flex items-center gap-2">
            <ShieldCheck size={14} className="text-workbench-accent" />
            <span>Pre-Trade Gate, CSCV PBO & FIFO Review</span>
          </div>
          <button
            type="button"
            disabled={loading}
            onClick={() => void runFullAudit()}
            className="flex items-center gap-1.5 px-2 py-1 rounded bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs font-medium text-[var(--color-fg-muted)] transition-colors"
          >
            <RotateCw size={12} className={loading ? 'animate-spin' : ''} />
            <span>Re-evaluate</span>
          </button>
        </div>
      }
    >
      <div className="flex h-full min-h-0 flex-col overflow-y-auto p-4 space-y-4 bg-[var(--color-canvas)] text-[var(--color-fg)]">
        {!gateResult && !reviewReport && !pboResult ? (
          <WorkbenchEmptyState
            title={loading ? 'Auditing Quant Risk State…' : 'No Risk Data Recorded Yet'}
            description="Ingest market bars and run backtests or paper fills to audit real Pre-Trade Gates, FIFO Net PnL cohorts, and CSCV Overfitting probability."
            action={
              <button
                type="button"
                onClick={() => onOpenTab?.('alpha')}
                className="canvas-open-button"
              >
                <ShieldCheck size={14} />
                <span>Open Alpha Studio</span>
              </button>
            }
            busy={loading}
          />
        ) : (
          <>
            {gateResult && (
              <div
                className={`flex items-start gap-3 p-4 rounded-lg border ${
                  gateResult.is_allowed
                    ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-400'
                    : 'border-rose-500/20 bg-rose-500/10 text-rose-400'
                }`}
              >
                {gateResult.is_allowed ? (
                  <CheckCircle2 size={20} className="shrink-0 mt-0.5" />
                ) : (
                  <XCircle size={20} className="shrink-0 mt-0.5" />
                )}
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h4 className="text-sm font-semibold">Pre-Trade Decision Gate: {gateResult.outcome.status}</h4>
                    <span className="text-[10px] font-mono px-1.5 py-0.5 rounded border border-current opacity-70">
                      {gateResult.symbol} · ${gateResult.order_notional.toLocaleString()}
                    </span>
                  </div>
                  <p className="mt-1 text-xs opacity-90">{gateResult.outcome.details.reason}</p>
                </div>
                {gateResult.is_allowed && (
                  <button
                    type="button"
                    onClick={() => {
                      onShowToast?.('Opening Execution Desk...');
                      onOpenTab?.('execution');
                    }}
                    className="px-3 py-1.5 rounded-md bg-emerald-500 text-white text-xs font-semibold hover:bg-emerald-600 shrink-0"
                  >
                    Proceed to Execution
                  </button>
                )}
              </div>
            )}

            <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4">
              <div className="flex items-center justify-between mb-2">
                <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
                  CSCV Probability of Backtest Overfitting (PBO)
                </h4>
                {pboResult && (
                  <span
                    className={`text-xs font-mono font-bold px-2 py-0.5 rounded ${
                      pboResult.is_overfitted
                        ? 'bg-rose-500/15 text-rose-400'
                        : 'bg-emerald-500/15 text-emerald-400'
                    }`}
                  >
                    PBO {(pboResult.probability * 100).toFixed(1)}% ({pboResult.samples} splits)
                  </span>
                )}
              </div>
              {pboResult ? (
                <p className="text-xs text-[var(--color-fg-muted)]">
                  Status: <strong className="text-[var(--color-fg)]">{pboResult.status}</strong> · Median Logit:{' '}
                  <span className="font-mono">{pboResult.median_logit.toFixed(3)}</span>
                </p>
              ) : (
                <p className="text-xs text-[var(--color-fg-muted)]">{pboNotice}</p>
              )}
            </div>

            {reviewReport && (
              <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
                    Post-Trade FIFO Net PnL Review ({reviewReport.strategy_id})
                  </h4>
                  <span className="text-xs font-mono text-[var(--color-fg-muted)]">
                    {reviewReport.total_trades} Paired Trades
                  </span>
                </div>

                {reviewReport.total_trades === 0 ? (
                  <p className="text-xs text-[var(--color-fg-muted)]">
                    No closed paired fills persisted in SQLite for this strategy yet.
                  </p>
                ) : (
                  <>
                    <div className="grid grid-cols-4 gap-3 text-xs">
                      <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
                        <div className="text-[10px] text-[var(--color-fg-muted)] uppercase">Gross PnL</div>
                        <div className="mt-1 font-mono font-bold">${reviewReport.gross_pnl.toFixed(2)}</div>
                      </div>
                      <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
                        <div className="text-[10px] text-[var(--color-fg-muted)] uppercase">Total Fees</div>
                        <div className="mt-1 font-mono font-bold text-amber-400">
                          -${reviewReport.total_commission.toFixed(2)}
                        </div>
                      </div>
                      <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
                        <div className="text-[10px] text-[var(--color-fg-muted)] uppercase">Net Realized PnL</div>
                        <div
                          className={`mt-1 font-mono font-bold ${
                            reviewReport.net_pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'
                          }`}
                        >
                          ${reviewReport.net_pnl.toFixed(2)}
                        </div>
                      </div>
                      <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
                        <div className="text-[10px] text-[var(--color-fg-muted)] uppercase">Profit Factor</div>
                        <div className="mt-1 font-mono font-bold">{reviewReport.profit_factor.toFixed(2)}</div>
                      </div>
                    </div>

                    {reviewReport.exit_cohorts.length > 0 && (
                      <div className="space-y-1">
                        <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">
                          Exit Reason Cohorts
                        </div>
                        <div className="grid grid-cols-3 gap-2 text-xs font-mono">
                          {reviewReport.exit_cohorts.map((ex) => (
                            <div
                              key={ex.reason}
                              className="p-2 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] flex items-center justify-between"
                            >
                              <span className="truncate">{ex.reason} ({ex.count})</span>
                              <span className={ex.net_pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}>
                                ${ex.net_pnl.toFixed(2)}
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {reviewReport.diagnostics.length > 0 && (
                      <div className="space-y-1.5">
                        {reviewReport.diagnostics.map((d) => (
                          <div
                            key={d.code}
                            className="flex items-start gap-2 p-2.5 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-xs"
                          >
                            <AlertTriangle size={14} className="text-amber-400 shrink-0 mt-0.5" />
                            <div>
                              <div className="font-semibold">
                                [{d.severity.toUpperCase()}] {d.summary}
                              </div>
                              <div className="text-[11px] text-[var(--color-fg-muted)]">{d.recommendation}</div>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </>
                )}
              </div>
            )}
          </>
        )}
      </div>
    </WorkbenchPane>
  );
}
