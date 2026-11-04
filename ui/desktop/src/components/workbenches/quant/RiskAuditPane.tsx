import { useState, useEffect } from 'react';
import { ShieldCheck, CheckCircle2, AlertTriangle, XCircle, RotateCw } from 'lucide-react';
import { WorkbenchPane, WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
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

export function RiskAuditPane({ onShowToast, onOpenTab }: RiskAuditPaneProps) {
  const [loading, setLoading] = useState<boolean>(false);
  const [gateResult, setGateResult] = useState<GateOutcomeView | null>(null);
  const [reviewReport, setReviewReport] = useState<ReviewReportView | null>(null);

  const runGateEvaluation = async () => {
    setLoading(true);
    try {
      onShowToast?.('Evaluating pre-trade decision gate & exit immunity...');
      const res = await daemonClient.evaluateQuantGate({
        symbol: 'NVDA',
        direction: 'buy',
        estimated_price: 135.50,
        quantity: 500,
        daily_executed_notional: 350_000,
        current_drawdown: 0.015,
        consecutive_losses: 1,
        max_order_notional: 150_000,
        max_daily_notional: 1_000_000,
        max_drawdown_limit: 0.05,
        max_consecutive_losses: 5,
      });
      setGateResult(res);
      onShowToast?.(`Pre-Trade Gate evaluated: ${res.outcome.status}`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Gate evaluation failed: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  const runTradeReview = async () => {
    setLoading(true);
    try {
      onShowToast?.('Computing post-trade FIFO cohort review...');
      const res = await daemonClient.computeQuantReview({
        strategy_id: 'sig_momentum_res',
        symbol: 'NVDA',
        fills: [
          {
            fill_id: 'fill_01',
            order_id: 'ord_01',
            symbol: 'NVDA',
            direction: 'buy',
            price: 130.0,
            quantity: 100,
            commission: 1.5,
            timestamp: 1730000000,
          },
          {
            fill_id: 'fill_02',
            order_id: 'ord_02',
            symbol: 'NVDA',
            direction: 'sell',
            price: 135.0,
            quantity: 100,
            commission: 1.5,
            timestamp: 1730001800,
            exit_reason: 'take_profit',
          },
        ],
      });
      setReviewReport(res);
      onShowToast?.(`Review computed: Net PnL $${res.net_pnl.toFixed(2)}`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      onShowToast?.(`Review failed: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Initial evaluation on mount
    void runGateEvaluation();
    void runTradeReview();
  }, []);

  return (
    <WorkbenchPane
      toolbar={
        <div className="flex items-center justify-between w-full px-3 py-1.5 text-xs font-semibold text-[var(--color-fg)]">
          <div className="flex items-center gap-2">
            <ShieldCheck size={14} className="text-workbench-accent" />
            <span>Pre-Trade Decision Gate & Post-Trade Review</span>
          </div>
          <button
            type="button"
            disabled={loading}
            onClick={() => {
              void runGateEvaluation();
              void runTradeReview();
            }}
            className="flex items-center gap-1.5 px-2 py-1 rounded bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs font-medium text-[var(--color-fg-muted)] transition-colors"
          >
            <RotateCw size={12} className={loading ? 'animate-spin' : ''} />
            <span>Re-evaluate</span>
          </button>
        </div>
      }
    >
      <div className="flex h-full min-h-0 flex-col overflow-y-auto p-4 space-y-4 bg-[var(--color-canvas)] text-[var(--color-fg)]">
        {/* Gate Evaluation Section */}
        {gateResult ? (
          <div className={`flex items-start gap-3 p-4 rounded-lg border ${
            gateResult.is_allowed
              ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-400'
              : 'border-rose-500/20 bg-rose-500/10 text-rose-400'
          }`}>
            {gateResult.is_allowed ? (
              <CheckCircle2 size={20} className="shrink-0 mt-0.5" />
            ) : (
              <XCircle size={20} className="shrink-0 mt-0.5" />
            )}
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <h4 className="text-sm font-semibold">
                  Pre-Trade Decision Gate: {gateResult.outcome.status}
                </h4>
                <span className="text-[10px] font-mono px-1.5 py-0.5 rounded border border-current opacity-70">
                  {gateResult.symbol} · ${gateResult.order_notional.toLocaleString()}
                </span>
              </div>
              <p className="mt-1 text-xs opacity-90">{gateResult.outcome.details.reason}</p>
            </div>
            {gateResult.is_allowed && (
              <button
                type="button"
                onClick={() => onOpenTab?.('execution')}
                className="px-3 py-1.5 rounded-md bg-emerald-500 text-white text-xs font-semibold hover:bg-emerald-600 shrink-0"
              >
                Proceed to Execution
              </button>
            )}
          </div>
        ) : (
          <WorkbenchEmptyState
            title="Pre-Trade Gate Not Evaluated"
            description="Run the pre-trade decision gate to evaluate notional caps, loss streaks, drawdown limits, and exit immunity."
            action={
              <button
                type="button"
                onClick={() => void runGateEvaluation()}
                className="px-3 py-1.5 rounded-md bg-emerald-500 text-white text-xs font-semibold hover:bg-emerald-600"
              >
                Evaluate Gate
              </button>
            }
          />
        )}

        {/* Post-Trade Review & Cohort Diagnostics */}
        {reviewReport && (
          <div className="space-y-4">
            {/* KPI Summary Cards */}
            <div className="grid grid-cols-4 gap-3">
              <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">Total Trades</div>
                <div className="text-base font-mono font-bold mt-1 text-[var(--color-fg)]">{reviewReport.total_trades}</div>
              </div>
              <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">Net PnL</div>
                <div className={`text-base font-mono font-bold mt-1 ${reviewReport.net_pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                  {reviewReport.net_pnl >= 0 ? `+$${reviewReport.net_pnl.toFixed(2)}` : `-$${Math.abs(reviewReport.net_pnl).toFixed(2)}`}
                </div>
              </div>
              <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">Profit Factor</div>
                <div className="text-base font-mono font-bold mt-1 text-[var(--color-fg)]">{reviewReport.profit_factor.toFixed(2)}</div>
              </div>
              <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">Commission Drag</div>
                <div className="text-base font-mono font-bold mt-1 text-[var(--color-fg-muted)]">${reviewReport.total_commission.toFixed(2)}</div>
              </div>
            </div>

            {/* Holding Cohorts Table */}
            <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] overflow-hidden">
              <div className="px-4 py-2.5 border-b border-[var(--color-border-subtle)] flex items-center justify-between">
                <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
                  FIFO Holding Duration Cohorts
                </span>
                <span className="text-[11px] font-mono text-[var(--color-fg-muted)]">Multi-Timeframe Analysis</span>
              </div>
              <table className="w-full text-left text-xs">
                <thead className="border-b border-[var(--color-border-subtle)] text-[var(--color-fg-muted)]">
                  <tr>
                    <th className="px-4 py-2 font-medium">Cohort Window</th>
                    <th className="px-4 py-2 font-medium">Trade Count</th>
                    <th className="px-4 py-2 font-medium">Win Rate</th>
                    <th className="px-4 py-2 font-medium">Gross PnL</th>
                    <th className="px-4 py-2 font-medium">Net PnL</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[var(--color-border-subtle)]">
                  {reviewReport.holding_cohorts.map((cohort, idx) => (
                    <tr key={idx} className="hover:bg-[var(--color-surface-2)]">
                      <td className="px-4 py-2 font-semibold font-mono">{cohort.label}</td>
                      <td className="px-4 py-2 font-mono">{cohort.trade_count}</td>
                      <td className="px-4 py-2 font-mono">{(cohort.win_rate * 100).toFixed(1)}%</td>
                      <td className={`px-4 py-2 font-mono ${cohort.gross_pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                        ${cohort.gross_pnl.toFixed(2)}
                      </td>
                      <td className={`px-4 py-2 font-mono ${cohort.net_pnl >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                        ${cohort.net_pnl.toFixed(2)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Diagnostic Checks */}
            {reviewReport.diagnostics.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
                  Automated Diagnostic Assessment
                </h4>
                {reviewReport.diagnostics.map((diag, idx) => (
                  <div
                    key={idx}
                    className={`flex items-start gap-3 p-3 rounded-lg border ${
                      diag.severity === 'high'
                        ? 'border-rose-500/20 bg-rose-500/10 text-rose-300'
                        : diag.severity === 'medium'
                        ? 'border-amber-500/20 bg-amber-500/10 text-amber-300'
                        : 'border-blue-500/20 bg-blue-500/10 text-blue-300'
                    }`}
                  >
                    <AlertTriangle size={16} className="shrink-0 mt-0.5" />
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-semibold">{diag.summary}</span>
                        <span className="text-[10px] uppercase font-mono px-1 py-0.5 rounded bg-black/20">
                          {diag.code}
                        </span>
                      </div>
                      <p className="mt-1 text-xs opacity-90">{diag.recommendation}</p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </WorkbenchPane>
  );
}
