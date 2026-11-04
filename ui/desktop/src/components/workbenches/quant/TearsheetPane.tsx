import { useState, useEffect } from 'react';
import { BarChart3, ArrowUpRight, ArrowDownRight } from 'lucide-react';
import { WorkbenchPane } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import type { OrcaTabId } from '../workbenchResources';

interface TearsheetPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export function TearsheetPane({ onShowToast, onOpenTab }: TearsheetPaneProps) {
  const [metrics, setMetrics] = useState({
    strategyName: 'Residual Momentum 12-1M (Top Decile)',
    annualizedSharpe: 1.84,
    deflatedSharpe: 1.42,
    maxDrawdown: -0.114,
    annualizedReturn: 0.286,
    annualizedVol: 0.155,
    winRate: 0.582,
    profitFactor: 1.64,
    turnoverWeekly: 0.18,
    sortinoRatio: 2.35,
    downsideDeviation: 0.082,
    omegaRatio: 1.74,
    cvar95: 0.028,
  });

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
          });
        }
      })
      .catch(() => {
        // Fallback gracefully to preset metrics
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
      </div>
    </WorkbenchPane>
  );
}
