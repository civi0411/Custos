import React from 'react';
import { useNavigate } from 'react-router-dom';
import { LayoutDashboard, MessageSquareCode, KeyRound, GitFork, Activity, CheckCircle2, Cpu, Shield, ArrowUpRight } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

export const DashboardPage: React.FC = () => {
  const navigate = useNavigate();
  const { currentSessions, providers } = useAppContext();

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6 text-fg-editor font-sans">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-border-muted">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="w-9 h-9 rounded-xl bg-surface-2 border border-border-default flex items-center justify-center workbench-accent">
              <LayoutDashboard className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-base sm:text-lg font-semibold tracking-[-0.02em] text-fg-editor">Custos System Dashboard</h1>
              <p className="text-xs text-fg-muted mt-0.5">Autonomous platform health, model throughput, and task execution status</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <div className="px-3 py-1.5 rounded-lg bg-surface-1 border border-border-default flex items-center gap-2 text-xs">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            <span className="text-fg-editor font-medium">Custos Daemon v0.1.0</span>
            <span className="text-fg-subtle font-mono">PID 4120</span>
          </div>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="p-4 bg-surface-1 border border-border-default rounded-xl shadow-xs">
          <div className="flex items-center justify-between">
            <span className="text-xs text-fg-subtle">Active Sessions</span>
            <MessageSquareCode className="w-4 h-4 workbench-accent" />
          </div>
          <div className="text-2xl font-semibold font-mono text-fg-editor mt-2">{currentSessions.length}</div>
          <span className="text-[11px] text-emerald-500 flex items-center gap-1 mt-1">
            <CheckCircle2 className="w-3 h-3" /> Ready for reasoning
          </span>
        </div>

        <div className="p-4 bg-surface-1 border border-border-default rounded-xl shadow-xs">
          <div className="flex items-center justify-between">
            <span className="text-xs text-fg-subtle">Configured Providers</span>
            <KeyRound className="w-4 h-4 workbench-accent" />
          </div>
          <div className="text-2xl font-semibold font-mono text-fg-editor mt-2">{providers.length}</div>
          <span className="text-[11px] text-fg-muted mt-1 block">Anthropic, OpenAI, Local</span>
        </div>

        <div className="p-4 bg-surface-1 border border-border-default rounded-xl shadow-xs">
          <div className="flex items-center justify-between">
            <span className="text-xs text-fg-subtle">Tokens Processed</span>
            <Cpu className="w-4 h-4 text-amber-500" />
          </div>
          <div className="text-2xl font-semibold font-mono text-fg-editor mt-2">148,200</div>
          <span className="text-[11px] text-fg-muted mt-1 block">Today across all tasks</span>
        </div>

        <div className="p-4 bg-surface-1 border border-border-default rounded-xl shadow-xs">
          <div className="flex items-center justify-between">
            <span className="text-xs text-fg-subtle">Kernel Reliability</span>
            <Shield className="w-4 h-4 text-emerald-500" />
          </div>
          <div className="text-2xl font-semibold font-mono text-emerald-500 mt-2">100.0%</div>
          <span className="text-[11px] text-fg-muted mt-1 block">0 panics, zero unwrap</span>
        </div>
      </div>

      {/* Quick Navigation Cards */}
      <div>
        <h2 className="text-xs font-semibold text-fg-subtle uppercase tracking-wider mb-3">Quick Navigation Routes</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div
            onClick={() => navigate('/studio')}
            className="p-4 bg-surface-1 hover:bg-surface-2 border border-border-default hover:border-border-emphasis rounded-xl cursor-pointer transition shadow-xs group"
          >
            <div className="flex items-center justify-between mb-2">
              <MessageSquareCode className="w-5 h-5 workbench-accent" />
              <ArrowUpRight className="w-4 h-4 text-fg-subtle group-hover:text-fg-editor transition" />
            </div>
            <h3 className="text-sm font-semibold text-fg-editor">Studio Workspace</h3>
            <p className="text-xs text-fg-muted mt-1">Interactive chat reasoning and side-by-side code diffing</p>
          </div>

          <div
            onClick={() => navigate('/providers')}
            className="p-4 bg-surface-1 hover:bg-surface-2 border border-border-default hover:border-border-emphasis rounded-xl cursor-pointer transition shadow-xs group"
          >
            <div className="flex items-center justify-between mb-2">
              <KeyRound className="w-5 h-5 workbench-accent" />
              <ArrowUpRight className="w-4 h-4 text-fg-subtle group-hover:text-fg-editor transition" />
            </div>
            <h3 className="text-sm font-semibold text-fg-editor">API Keys & Providers</h3>
            <p className="text-xs text-fg-muted mt-1">Configure LLM endpoints, credentials, and client tokens</p>
          </div>

          <div
            onClick={() => navigate('/chains')}
            className="p-4 bg-surface-1 hover:bg-surface-2 border border-border-default hover:border-border-emphasis rounded-xl cursor-pointer transition shadow-xs group"
          >
            <div className="flex items-center justify-between mb-2">
              <GitFork className="w-5 h-5 text-emerald-500" />
              <ArrowUpRight className="w-4 h-4 text-fg-subtle group-hover:text-fg-editor transition" />
            </div>
            <h3 className="text-sm font-semibold text-fg-editor">OmniRoute Chains</h3>
            <p className="text-xs text-fg-muted mt-1">Model failover pipelines and latency circuit breakers</p>
          </div>

          <div
            onClick={() => navigate('/telemetry')}
            className="p-4 bg-surface-1 hover:bg-surface-2 border border-border-default hover:border-border-emphasis rounded-xl cursor-pointer transition shadow-xs group"
          >
            <div className="flex items-center justify-between mb-2">
              <Activity className="w-5 h-5 workbench-accent" />
              <ArrowUpRight className="w-4 h-4 text-fg-subtle group-hover:text-fg-editor transition" />
            </div>
            <h3 className="text-sm font-semibold text-fg-editor">Logs & Telemetry</h3>
            <p className="text-xs text-fg-muted mt-1">Inspect real-time kernel spans, logs, and timing metrics</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default DashboardPage;
