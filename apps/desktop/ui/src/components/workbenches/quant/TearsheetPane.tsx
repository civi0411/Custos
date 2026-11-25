import { useState, useEffect } from 'react';
import { BarChart3, ArrowUpRight, ArrowDownRight, Sparkles, CheckCircle2, AlertTriangle } from 'lucide-react';
import { WorkbenchPane, WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import { quantSelectionStore, useQuantSelection } from './quantSelectionStore';
import type { OrcaTabId } from '../workbenchResources';

interface TearsheetPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

interface TearsheetView {
  strategyId: string;
  annualizedSharpe: number;
  deflatedSharpe: number;
  maxDrawdown: number;
  totalReturn: number;
  calmarRatio: number;
  annualizedVol: number;
  winRate: number;
  profitFactor: number;
  turnover: number;
  sortinoRatio: number | null;
  downsideDeviation: number | null;
  omegaRatio: number | null;
  cvar95: number | null;
  lookaheadClean: boolean;
  startTimestamp: number | null;
  endTimestamp: number | null;
  barCount: number;
  monthlyReturns: Array<{ year: number; month: number; return_pct: number }>;
  equityCurve: Array<{ timestamp: number; equity: number; drawdown: number }>;
}

function fmtDate(ts: number | null): string {
  if (!ts) return '—';
  return new Date(ts * 1000).toISOString().slice(0, 10);
}

export function TearsheetPane({ onShowToast, onOpenTab }: TearsheetPaneProps) {
  const selection = useQuantSelection();
  const [metrics, setMetrics] = useState<TearsheetView | null>(null);
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    let mounted = true;
    setLoading(true);

    const resolveAndLoad = async () => {
      try {
        let targetId = selection.strategyId;
        if (!targetId) {
          const list = await daemonClient.listQuantStrategies();
          if (list?.strategies?.length) {
            targetId = list.strategies[0].id;
            quantSelectionStore.update({ strategyId: targetId });
          }
        }
        if (!targetId) {
          if (mounted) setMetrics(null);
          return;
        }
        const sheet = await daemonClient.getQuantTearsheet(targetId);
        if (mounted && sheet) {
          setMetrics({
            strategyId: sheet.strategy_id,
            annualizedSharpe: sheet.annualized_sharpe,
            deflatedSharpe: sheet.deflated_sharpe,
            maxDrawdown: -Math.abs(sheet.max_drawdown),
            totalReturn: sheet.total_return,
            calmarRatio: sheet.calmar_ratio,
            annualizedVol: sheet.annualized_volatility,
            winRate: sheet.win_rate,
            profitFactor: sheet.profit_factor,
            turnover: sheet.turnover,
            sortinoRatio: typeof sheet.sortino_ratio === 'number' ? sheet.sortino_ratio : null,
            downsideDeviation: typeof sheet.downside_deviation === 'number' ? sheet.downside_deviation : null,
            omegaRatio: typeof sheet.omega_ratio === 'number' ? sheet.omega_ratio : null,
            cvar95: typeof sheet.cvar_95 === 'number' ? sheet.cvar_95 : null,
            lookaheadClean: sheet.lookahead_clean !== false,
            startTimestamp: sheet.start_timestamp ?? null,
            endTimestamp: sheet.end_timestamp ?? null,
            barCount: sheet.bar_count ?? 0,
            monthlyReturns: sheet.monthly_returns ?? [],
            equityCurve: sheet.equity_curve ?? [],
          });
        }
      } catch {
        if (mounted) setMetrics(null);
      } finally {
        if (mounted) setLoading(false);
      }
    };

    void resolveAndLoad();
    return () => {
      mounted = false;
    };
  }, [selection.strategyId, selection.revision]);

  const equityPointsSvg = (() => {
    if (!metrics || metrics.equityCurve.length < 2) return null;
    const pts = metrics.equityCurve;
    const minEq = Math.min(...pts.map((p) => p.equity));
    const maxEq = Math.max(...pts.map((p) => p.equity));
    const span = Math.max(maxEq - minEq, 1e-6);
    const poly = pts
      .map((p, idx) => {
        const x = (idx / (pts.length - 1)) * 600;
        const y = 110 - ((p.equity - minEq) / span) * 100;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
    const ddPoly = pts
      .map((p, idx) => {
        const x = (idx / (pts.length - 1)) * 600;
        const y = 10 + Math.min(Math.max(p.drawdown, 0), 1) * 50;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
    return { poly, ddPoly, minEq, maxEq };
  })();

  return (
    <WorkbenchPane
      toolbar={
        <div className="flex items-center justify-between w-full px-3 py-1.5 text-xs font-semibold text-[var(--color-fg)]">
          <div className="flex items-center gap-2">
            <BarChart3 size={14} className="text-workbench-accent" />
            <span>Strategy Tearsheet</span>
          </div>
          {metrics && (
            <span className="text-[10px] font-mono text-[var(--color-fg-muted)]">
              {metrics.barCount > 0 ? `${metrics.barCount} bars simulated` : 'External summary'}
            </span>
          )}
        </div>
      }
    >
      {!metrics ? (
        <WorkbenchEmptyState
          title={loading ? 'Loading Strategy Tearsheet…' : 'No Strategy Tearsheet Recorded'}
          description="Run a zero-lookahead backtest from Alpha Studio to persist real equity curves, Deflated Sharpe Ratio (DSR), Sortino, Omega, and CVaR."
          action={
            <button
              type="button"
              onClick={() => onOpenTab?.('alpha')}
              className="canvas-open-button"
            >
              <Sparkles size={13} />
              <span>Open Alpha Studio</span>
            </button>
          }
          busy={loading}
        />
      ) : (
        <div className="flex h-full min-h-0 flex-col overflow-y-auto p-4 space-y-4 bg-[var(--color-canvas)] text-[var(--color-fg)]">
          <div className="flex items-center justify-between pb-3 border-b border-[var(--color-border-subtle)]">
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-base font-semibold font-mono">{metrics.strategyId}</h3>
                {metrics.lookaheadClean ? (
                  <span className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    <CheckCircle2 size={11} /> ZERO-LOOKAHEAD
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.5 rounded bg-rose-500/10 text-rose-400 border border-rose-500/20">
                    <AlertTriangle size={11} /> UNVERIFIED
                  </span>
                )}
              </div>
              <span className="text-xs text-[var(--color-fg-muted)]">
                Period: {fmtDate(metrics.startTimestamp)} to {fmtDate(metrics.endTimestamp)} · {metrics.barCount} Closed Bars
              </span>
            </div>
            <button
              type="button"
              onClick={() => {
                onShowToast?.('Proceeding to Risk Audit...');
                onOpenTab?.('risk_audit');
              }}
              className="px-3 py-1.5 rounded-md bg-[var(--color-primary-emphasis)] text-white text-xs font-medium hover:opacity-90"
            >
              Verify Risk & FIFO Review
            </button>
          </div>

          <div className="grid grid-cols-4 gap-3">
            <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
              <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Total Return</div>
              <div className={`mt-1 flex items-center gap-1 text-xl font-bold font-mono ${metrics.totalReturn >= 0 ? 'text-emerald-400' : 'text-rose-400'}`}>
                {metrics.totalReturn >= 0 ? <ArrowUpRight size={18} /> : <ArrowDownRight size={18} />}
                <span>{metrics.totalReturn >= 0 ? '+' : ''}{(metrics.totalReturn * 100).toFixed(2)}%</span>
              </div>
            </div>
            <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
              <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Annualized Sharpe</div>
              <div className="mt-1 text-xl font-bold font-mono text-emerald-400">
                {metrics.annualizedSharpe.toFixed(2)}
              </div>
            </div>
            <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
              <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Deflated Sharpe (DSR)</div>
              <div className="mt-1 text-xl font-bold font-mono text-cyan-400">
                {metrics.deflatedSharpe.toFixed(3)}
              </div>
            </div>
            <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
              <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Max Drawdown</div>
              <div className="mt-1 flex items-center gap-1 text-xl font-bold font-mono text-rose-400">
                <ArrowDownRight size={18} />
                <span>{(metrics.maxDrawdown * 100).toFixed(2)}%</span>
              </div>
            </div>
          </div>

          {equityPointsSvg && (
            <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4 space-y-2">
              <div className="flex items-center justify-between text-xs">
                <span className="font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
                  Equity Curve & Underwater Drawdown
                </span>
                <span className="font-mono text-[11px] text-[var(--color-fg-muted)]">
                  Range: ${equityPointsSvg.minEq.toFixed(0)} – ${equityPointsSvg.maxEq.toFixed(0)}
                </span>
              </div>
              <svg viewBox="0 0 600 120" className="w-full h-28 overflow-visible">
                <polyline fill="none" stroke="#34d399" strokeWidth="2" points={equityPointsSvg.poly} />
              </svg>
              <svg viewBox="0 0 600 65" className="w-full h-14 overflow-visible border-t border-[var(--color-border-subtle)] pt-1">
                <polyline fill="none" stroke="#fb7185" strokeWidth="1.5" points={equityPointsSvg.ddPoly} />
              </svg>
            </div>
          )}

          <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4">
            <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)] mb-3">
              Verified Risk & Distribution Metrics
            </h4>
            <div className="grid grid-cols-3 gap-y-3 text-xs">
              <div>
                <span className="text-[var(--color-fg-muted)]">Volatility (Ann):</span>
                <span className="ml-2 font-mono font-medium">{(metrics.annualizedVol * 100).toFixed(2)}%</span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Calmar Ratio:</span>
                <span className="ml-2 font-mono font-medium">{metrics.calmarRatio.toFixed(2)}</span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Win Rate:</span>
                <span className="ml-2 font-mono font-medium">{(metrics.winRate * 100).toFixed(1)}%</span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Profit Factor:</span>
                <span className="ml-2 font-mono font-medium">{metrics.profitFactor.toFixed(2)}</span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Sortino Ratio:</span>
                <span className="ml-2 font-mono font-medium text-emerald-400">
                  {metrics.sortinoRatio != null ? metrics.sortinoRatio.toFixed(2) : '—'}
                </span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Downside Deviation:</span>
                <span className="ml-2 font-mono font-medium">
                  {metrics.downsideDeviation != null ? `${(metrics.downsideDeviation * 100).toFixed(2)}%` : '—'}
                </span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Omega Ratio:</span>
                <span className="ml-2 font-mono font-medium text-cyan-400">
                  {metrics.omegaRatio != null ? metrics.omegaRatio.toFixed(2) : '—'}
                </span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">CVaR (95% Tail Risk):</span>
                <span className="ml-2 font-mono font-medium text-amber-400">
                  {metrics.cvar95 != null ? `${(metrics.cvar95 * 100).toFixed(2)}%` : '—'}
                </span>
              </div>
              <div>
                <span className="text-[var(--color-fg-muted)]">Trades / Turnover:</span>
                <span className="ml-2 font-mono font-medium">{metrics.turnover.toFixed(1)}</span>
              </div>
            </div>
          </div>

          {metrics.monthlyReturns.length > 0 && (
            <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4">
              <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)] mb-3">
                Monthly Returns Heatmap
              </h4>
              <div className="grid grid-cols-6 gap-2 text-xs font-mono">
                {metrics.monthlyReturns.map((m) => (
                  <div
                    key={`${m.year}-${m.month}`}
                    className={`p-2 rounded border text-center ${
                      m.return_pct >= 0
                        ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-300'
                        : 'border-rose-500/20 bg-rose-500/10 text-rose-300'
                    }`}
                  >
                    <div className="text-[10px] opacity-70">{m.year}-{String(m.month).padStart(2, '0')}</div>
                    <div className="font-bold">
                      {m.return_pct >= 0 ? '+' : ''}{(m.return_pct * 100).toFixed(2)}%
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </WorkbenchPane>
  );
}
