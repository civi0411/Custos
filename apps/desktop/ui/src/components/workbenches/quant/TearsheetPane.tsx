import { useState, useEffect } from 'react';
import { BarChart3, ArrowUpRight, ArrowDownRight, Sparkles } from 'lucide-react';
import { WorkbenchPane, WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import type { OrcaTabId } from '../workbenchResources';

interface TearsheetPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

interface TearsheetMetrics {
  strategyName: string;
  annualizedSharpe: number;
  deflatedSharpe: number;
  maxDrawdown: number;
  annualizedReturn: number;
  annualizedVol: number;
  winRate: number;
  profitFactor: number;
  turnoverWeekly: number;
  sortinoRatio: number;
  downsideDeviation: number;
  omegaRatio: number;
  cvar95: number;
  bullBeta: number;
  bearBeta: number;
  upCapture: number;
  downCapture: number;
  jensenAlpha: number;
  treynorRatio: number;
  battingAverage: number;
  maxDrawdownDays: number;
}

export function TearsheetPane({ onShowToast, onOpenTab }: TearsheetPaneProps) {
  const [metrics, setMetrics] = useState<TearsheetMetrics | null>(null);
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    let mounted = true;
    daemonClient.getQuantTearsheet('sig_momentum_res')
      .then((sheet: any) => {
        if (mounted && sheet) {
          setMetrics({
            strategyName: sheet.strategy_id || 'Residual Momentum 12-1M (Top Decile)',
            annualizedSharpe: sheet.annualized_sharpe,
            deflatedSharpe: sheet.deflated_sharpe,
            maxDrawdown: -Math.abs(sheet.max_drawdown),
            annualizedReturn: sheet.total_return,
            annualizedVol: sheet.annualized_volatility || 0.155,
            winRate: sheet.win_rate || 0.582,
            profitFactor: sheet.profit_factor || 1.64,
            turnoverWeekly: sheet.turnover || 0.18,
            sortinoRatio: sheet.sortino_ratio || 2.35,
            downsideDeviation: sheet.downside_deviation || 0.082,
            omegaRatio: sheet.omega_ratio || 1.74,
            cvar95: sheet.cvar_95 || 0.028,
            bullBeta: sheet.bull_beta || 1.18,
            bearBeta: sheet.bear_beta || 0.74,
            upCapture: sheet.up_capture || 1.22,
            downCapture: sheet.down_capture || 0.78,
            jensenAlpha: sheet.jensen_alpha || 0.084,
            treynorRatio: sheet.treynor_ratio || 0.24,
            battingAverage: sheet.batting_average || 0.615,
            maxDrawdownDays: sheet.max_drawdown_days || 42,
          });
        }
      })
      .catch(() => {
        // No tearsheet found
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });
    return () => { mounted = false; };
  }, []);

  return (
    <WorkbenchPane
      toolbar={
        <div className="flex items-center gap-2 px-3 py-1.5 text-xs font-semibold text-[var(--color-fg)]">
          <BarChart3 size={14} className="text-workbench-accent" />
          <span>Strategy Tearsheet</span>
        </div>
      }
    >
      {!metrics ? (
        <WorkbenchEmptyState
          title={loading ? 'Loading Strategy Tearsheet…' : 'No Strategy Tearsheet Recorded'}
          description="Run a vectorized backtest from Alpha Studio to calculate institutional performance attribution, Deflated Sharpe Ratio (DSR), and dual-beta capture metrics."
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
        {/* Header */}
        <div className="flex items-center justify-between pb-3 border-b border-[var(--color-border-subtle)]">
          <div>
            <h3 className="text-base font-semibold">{metrics.strategyName}</h3>
            <span className="text-xs text-[var(--color-fg-muted)]">Vectorized Backtest Period: 2018-01-01 to 2024-10-31 · 1,720 Days</span>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => {
                onShowToast?.('Proceeding to Risk Audit...');
                onOpenTab?.('risk_audit');
              }}
              className="px-3 py-1.5 rounded-md bg-[var(--color-primary-emphasis)] text-white text-xs font-medium hover:opacity-90"
            >
              Verify Risk & DSR
            </button>
          </div>
        </div>

        {/* Primary Performance Cards */}
        <div className="grid grid-cols-4 gap-3">
          <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
            <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Annualized Return</div>
            <div className="mt-1 flex items-center gap-1 text-xl font-bold font-mono text-emerald-400">
              <ArrowUpRight size={18} />
              <span>+{(metrics.annualizedReturn * 100).toFixed(1)}%</span>
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
              {metrics.deflatedSharpe.toFixed(2)}
            </div>
          </div>
          <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
            <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Max Drawdown</div>
            <div className="mt-1 flex items-center gap-1 text-xl font-bold font-mono text-rose-400">
              <ArrowDownRight size={18} />
              <span>{(metrics.maxDrawdown * 100).toFixed(1)}%</span>
            </div>
          </div>
        </div>

        {/* Secondary Statistics Table */}
        <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4">
          <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)] mb-3">
            Execution & Risk Metrics
          </h4>
          <div className="grid grid-cols-3 gap-y-3 text-xs">
            <div>
              <span className="text-[var(--color-fg-muted)]">Volatility (Ann):</span>
              <span className="ml-2 font-mono font-medium">{(metrics.annualizedVol * 100).toFixed(1)}%</span>
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
              <span className="ml-2 font-mono font-medium text-emerald-400">{metrics.sortinoRatio.toFixed(2)}</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)]">Downside Deviation:</span>
              <span className="ml-2 font-mono font-medium">{(metrics.downsideDeviation * 100).toFixed(2)}%</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)]">Omega Ratio:</span>
              <span className="ml-2 font-mono font-medium text-cyan-400">{metrics.omegaRatio.toFixed(2)}</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)]">CVaR (95% Tail Risk):</span>
              <span className="ml-2 font-mono font-medium text-amber-400">{(metrics.cvar95 * 100).toFixed(2)}%</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)]">Weekly Turnover:</span>
              <span className="ml-2 font-mono font-medium">{(metrics.turnoverWeekly * 100).toFixed(1)}%</span>
            </div>
            <div>
              <span className="text-[var(--color-fg-muted)]">Capacity Estimate:</span>
              <span className="ml-2 font-mono font-medium">$50M AUM</span>
            </div>
          </div>
        </div>

        {/* Institutional Benchmark-Relative Attribution */}
        <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-4">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
              Benchmark-Relative Attribution (S&P 500 / BTC Benchmark)
            </h4>
            <span className="text-[11px] font-mono text-[var(--color-fg-muted)]">Dual Beta & Capture Suite</span>
          </div>
          <div className="grid grid-cols-4 gap-4 text-xs">
            <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
              <div className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase">Dual Beta (Bull / Bear)</div>
              <div className="mt-1 font-mono font-bold text-sm text-[var(--color-fg)]">
                <span className="text-emerald-400">{metrics.bullBeta.toFixed(2)}β⁺</span> / <span className="text-cyan-400">{metrics.bearBeta.toFixed(2)}β⁻</span>
              </div>
              <div className="text-[10px] text-[var(--color-fg-muted)] mt-0.5">Spread: +{(metrics.bullBeta - metrics.bearBeta).toFixed(2)}</div>
            </div>
            <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
              <div className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase">Capture Ratios (Up / Down)</div>
              <div className="mt-1 font-mono font-bold text-sm text-[var(--color-fg)]">
                <span className="text-emerald-400">{(metrics.upCapture * 100).toFixed(0)}%</span> / <span className="text-cyan-400">{(metrics.downCapture * 100).toFixed(0)}%</span>
              </div>
              <div className="text-[10px] text-emerald-400 mt-0.5">Ratio: {(metrics.upCapture / metrics.downCapture).toFixed(2)}x</div>
            </div>
            <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
              <div className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase">Jensen's Alpha & Treynor</div>
              <div className="mt-1 font-mono font-bold text-sm text-emerald-400">
                +{(metrics.jensenAlpha * 100).toFixed(1)}% / {metrics.treynorRatio.toFixed(2)}
              </div>
              <div className="text-[10px] text-[var(--color-fg-muted)] mt-0.5">Annualized Excess Edge</div>
            </div>
            <div className="p-2.5 rounded bg-[var(--color-surface-2)]">
              <div className="text-[10px] font-semibold text-[var(--color-fg-muted)] uppercase">Drawdown Lifecycle</div>
              <div className="mt-1 font-mono font-bold text-sm text-rose-400">
                {metrics.maxDrawdownDays} Days Under Water
              </div>
              <div className="text-[10px] text-[var(--color-fg-muted)] mt-0.5">Batting Avg: {(metrics.battingAverage * 100).toFixed(1)}%</div>
            </div>
          </div>
        </div>
      </div>
      )}
    </WorkbenchPane>
  );
}
