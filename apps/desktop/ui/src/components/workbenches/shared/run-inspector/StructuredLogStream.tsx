import React, { useState } from 'react';
import type { SupervisionLogEntry, LogSeverity } from '@/types/workflow_telemetry';

interface StructuredLogStreamProps {
  logs: SupervisionLogEntry[];
}

export const StructuredLogStream: React.FC<StructuredLogStreamProps> = ({ logs }) => {
  const [filterLevel, setFilterLevel] = useState<string>('all');
  const [search, setSearch] = useState<string>('');

  const filtered = logs.filter((log) => {
    if (filterLevel !== 'all' && log.level !== filterLevel) return false;
    if (search && !log.message.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  const levelColor = (level: LogSeverity) => {
    switch (level) {
      case 'fatal':
      case 'error':
        return 'text-rose-400 bg-rose-500/15 border-rose-500/30';
      case 'warn':
        return 'text-amber-400 bg-amber-500/15 border-amber-500/30';
      case 'info':
        return 'text-sky-400 bg-sky-500/15 border-sky-500/30';
      case 'debug':
      case 'trace':
      default:
        return 'text-white/50 bg-white/5 border-white/10';
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col space-y-2">
      <div className="flex items-center gap-2">
        <select
          value={filterLevel}
          onChange={(e) => setFilterLevel(e.target.value)}
          className="rounded border border-white/10 bg-white/[0.04] px-2 py-1 font-mono text-[11px] text-white/80 focus:outline-none"
        >
          <option value="all">All levels</option>
          <option value="error">Error / Fatal</option>
          <option value="warn">Warn</option>
          <option value="info">Info</option>
          <option value="debug">Debug / Trace</option>
        </select>
        <input
          type="text"
          placeholder="Filter log messages…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="flex-1 rounded border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[11px] text-white/90 placeholder:text-white/40 focus:outline-none"
        />
        <span className="font-mono text-[10px] text-white/40">{filtered.length} entries</span>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-white/10 bg-[#0d0e11] p-2 space-y-1 font-mono text-[10px]">
        {filtered.length === 0 ? (
          <p className="p-4 text-center text-white/40">No matching log entries recorded.</p>
        ) : (
          filtered.map((log) => (
            <div
              key={log.id}
              className="flex items-start gap-2 rounded px-1.5 py-1 hover:bg-white/[0.04] transition"
            >
              <span className="shrink-0 text-white/40">
                {new Date(log.timestamp_ms).toLocaleTimeString()}
              </span>
              <span
                className={`shrink-0 rounded px-1 py-0.5 text-[9px] uppercase font-semibold border ${levelColor(
                  log.level
                )}`}
              >
                {log.level}
              </span>
              <span className="min-w-0 flex-1 whitespace-pre-wrap break-all leading-relaxed text-white/85">
                {log.message}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

