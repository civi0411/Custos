import React from 'react';
import { AlertCircle } from 'lucide-react';
import type { ErrorGroupSummary } from '@/types/workflow_telemetry';

interface ErrorGroupsTableProps {
  groups: ErrorGroupSummary[];
  onSelectGroup?: (signatureHash: string) => void;
}

export const ErrorGroupsTable: React.FC<ErrorGroupsTableProps> = ({ groups, onSelectGroup }) => {
  if (groups.length === 0) {
    return (
      <div className="p-4 text-center text-xs text-white/50">
        No recurring error signatures detected across this scope.
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {groups.map((g) => (
        <button
          key={g.signature_hash}
          type="button"
          onClick={() => onSelectGroup?.(g.signature_hash)}
          className="w-full text-left rounded-lg border border-white/10 bg-white/[0.04] p-3 hover:bg-white/[0.08] transition duration-150 space-y-1.5"
        >
          <div className="flex items-center justify-between gap-2">
            <span className="inline-flex items-center gap-1.5 font-mono text-[11px] font-semibold text-rose-400">
              <AlertCircle size={13} className="shrink-0 text-rose-400" />
              {g.error_type}
            </span>
            <span className="rounded-full bg-rose-500/15 border border-rose-500/30 px-2 py-0.5 font-mono text-[10px] font-medium text-rose-300">
              {g.count} occurrence{g.count === 1 ? '' : 's'}
            </span>
          </div>
          <p className="line-clamp-2 font-mono text-[10px] leading-relaxed text-white/70">
            {g.sample_message}
          </p>
          <div className="flex items-center justify-between text-[9px] text-white/40 pt-1 border-t border-white/6">
            <span>Sig: {g.signature_hash.slice(0, 8)}…</span>
            <span>{g.affected_runs.length} affected run(s)</span>
          </div>
        </button>
      ))}
    </div>
  );
};

