import { useState, useEffect, useCallback } from 'react';
import { Activity, Play, Sparkles, Filter, CheckCircle2, AlertTriangle } from 'lucide-react';
import { WorkbenchPane, WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { daemonClient } from '@/api/daemon_client';
import { quantSelectionStore, useQuantSelection } from './quantSelectionStore';
import { VERIFIED_HISTORICAL_BENCHMARK_BARS } from './quantBenchmarkData';
import type { OrcaTabId } from '../workbenchResources';

interface AlphaStudioPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

interface AlphaCandidate {
  id: string;
  name: string;
  hypothesis: string;
  formula: string;
  icMean: number;
  icStd: number;
  ir: number;
  qValue?: number;
  classification?: string;
  lookaheadClean: boolean;
  status: 'discovered' | 'verified' | 'rejected';
}

export function AlphaStudioPane({ sessionId, onShowToast, onOpenTab }: AlphaStudioPaneProps) {
  const selection = useQuantSelection();
  const [symbol, setSymbol] = useState<string>(selection.symbol || 'SPY');
  const [timeframe, setTimeframe] = useState<string>(selection.timeframe || '1d');
  const [customFormula, setCustomFormula] = useState<string>('');
  const [candidates, setCandidates] = useState<AlphaCandidate[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [runningBacktest, setRunningBacktest] = useState<boolean>(false);
  const [selectedId, setSelectedId] = useState<string>('');
  const activeCandidate = candidates.find((c) => c.id === selectedId) || candidates[0];

  const handleDiscover = useCallback(async (targetSym = symbol, targetTf = timeframe) => {
    setLoading(true);
    try {
      const formulas = customFormula.trim() ? [customFormula.trim()] : undefined;
      onShowToast?.(`Scanning ${targetSym} (${targetTf}) with Factor DSL & BH-FDR control...`);
      const res = await daemonClient.discoverQuantFactors({
        symbol: targetSym,
        timeframe: targetTf,
        formulas,
      });
      if (res?.candidates && res.candidates.length > 0) {
        const mapped: AlphaCandidate[] = res.candidates.map((c) => ({
          id: c.id,
          name: c.name,
          hypothesis: c.hypothesis,
          formula: c.formula,
          icMean: c.ic_mean,
          icStd: c.ic_std,
          ir: c.ir,
          qValue: c.q_value,
          classification: c.classification,
          lookaheadClean: c.lookahead_clean,
          status: c.status === 'verified' ? 'verified' : 'discovered',
        }));
        setCandidates(mapped);
        setSelectedId(mapped[0].id);
        quantSelectionStore.update({ symbol: targetSym, timeframe: targetTf });
        onShowToast?.(`Evaluated ${res.total_evaluated} factors (${res.passed_fdr} passed BH-FDR q<=0.05)`);
      } else {
        setCandidates([]);
      }
    } catch (err) {
      onShowToast?.(`Discovery notice: ${err instanceof Error ? err.message : String(err)}`);
      setCandidates([]);
    } finally {
      setLoading(false);
    }
  }, [symbol, timeframe, customFormula, onShowToast]);

  const handleIngestAndDiscover = async () => {
    setLoading(true);
    try {
      onShowToast?.('Ingesting verified SPY historical benchmark bars...');
      setSymbol('SPY');
      setTimeframe('1d');
      await daemonClient.ingestQuantMarketBars({
        symbol: 'SPY',
        timeframe: '1d',
        bars: VERIFIED_HISTORICAL_BENCHMARK_BARS,
      });
      await handleDiscover('SPY', '1d');
    } catch (err) {
      onShowToast?.(`Ingest failed: ${err instanceof Error ? err.message : String(err)}`);
      setLoading(false);
    }
  };

  useEffect(() => {
    void handleDiscover(selection.symbol || 'SPY', selection.timeframe || '1d');
  }, []);

  const handleRunBacktest = async () => {
    if (!activeCandidate) return;
    setRunningBacktest(true);
    try {
      onShowToast?.(`Running deterministic next-bar-open backtest for ${activeCandidate.name}...`);
      const sheet = await daemonClient.submitQuantBacktest({
        strategy_id: activeCandidate.id,
        task_id: sessionId || `quant_bt_${Date.now()}`,
        symbol,
        timeframe,
        formula: activeCandidate.formula,
        trials_count: Math.max(candidates.length, 1),
        stop_loss_pct: 0.05,
        take_profit_pct: 0.15,
        trailing_stop_pct: 0.03,
      });
      quantSelectionStore.update(
        {
          strategyId: sheet.strategy_id,
          symbol,
          timeframe,
          formula: activeCandidate.formula,
        },
        true
      );
      onShowToast?.(
        `Backtest saved (${sheet.bar_count ?? 0} bars · SR ${sheet.annualized_sharpe.toFixed(2)} · DSR ${sheet.deflated_sharpe.toFixed(2)})`
      );
      onOpenTab?.('tearsheet');
    } catch (err) {
      onShowToast?.(`Backtest failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setRunningBacktest(false);
    }
  };

  return (
    <WorkbenchPane
      toolbar={
        <div className="flex items-center justify-between w-full gap-2 px-3 py-1.5 text-xs font-semibold text-[var(--color-fg)]">
          <div className="flex items-center gap-2">
            <Activity size={14} className="text-workbench-accent" />
            <span>Alpha Studio</span>
          </div>
          <div className="flex items-center gap-2 font-normal">
            <input
              type="text"
              value={symbol}
              onChange={(e) => setSymbol(e.target.value.toUpperCase())}
              placeholder="Symbol"
              className="w-20 px-2 py-0.5 text-[11px] font-mono rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]"
            />
            <select
              value={timeframe}
              onChange={(e) => setTimeframe(e.target.value)}
              className="px-1.5 py-0.5 text-[11px] font-mono rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]"
            >
              <option value="1d">1d</option>
              <option value="4h">4h</option>
              <option value="1h">1h</option>
              <option value="15m">15m</option>
            </select>
            <input
              type="text"
              value={customFormula}
              onChange={(e) => setCustomFormula(e.target.value)}
              placeholder="Optional DSL (e.g. rsi(14), zscore(20))"
              className="w-52 px-2 py-0.5 text-[11px] font-mono rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]"
            />
            <button
              type="button"
              onClick={() => void handleDiscover()}
              className="flex items-center gap-1 px-2 py-0.5 text-[11px] rounded bg-[var(--color-primary-emphasis)] text-white hover:opacity-90"
            >
              <Sparkles size={11} />
              <span>Scan</span>
            </button>
          </div>
        </div>
      }
    >
      {candidates.length === 0 ? (
        <WorkbenchEmptyState
          title={loading ? 'Scanning Market Factors…' : 'No Alpha Factors Evaluated'}
          description={`Ingest at least 20 closed historical bars for ${symbol} (${timeframe}) to evaluate empirical rank IC, IR, and Benjamini-Hochberg FDR without mocks.`}
          action={
            <div className="flex items-center gap-2">
              <button type="button" onClick={handleIngestAndDiscover} className="canvas-open-button">
                <Sparkles size={13} />
                <span>Ingest Verified SPY Benchmark & Scan</span>
              </button>
              <button
                type="button"
                onClick={() => void handleDiscover()}
                className="px-3 py-1.5 rounded-md border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs font-medium hover:bg-[var(--color-surface-2)]"
              >
                Scan Stored Bars
              </button>
            </div>
          }
          busy={loading}
        />
      ) : (
        <div className="flex h-full min-h-0 divide-x divide-[var(--color-border-subtle)] bg-[var(--color-canvas)] text-[var(--color-fg)]">
          <div className="flex w-72 shrink-0 flex-col overflow-y-auto p-3 space-y-2">
            <div className="flex items-center justify-between pb-2 border-b border-[var(--color-border-subtle)]">
              <span className="text-xs font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                Discovered Alphas ({candidates.length})
              </span>
            </div>
            {candidates.map((alpha) => (
              <div
                key={alpha.id}
                onClick={() => setSelectedId(alpha.id)}
                className={`p-2.5 rounded-lg border text-left cursor-pointer transition ${
                  alpha.id === activeCandidate?.id
                    ? 'border-[var(--color-border-focus)] bg-[var(--color-surface-2)] shadow-xs'
                    : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)]'
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium truncate">{alpha.name}</span>
                  {alpha.lookaheadClean && (
                    <span title="Lookahead bias clean">
                      <CheckCircle2 size={12} className="text-emerald-400 shrink-0" />
                    </span>
                  )}
                </div>
                <div className="mt-1 flex items-center justify-between text-[11px] text-[var(--color-fg-muted)]">
                  <span>IC: {(alpha.icMean * 100).toFixed(1)}% · IR: {alpha.ir.toFixed(2)}</span>
                  {alpha.classification && (
                    <span className="text-[10px] font-mono px-1 rounded bg-[var(--color-surface-3)] text-[var(--color-fg-muted)]">
                      {alpha.classification}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>

          <div className="flex flex-1 flex-col overflow-y-auto p-4 space-y-4">
            {activeCandidate ? (
              <>
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="text-base font-semibold">{activeCandidate.name}</h3>
                    {activeCandidate.classification && (
                      <span
                        className={`text-[10px] font-mono px-1.5 py-0.5 rounded border ${
                          activeCandidate.classification.toLowerCase().includes('reject')
                            ? 'border-rose-500/30 bg-rose-500/10 text-rose-400'
                            : 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'
                        }`}
                      >
                        FDR {activeCandidate.classification} (q={activeCandidate.qValue != null ? activeCandidate.qValue.toFixed(3) : '—'})
                      </span>
                    )}
                  </div>
                  <p className="mt-1 text-xs text-[var(--color-fg-muted)]">{activeCandidate.hypothesis}</p>
                  <div className="mt-2 text-[11px] font-mono px-2 py-1 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]">
                    Formula: {activeCandidate.formula} · Universe: {symbol} ({timeframe})
                  </div>
                </div>

                <div className="grid grid-cols-4 gap-3">
                  <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                    <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Information Coeff (Mean)</div>
                    <div className="mt-1 text-lg font-bold font-mono text-emerald-400">
                      {activeCandidate.icMean >= 0 ? '+' : ''}{(activeCandidate.icMean * 100).toFixed(2)}%
                    </div>
                  </div>
                  <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                    <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">IC Volatility (Std)</div>
                    <div className="mt-1 text-lg font-bold font-mono">
                      {(activeCandidate.icStd * 100).toFixed(2)}%
                    </div>
                  </div>
                  <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                    <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Information Ratio (IR)</div>
                    <div className="mt-1 text-lg font-bold font-mono text-cyan-400">
                      {activeCandidate.ir.toFixed(3)}
                    </div>
                  </div>
                  <div className="p-3 rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]">
                    <div className="text-[10px] uppercase font-semibold text-[var(--color-fg-muted)]">Lookahead Audit</div>
                    <div
                      className={`mt-1 flex items-center gap-1.5 text-sm font-semibold ${
                        activeCandidate.lookaheadClean ? 'text-emerald-400' : 'text-rose-400'
                      }`}
                    >
                      {activeCandidate.lookaheadClean ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
                      <span>{activeCandidate.lookaheadClean ? 'Verified Clean' : 'Leak Detected'}</span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-3 pt-2">
                  <button
                    type="button"
                    disabled={runningBacktest}
                    onClick={() => void handleRunBacktest()}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-[var(--color-primary-emphasis)] text-white text-xs font-medium hover:opacity-90 disabled:opacity-50"
                  >
                    <Play size={12} fill="currentColor" />
                    <span>{runningBacktest ? 'Simulating…' : 'Run Zero-Lookahead Backtest'}</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      quantSelectionStore.update({ strategyId: activeCandidate.id, symbol, timeframe });
                      onOpenTab?.('risk_audit');
                    }}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-xs font-medium hover:bg-[var(--color-surface-2)]"
                  >
                    <Filter size={12} />
                    <span>Audit Risk Metrics</span>
                  </button>
                </div>
              </>
            ) : (
              <WorkbenchEmptyState title="No signal selected" description="Select an alpha candidate or run discovery." />
            )}
          </div>
        </div>
      )}
    </WorkbenchPane>
  );
}
