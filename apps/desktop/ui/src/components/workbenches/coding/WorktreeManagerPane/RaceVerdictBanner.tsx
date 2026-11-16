import React from 'react';
import { Trophy, X } from 'lucide-react';
import type { daemonClient } from '@/api/daemon_client';

export interface RaceVerdictBannerProps {
  raceResult: Awaited<ReturnType<typeof daemonClient.runCandidateRace>> | null;
  onDismiss: () => void;
}

export const RaceVerdictBanner: React.FC<RaceVerdictBannerProps> = ({
  raceResult,
  onDismiss,
}) => {
  if (!raceResult) return null;

  return (
    <div className="rounded-xl border border-amber-500/30 bg-amber-950/20 p-3.5 space-y-2.5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Trophy className="h-4 w-4 text-amber-400 shrink-0" />
          <span className="text-xs font-semibold text-amber-200">
            Candidate Race Verdict:{' '}
            {raceResult.winner_candidate_id
              ? `Winner: ${raceResult.winner_candidate_id}`
              : 'No Automated Winner'}
          </span>
        </div>
        <button
          type="button"
          onClick={onDismiss}
          className="text-white/40 hover:text-white p-0.5 rounded transition"
        >
          <X className="h-3 w-3" />
        </button>
      </div>
      <div className="grid grid-cols-2 gap-2 text-[11px]">
        {raceResult.candidate_reports.map((r) => (
          <div
            key={r.candidate_id}
            className={`p-2 rounded-lg border ${
              r.candidate_id === raceResult.winner_candidate_id
                ? 'border-amber-500/40 bg-amber-500/10'
                : 'border-white/10 bg-white/5'
            }`}
          >
            <div className="flex items-center justify-between font-mono">
              <span className="font-semibold text-white">
                {r.candidate_id} ({r.model_id})
              </span>
              <span
                className={
                  r.test_passed
                    ? 'text-emerald-400 font-medium'
                    : 'text-rose-400 font-medium'
                }
              >
                {r.test_passed ? `✓ ${r.tests_succeeded} passed` : `✗ ${r.tests_failed} failed`}
              </span>
            </div>
            <div className="text-[10px] text-white/50 mt-1">
              Duration: {Math.round(r.execution_duration_ms / 1000)}s | Files: {r.modified_files.length}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
