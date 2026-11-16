import React from 'react';
import { TrendingUp, ShieldCheck, Activity, BarChart2 } from 'lucide-react';
import { Tooltip } from '@/components/common';
import type { OrcaTabId } from '@/components/workbenches/workbenchResources';

export interface QuantStrategyExplorerProps {
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
  strategies?: StrategyItem[];
}

export interface StrategyItem {
  id: string;
  name: string;
  universe: string;
  sharpe: number;
  icMean: number;
  status: 'live' | 'backtest' | 'audit';
}

export const QuantStrategyExplorer: React.FC<QuantStrategyExplorerProps> = ({
  onShowToast,
  onOpenTab,
  strategies = [],
}) => {
  return (
    <div className="space-y-2">
      <div className="p-2.5 rounded-xl border border-white/10 bg-white/[0.03] space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 min-w-0">
            <TrendingUp className="size-4 text-workbench-accent shrink-0" />
            <span className="font-bold text-xs text-white truncate">Active Strategy Portfolio</span>
          </div>
          <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-violet-400/10 text-violet-300 border border-violet-400/20">
            QUANT
          </span>
        </div>

        <div className="text-[11px] text-white/50 leading-relaxed">
          Supervised factor models, risk parity mandates, and execution order tickets.
        </div>
      </div>

      <div className="space-y-1.5 pt-1">
        <div className="text-[10px] font-semibold text-white/40 uppercase tracking-wider px-1">
          Strategies ({strategies.length})
        </div>

        {strategies.length === 0 ? (
          <div className="p-3 text-center border border-dashed border-white/10 rounded-lg text-white/50 text-[11px] space-y-2">
            <p>No active quantitative strategies registered in workspace.</p>
            <button
              type="button"
              onClick={() => (onOpenTab ? onOpenTab('alpha') : onShowToast?.('Opening Alpha Studio...'))}
              className="px-2.5 py-1 text-[11px] bg-white/10 hover:bg-white/15 text-white rounded transition"
            >
              Scan Strategy in Studio
            </button>
          </div>
        ) : (
          strategies.map((strat) => (
            <div
              key={strat.id}
              onClick={() => onShowToast?.(`Selected ${strat.name}`)}
              className="p-2.5 rounded-lg border border-white/8 bg-white/[0.02] hover:bg-white/[0.05] text-left cursor-pointer transition space-y-1"
            >
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-white truncate">{strat.name}</span>
                <span
                  className={`text-[9px] font-mono px-1.5 py-0.2 rounded border ${
                    strat.status === 'live'
                      ? 'bg-emerald-400/10 text-emerald-300 border-emerald-400/20'
                      : 'bg-amber-400/10 text-amber-300 border-amber-400/20'
                  }`}
                >
                  {strat.status.toUpperCase()}
                </span>
              </div>

              <div className="flex items-center justify-between text-[11px] text-white/40">
                <span>{strat.universe}</span>
                <div className="flex items-center gap-2 font-mono text-[10px]">
                  <span className="text-emerald-400/90">SR {strat.sharpe.toFixed(2)}</span>
                  <span className="text-white/60">IC {(strat.icMean * 100).toFixed(1)}%</span>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      <div className="pt-2 border-t border-white/6 flex items-center justify-around text-xs text-white/60">
        <Tooltip content="Open Alpha Factor Studio">
          <button
            type="button"
            onClick={() => (onOpenTab ? onOpenTab('alpha') : onShowToast?.('Opening Alpha Studio in main canvas...'))}
            className="flex items-center gap-1 hover:text-white transition py-1 text-[11px]"
          >
            <Activity className="size-3 text-workbench-accent" />
            <span>Alphas</span>
          </button>
        </Tooltip>
        <span className="text-white/20">|</span>
        <Tooltip content="Open Performance Tearsheet">
          <button
            type="button"
            onClick={() => (onOpenTab ? onOpenTab('tearsheet') : onShowToast?.('Opening Tearsheet in main canvas...'))}
            className="flex items-center gap-1 hover:text-white transition py-1 text-[11px]"
          >
            <BarChart2 className="size-3 text-emerald-400" />
            <span>Tearsheet</span>
          </button>
        </Tooltip>
        <span className="text-white/20">|</span>
        <Tooltip content="Open Risk & Lookahead Bias Audit">
          <button
            type="button"
            onClick={() => (onOpenTab ? onOpenTab('risk_audit') : onShowToast?.('Opening Risk Audit in main canvas...'))}
            className="flex items-center gap-1 hover:text-white transition py-1 text-[11px]"
          >
            <ShieldCheck className="size-3 text-amber-400" />
            <span>Risk</span>
          </button>
        </Tooltip>
      </div>
    </div>
  );
};
