import React from 'react';
import { Terminal, Trash2, ChevronDown, AlertCircle, Info } from 'lucide-react';

export interface ConsoleLogItem {
  id: string;
  type: 'log' | 'warn' | 'error' | 'info';
  message: string;
  timestamp: string;
  source?: string;
}

export interface DevDrawerProps {
  logs: ConsoleLogItem[];
  onClear: () => void;
  onClose: () => void;
}

export const DevDrawer: React.FC<DevDrawerProps> = ({ logs, onClear, onClose }) => {
  return (
    <div className="h-44 shrink-0 flex flex-col border-t border-[var(--color-border-default)] bg-[var(--color-surface-0)]">
      <div className="h-7 shrink-0 flex items-center justify-between px-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-3">
          <div className="font-semibold text-[11px] flex items-center gap-1.5 text-[var(--color-editor-fg)]">
            <Terminal size={11} />
            <span>Browser navigation errors ({logs.length})</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={onClear}
            className="p-1 hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] rounded"
            title="Clear logs"
          >
            <Trash2 size={12} />
          </button>
          <button
            onClick={onClose}
            className="p-1 hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] rounded"
            title="Close browser error log"
          >
            <ChevronDown size={13} />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-2 font-mono text-[11px] space-y-1">
        {logs.length === 0 ? (
          <div className="text-[var(--color-fg-subtle)] p-4 text-center">
            No Custos navigation errors recorded. Page console and network capture are not available.
          </div>
        ) : (
          logs.map((log) => (
            <div
              key={log.id}
              className={`flex items-start gap-2 py-0.5 px-1.5 rounded ${
                log.type === 'error'
                  ? 'bg-rose-500/10 text-rose-300'
                  : log.type === 'warn'
                  ? 'bg-amber-500/10 text-amber-300'
                  : 'text-[var(--color-fg-default)]'
              }`}
            >
              <span className="text-[9px] text-[var(--color-fg-muted)] shrink-0">{log.timestamp}</span>
              <span className="shrink-0">
                {log.type === 'error' ? (
                  <AlertCircle size={11} className="text-rose-400" />
                ) : (
                  <Info size={11} className="text-sky-400" />
                )}
              </span>
              <span className="flex-1 break-words">{log.message}</span>
              {log.source && <span className="text-[9px] text-[var(--color-fg-subtle)] shrink-0">{log.source}</span>}
            </div>
          ))
        )}
      </div>
    </div>
  );
};
