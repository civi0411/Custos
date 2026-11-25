import React from 'react';
import { Layers, Plus, Play } from 'lucide-react';
import type { HeadlessAutomationJob } from '@/types/domain';

export interface JobsTabViewProps {
  jobs: HeadlessAutomationJob[];
  canCreate: boolean;
  onOpenCreateModal: () => void;
  onRunJob: (jobId: string) => void;
}

export const JobsTabView: React.FC<JobsTabViewProps> = ({
  jobs,
  canCreate,
  onOpenCreateModal,
  onRunJob,
}) => {
  return (
    <div className="flex-1 flex flex-col bg-[var(--color-canvas)] overflow-hidden">
      <div className="flex items-center justify-between p-3.5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-[var(--color-editor-fg)] text-xs">Automation Jobs</span>
          <span className="text-[10px] font-mono px-1.5 py-0.5 bg-white/5 border border-white/10 rounded text-white/60">
            {jobs.length}
          </span>
        </div>
        <button
          disabled={!canCreate}
          onClick={onOpenCreateModal}
          className="flex items-center gap-1.5 px-2.5 py-1 bg-white text-black hover:bg-white/90 disabled:opacity-40 disabled:cursor-not-allowed rounded-md text-xs font-semibold transition shadow-sm"
        >
          <Plus size={12} className="stroke-[2.5]" />
          <span>Create Job</span>
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {jobs.length === 0 ? (
          <div className="text-center p-12 text-[var(--color-fg-muted)] space-y-3">
            <Layers size={28} className="text-white/20 mx-auto" />
            <p className="text-white/80 text-xs font-medium">No automation jobs</p>
            <button
              disabled={!canCreate}
              onClick={onOpenCreateModal}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-white text-black hover:bg-white/90 disabled:opacity-40 rounded-md text-xs font-semibold transition shadow-sm"
            >
              <Plus size={12} className="stroke-[2.5]" />
              <span>Create Job</span>
            </button>
          </div>
        ) : (
          jobs.map((job) => {
            const isCompleted = job.status === 'completed';
            return (
              <div
                key={job.id}
                className="p-4 rounded-lg bg-[var(--color-surface-1)] border border-[var(--color-border-default)] space-y-3 shadow-xs"
              >
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-[var(--color-editor-fg)] text-xs">{job.name}</span>
                      <span
                        className={`px-1.5 py-0.5 rounded text-[10px] font-mono border ${
                          isCompleted
                            ? 'bg-white/10 text-white border-white/20'
                            : 'bg-white/5 text-white/70 border-white/10'
                        }`}
                      >
                        {job.status}
                      </span>
                    </div>
                    <p className="text-[11px] text-[var(--color-fg-muted)] font-mono">
                      ID: {job.id} • Target: {job.spec.target_type}
                    </p>
                  </div>

                  <button
                    onClick={() => onRunJob(job.id)}
                    className="flex items-center gap-1.5 px-2.5 py-1 bg-white/10 hover:bg-white/20 text-white border border-white/20 rounded-md text-xs font-medium transition shadow-xs"
                  >
                    <Play size={11} className="fill-white" />
                    <span>Run</span>
                  </button>
                </div>

                <div className="p-2.5 rounded-md bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] font-mono text-[11px] space-y-1 text-[var(--color-editor-fg)]">
                  <div className="text-[var(--color-fg-muted)] text-[10px]">Command / Script:</div>
                  <div>{job.spec.command_or_script}</div>
                  {job.output_log && (
                    <div className="pt-2 border-t border-[var(--color-border-muted)] text-[var(--color-fg-muted)] whitespace-pre-wrap text-[10px]">
                      {job.output_log}
                    </div>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
