import React from 'react';
import { useNavigate } from 'react-router-dom';
import { LayoutDashboard, MessageSquareCode, KeyRound, GitFork, Activity, CheckCircle2, Cpu, Shield, ArrowUpRight } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

export const DashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const { currentSessions, providers } = useAppContext();

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-surface-border">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-blue/10 border border-brand-blue/30 text-brand-blue">
              <LayoutDashboard className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-lg font-bold text-white tracking-tight">Custos System Dashboard</h1>
              <p className="text-xs text-neutral-400">Autonomous platform health, model throughput, and task execution status</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <div className="px-3 py-1.5 rounded-lg bg-surface-card border border-surface-border flex items-center gap-2 text-xs">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span className="text-neutral-300 font-medium">Custos Daemon v0.1.0</span>
            <span className="text-neutral-500 font-mono">PID 4120</span>
          </div>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="p-4 bg-surface-card border border-surface-border rounded-xl">
          <div className="flex items-center justify-between">
            <span className="text-xs text-neutral-400">Active Sessions</span>
            <MessageSquareCode className="w-4 h-4 text-brand-blue" />
          </div>
          <div className="text-2xl font-bold font-mono text-white mt-2">{currentSessions.length}</div>
          <span className="text-[11px] text-emerald-400 flex items-center gap-1 mt-1">
            <CheckCircle2 className="w-3 h-3" /> Ready for reasoning
          </span>
        </div>

        <div className="p-4 bg-surface-card border border-surface-border rounded-xl">
          <div className="flex items-center justify-between">
            <span className="text-xs text-neutral-400">Configured Providers</span>
            <KeyRound className="w-4 h-4 text-purple-400" />
          </div>
          <div className="text-2xl font-bold font-mono text-white mt-2">{providers.length}</div>
          <span className="text-[11px] text-neutral-400 mt-1 block">Anthropic, OpenAI, Local</span>
        </div>

        <div className="p-4 bg-surface-card border border-surface-border rounded-xl">
          <div className="flex items-center justify-between">
            <span className="text-xs text-neutral-400">Tokens Processed</span>
            <Cpu className="w-4 h-4 text-amber-400" />
          </div>
          <div className="text-2xl font-bold font-mono text-white mt-2">148,200</div>
          <span className="text-[11px] text-neutral-400 mt-1 block">Today across all tasks</span>
        </div>

        <div className="p-4 bg-surface-card border border-surface-border rounded-xl">
          <div className="flex items-center justify-between">
            <span className="text-xs text-neutral-400">Kernel Reliability</span>
            <Shield className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-2xl font-bold font-mono text-emerald-400 mt-2">100.0%</div>
          <span className="text-[11px] text-neutral-400 mt-1 block">0 panics, zero unwrap</span>
        </div>
      </div>

      {/* Quick Navigation Cards */}
      <div>
        <h2 className="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-3">Quick Navigation Routes</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div
            onClick={() => navigate('/studio')}
            className="p-4 bg-surface-card hover:bg-surface-elevated border border-surface-border hover:border-brand-blue/40 rounded-xl cursor-pointer transition group"
          >
            <div className="flex items-center justify-between mb-2">
              <MessageSquareCode className="w-5 h-5 text-brand-blue" />
              <ArrowUpRight className="w-4 h-4 text-neutral-500 group-hover:text-white transition" />
            </div>
            <h3 className="text-sm font-semibold text-white">Studio Workspace</h3>
            <p className="text-xs text-neutral-400 mt-1">Interactive chat reasoning and side-by-side code diffing</p>
          </div>

          <div
            onClick={() => navigate('/providers')}
            className="p-4 bg-surface-card hover:bg-surface-elevated border border-surface-border hover:border-purple-500/40 rounded-xl cursor-pointer transition group"
          >
            <div className="flex items-center justify-between mb-2">
              <KeyRound className="w-5 h-5 text-purple-400" />
              <ArrowUpRight className="w-4 h-4 text-neutral-500 group-hover:text-white transition" />
            </div>
            <h3 className="text-sm font-semibold text-white">API Keys & Providers</h3>
            <p className="text-xs text-neutral-400 mt-1">Configure LLM endpoints, credentials, and client tokens</p>
          </div>

          <div
            onClick={() => navigate('/chains')}
            className="p-4 bg-surface-card hover:bg-surface-elevated border border-surface-border hover:border-cyan-500/40 rounded-xl cursor-pointer transition group"
          >
            <div className="flex items-center justify-between mb-2">
              <GitFork className="w-5 h-5 text-cyan-400" />
              <ArrowUpRight className="w-4 h-4 text-neutral-500 group-hover:text-white transition" />
            </div>
            <h3 className="text-sm font-semibold text-white">OmniRoute Chains</h3>
            <p className="text-xs text-neutral-400 mt-1">Model failover pipelines and latency circuit breakers</p>
          </div>

          <div
            onClick={() => navigate('/telemetry')}
            className="p-4 bg-surface-card hover:bg-surface-elevated border border-surface-border hover:border-emerald-500/40 rounded-xl cursor-pointer transition group"
          >
            <div className="flex items-center justify-between mb-2">
              <Activity className="w-5 h-5 text-emerald-400" />
              <ArrowUpRight className="w-4 h-4 text-neutral-500 group-hover:text-white transition" />
            </div>
            <h3 className="text-sm font-semibold text-white">Logs & Telemetry</h3>
            <p className="text-xs text-neutral-400 mt-1">Inspect real-time kernel spans, logs, and timing metrics</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
