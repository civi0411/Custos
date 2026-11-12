import React from 'react';
import { ExternalLink } from 'lucide-react';
import type { Task } from '@/types/domain';

export interface DashboardTaskDetailProps {
  task: Task;
  onOpenTask: (taskId: string) => void;
}

export const DashboardTaskDetail: React.FC<DashboardTaskDetailProps> = ({ task, onOpenTask }) => {
  return (
    <div className="space-y-4 overflow-y-auto p-6 text-xs text-white/80">
      <div className="flex items-center justify-between border-b border-white/10 pb-3">
        <div>
          <h2 className="text-sm font-semibold text-white">{task.title}</h2>
          <p className="mt-0.5 text-white/50">{task.contract?.description || 'No description recorded.'}</p>
        </div>
        <button
          type="button"
          onClick={() => onOpenTask(task.id)}
          className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-white hover:bg-white/[0.08]"
        >
          <ExternalLink size={13} />
          <span>Open Task in Studio</span>
        </button>
      </div>

      <dl className="grid grid-cols-2 gap-3 rounded-lg border border-white/8 bg-[#141518] p-3 font-mono text-[11px]">
        <div>
          <dt className="text-white/40">Status</dt>
          <dd className="font-semibold text-amber-300">{task.status}</dd>
        </div>
        <div>
          <dt className="text-white/40">Task ID</dt>
          <dd className="truncate text-white">{task.id}</dd>
        </div>
      </dl>

      {task.contract?.criteria && task.contract.criteria.length > 0 && (
        <div className="space-y-2">
          <h3 className="font-semibold text-white">Acceptance Criteria</h3>
          <div className="space-y-1.5">
            {task.contract.criteria.map((c, i) => (
              <div key={i} className="rounded border border-white/8 bg-[#141518] p-2.5">
                <div className="font-medium text-white">{c.name}</div>
                <div className="text-[11px] text-white/50">{c.description || 'No details recorded'}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

