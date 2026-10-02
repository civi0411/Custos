import React, { useState } from 'react';
import { Activity, Search, Trash2, Pause, Play } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

interface TraceLog {
  id: string;
  time: string;
  level: 'INFO' | 'DEBUG' | 'WARN';
  source: string;
  taskId: string;
  latency?: string;
  message: string;
}

const mockTraces: TraceLog[] = [
  { id: '1', time: '22:09:41.102', level: 'INFO', source: 'custos-kernel', taskId: 'task-auth-09', latency: '4ms', message: 'Task state transition verified: Draft -> Running' },
  { id: '2', time: '22:09:41.108', level: 'DEBUG', source: 'custos-persistence', taskId: 'task-auth-09', latency: '2ms', message: 'Outbox event appended to SQLite journal (seq_id: 1042)' },
  { id: '3', time: '22:09:41.152', level: 'INFO', source: 'custos-provider-sdk', taskId: 'task-auth-09', latency: '44ms', message: 'OmniRoute dispatch to Anthropic claude-3-7-sonnet succeeded (200 OK)' },
  { id: '4', time: '22:09:41.156', level: 'DEBUG', source: 'custos-security', taskId: 'task-auth-09', latency: '1ms', message: 'ExecutionPermit granted for sandbox path: src/middleware/gateway.ts' },
  { id: '5', time: '22:09:41.160', level: 'INFO', source: 'custos-kernel', taskId: 'task-auth-09', latency: '3ms', message: 'Evidence bundle compiled with 2 verifiable receipts' },
  { id: '6', time: '22:09:30.820', level: 'WARN', source: 'custos-adapters-mcp', taskId: 'task-mcp-01', latency: '180ms', message: 'MCP tool response exceeded soft budget 150ms; circuit breaker watching' },
  { id: '7', time: '22:09:12.400', level: 'DEBUG', source: 'custos-memory-service', taskId: 'task-ast-02', latency: '8ms', message: 'Retrieved 4 AST chunks from local vector store index' },
  { id: '8', time: '22:08:50.012', level: 'INFO', source: 'custos-daemon', taskId: 'sys-init', latency: '12ms', message: 'Custos Local API listening on 127.0.0.1:4140' }
];

export const TelemetryPage: React.FC = () => {
  const { showToast } = useAppContext();
  const [filterLevel, setFilterLevel] = useState<'ALL' | 'INFO' | 'WARN' | 'DEBUG'>('ALL');
  const [searchQuery, setSearchQuery] = useState('');
  const [isPaused, setIsPaused] = useState(false);
  const [traces, setTraces] = useState<TraceLog[]>(mockTraces);

  const filtered = traces.filter((t) => {
    const matchesLevel = filterLevel === 'ALL' || t.level === filterLevel;
    const matchesSearch =
      searchQuery.trim() === '' ||
      t.message.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.source.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.taskId.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesLevel && matchesSearch;
  });

  return (
    <div className="flex-1 flex flex-col h-full bg-surface overflow-hidden p-4 sm:p-6 space-y-4">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-3 border-b border-surface-border shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="p-2 rounded-xl bg-purple-500/10 border border-purple-500/30 text-purple-400">
            <Activity className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-lg font-bold text-white tracking-tight">Logs & Telemetry</h1>
            <p className="text-xs text-neutral-400">Real-time structured tracing, kernel spans, and correlation telemetry</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsPaused(!isPaused)}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition flex items-center gap-1.5 border ${
              isPaused
                ? 'bg-amber-500/10 text-amber-400 border-amber-500/30'
                : 'bg-surface-elevated text-neutral-300 border-surface-border hover:bg-surface-hover'
            }`}
          >
            {isPaused ? <Play className="w-3.5 h-3.5" /> : <Pause className="w-3.5 h-3.5" />}
            <span>{isPaused ? 'Resume' : 'Pause'}</span>
          </button>
          <button
            onClick={() => {
              setTraces([]);
              showToast('Telemetry stream cleared');
            }}
            className="px-3 py-1.5 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs font-medium text-neutral-300 transition flex items-center gap-1.5"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Clear</span>
          </button>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 shrink-0">
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Trace Events</div>
          <div className="text-lg font-mono font-bold text-white mt-0.5">{traces.length}</div>
        </div>
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Mean Latency</div>
          <div className="text-lg font-mono font-bold text-emerald-400 mt-0.5">18.4ms</div>
        </div>
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Active Spans</div>
          <div className="text-lg font-mono font-bold text-brand-blue mt-0.5">3 active</div>
        </div>
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Outbox Queue</div>
          <div className="text-lg font-mono font-bold text-purple-400 mt-0.5">0 pending</div>
        </div>
      </div>

      {/* Filter / Search Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3 shrink-0 bg-surface-card p-2 rounded-xl border border-surface-border text-xs">
        <div className="flex items-center gap-2 flex-1 min-w-[200px]">
          <Search className="w-4 h-4 text-neutral-500 ml-2 shrink-0" />
          <input
            type="text"
            placeholder="Search spans by source, task_id, or message..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-transparent border-none text-neutral-200 placeholder-neutral-500 focus:outline-none text-xs"
          />
        </div>

        <div className="flex items-center gap-1.5">
          {(['ALL', 'INFO', 'WARN', 'DEBUG'] as const).map((lvl) => (
            <button
              key={lvl}
              onClick={() => setFilterLevel(lvl)}
              className={`px-2.5 py-1 rounded-lg text-xs font-mono transition ${
                filterLevel === lvl
                  ? 'bg-brand-blue text-white font-medium'
                  : 'text-neutral-400 hover:text-white hover:bg-surface-elevated'
              }`}
            >
              {lvl}
            </button>
          ))}
        </div>
      </div>

      {/* Traces List Table */}
      <div className="flex-1 bg-surface-card border border-surface-border rounded-xl overflow-hidden flex flex-col min-h-0">
        <div className="overflow-y-auto flex-1 font-mono text-xs divide-y divide-surface-border/40">
          {filtered.length === 0 ? (
            <div className="p-8 text-center text-neutral-500 text-xs">No matching telemetry traces found.</div>
          ) : (
            filtered.map((trace) => (
              <div key={trace.id} className="p-2.5 sm:px-4 sm:py-2.5 hover:bg-surface-elevated/60 transition flex items-start gap-3">
                <span className="text-[11px] text-neutral-500 shrink-0">{trace.time}</span>
                <span
                  className={`text-[10px] px-1.5 py-0.5 rounded font-bold uppercase shrink-0 ${
                    trace.level === 'INFO'
                      ? 'bg-blue-500/10 text-blue-400 border border-blue-500/20'
                      : trace.level === 'WARN'
                      ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                      : 'bg-neutral-800 text-neutral-400 border border-neutral-700'
                  }`}
                >
                  {trace.level}
                </span>
                <span className="text-neutral-400 shrink-0 hidden sm:inline">{trace.source}</span>
                <span className="text-[10px] px-1.5 py-0.2 rounded bg-surface-elevated text-neutral-300 font-mono shrink-0">
                  {trace.taskId}
                </span>
                <span className="text-neutral-200 flex-1 truncate">{trace.message}</span>
                {trace.latency && (
                  <span className="text-[11px] text-emerald-400 font-mono shrink-0">{trace.latency}</span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default TelemetryPage;
