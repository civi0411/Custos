import React from 'react';
import { 
  X, 
  ShieldCheck, 
  CheckCircle2, 
  AlertCircle, 
  DollarSign, 
  Pause, 
  RotateCcw,
  ExternalLink,
  FileCheck
} from 'lucide-react';

interface TaskDetailsModalProps {
  isOpen: boolean;
  onClose: () => void;
  task: {
    id: string;
    title: string;
    status: string;
    model: string;
    budgetUsed: number;
    budgetLimit: number;
    turnsUsed: number;
    maxTurns: number;
    actor: string;
    criteria?: Array<{
      id: string;
      label: string;
      status: 'pass' | 'fail' | 'uncertain' | 'pending';
      evidenceHash?: string;
    }>;
  };
}

export const TaskDetailsModal: React.FC<TaskDetailsModalProps> = ({
  isOpen,
  onClose,
  task
}) => {
  if (!isOpen) return null;

  const defaultCriteria = task.criteria || [
    {
      id: 'crit-1',
      label: 'Cargo build and unit tests pass with exit code 0',
      status: 'pass' as const,
      evidenceHash: 'cas://bafy2bzace4v3k...9a'
    },
    {
      id: 'crit-2',
      label: 'PermitGate::acquire_ticket verified with zero-IO proof closure',
      status: 'pass' as const,
      evidenceHash: 'ledger://signed_ticket_8045'
    },
    {
      id: 'crit-3',
      label: 'Boundary Integrity: No Taint::Untrusted instruction conversion',
      status: 'uncertain' as const,
      evidenceHash: 'pending-reconciliation'
    }
  ];

  const budgetPercent = Math.min(100, Math.round((task.budgetUsed / task.budgetLimit) * 100));

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-in fade-in duration-150">
      <div 
        className="w-full max-w-xl bg-[#0d1017] border border-[#202637] rounded-2xl shadow-2xl overflow-hidden flex flex-col font-sans max-h-[85vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-[#1c2232] flex items-center justify-between bg-[#111520]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-blue-500/10 border border-blue-500/25 flex items-center justify-center text-blue-400">
              <ShieldCheck className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-mono text-xs font-semibold text-white">{task.id}</span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                  {task.status.toUpperCase()}
                </span>
              </div>
              <p className="text-xs text-neutral-300 font-medium mt-0.5 truncate max-w-md">
                {task.title}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-neutral-400 hover:text-white hover:bg-white/5 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5 text-xs text-neutral-300">
          {/* Metrics Overview Grid */}
          <div className="grid grid-cols-3 gap-3">
            <div className="p-3 rounded-xl bg-[#141824] border border-[#202636] space-y-1">
              <span className="text-[10px] font-mono uppercase text-neutral-400">Actor Attribution (INV-01)</span>
              <div className="text-white font-semibold text-[11.5px] truncate">{task.actor}</div>
              <div className="text-[9.5px] text-neutral-500 font-mono">Bound to Kernel PID 4092</div>
            </div>

            <div className="p-3 rounded-xl bg-[#141824] border border-[#202636] space-y-1">
              <span className="text-[10px] font-mono uppercase text-neutral-400">Model Pin (INV-02)</span>
              <div className="text-emerald-400 font-mono text-[11.5px] truncate">{task.model}</div>
              <div className="text-[9.5px] text-neutral-500 font-mono">Fixed Temperature 0.2</div>
            </div>

            <div className="p-3 rounded-xl bg-[#141824] border border-[#202636] space-y-1">
              <span className="text-[10px] font-mono uppercase text-neutral-400">Turns</span>
              <div className="text-white font-mono text-[11.5px]">
                {task.turnsUsed} / {task.maxTurns}
              </div>
              <div className="text-[9.5px] text-neutral-500 font-mono">{task.maxTurns - task.turnsUsed} remaining</div>
            </div>
          </div>

          {/* Budget Meter */}
          <div className="p-4 rounded-xl bg-[#141824] border border-[#202636] space-y-2">
            <div className="flex items-center justify-between text-xs">
              <span className="font-semibold text-white flex items-center gap-1.5">
                <DollarSign className="w-3.5 h-3.5 text-amber-400" />
                Budget Utilization (INV-02 Boundary)
              </span>
              <span className="font-mono text-neutral-300">
                ${task.budgetUsed.toFixed(2)} / ${task.budgetLimit.toFixed(2)} ({budgetPercent}%)
              </span>
            </div>
            <div className="w-full h-2 rounded-full bg-[#1b2130] overflow-hidden">
              <div 
                className="h-full bg-gradient-to-r from-emerald-500 to-amber-500 rounded-full transition-all duration-300"
                style={{ width: `${budgetPercent}%` }}
              />
            </div>
            <div className="flex items-center justify-between text-[10px] text-neutral-500 font-mono">
              <span>Zero overage tolerance enforced by daemon kernel</span>
              <span>Tokens: ~18,420</span>
            </div>
          </div>

          {/* Acceptance Criteria & Evidence Gate (INV-05 & INV-06) */}
          <div className="space-y-2.5">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-white uppercase text-[11px] tracking-wider flex items-center gap-1.5">
                <FileCheck className="w-3.5 h-3.5 text-cyan-400" />
                Completion Gate Criteria (INV-05)
              </span>
              <span className="text-[10px] text-neutral-500 font-mono">2 / 3 Verified</span>
            </div>

            <div className="space-y-2">
              {defaultCriteria.map((crit) => (
                <div 
                  key={crit.id}
                  className="p-3 rounded-xl bg-[#111520] border border-[#1e2436] space-y-1.5"
                >
                  <div className="flex items-center justify-between">
                    <span className="text-[11.5px] font-medium text-neutral-200">
                      {crit.label}
                    </span>
                    {crit.status === 'pass' && (
                      <span className="px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/25 text-[10px] font-mono flex items-center gap-1">
                        <CheckCircle2 className="w-3 h-3" />
                        PASS
                      </span>
                    )}
                    {crit.status === 'uncertain' && (
                      <span className="px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/25 text-[10px] font-mono flex items-center gap-1" title="Explicit Uncertainty (INV-06)">
                        <AlertCircle className="w-3 h-3" />
                        UNCERTAIN (INV-06)
                      </span>
                    )}
                    {crit.status === 'pending' && (
                      <span className="px-2 py-0.5 rounded-full bg-neutral-800 text-neutral-400 text-[10px] font-mono">
                        PENDING
                      </span>
                    )}
                  </div>
                  {crit.evidenceHash && (
                    <div className="text-[10px] font-mono text-cyan-400/80 bg-cyan-950/20 px-2 py-1 rounded border border-cyan-800/30 flex items-center justify-between truncate">
                      <span className="truncate">Evidence CAS: {crit.evidenceHash}</span>
                      <ExternalLink className="w-3 h-3 shrink-0 ml-1 text-cyan-500" />
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-3.5 border-t border-[#1c2232] bg-[#111520] flex items-center justify-between">
          <div className="flex items-center gap-2">
            <button className="px-3 py-1.5 rounded-lg bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/25 text-amber-300 text-xs font-medium flex items-center gap-1.5 transition">
              <Pause className="w-3 h-3" />
              <span>Pause Task</span>
            </button>
            <button className="px-3 py-1.5 rounded-lg bg-red-500/10 hover:bg-red-500/20 border border-red-500/25 text-red-300 text-xs font-medium flex items-center gap-1.5 transition">
              <RotateCcw className="w-3 h-3" />
              <span>Abort & Rollback</span>
            </button>
          </div>

          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-xl bg-[#1e2436] hover:bg-[#272e45] text-white text-xs font-medium border border-[#2d354d] transition"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
