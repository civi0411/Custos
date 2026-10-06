import React, { useState } from 'react';
import { 
  X, 
  ShieldCheck, 
  CheckCircle2, 
  AlertCircle, 
  DollarSign, 
  Pause, 
  RotateCcw,
  ExternalLink,
  FileCheck,
  Layers,
  Terminal,
  Activity,
  Clock
} from 'lucide-react';

interface TaskDetailsModalProps {
  isOpen: boolean;
  onClose: () => void;
  task: {
    id: string;
    title: string;
    status: string;
    model: string;
    budgetUsed?: number;
    budgetLimit?: number;
    turnsUsed?: number;
    maxTurns?: number;
    actor?: string;
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
  const [activeTab, setActiveTab] = useState<'dag' | 'attempts' | 'criteria' | 'fidelity'>('dag');

  if (!isOpen) return null;

  // Orca O6: Task DAG Nodes
  const taskDagNodes = [
    {
      id: 'T-01',
      title: 'Invariant Contract Fencing Gate',
      status: 'completed',
      assignee: 'S2Planner',
      runtime: '1.2s',
      deps: []
    },
    {
      id: 'T-02',
      title: 'SIMD Vectorization of Workflow Dispatcher',
      status: 'running',
      assignee: 'Claude Code (S2-Worker)',
      runtime: '24.8s (Active)',
      deps: ['T-01']
    },
    {
      id: 'T-03',
      title: 'Zero-IO Proof Closure & Benchmark Suite',
      status: 'blocked',
      assignee: 'Custos Verifier',
      runtime: 'Pending',
      deps: ['T-02']
    }
  ];

  // Orca O6: Dispatch Attempts
  const dispatchAttempts = [
    {
      attemptId: 'DISPATCH-8041-A',
      taskId: 'T-01',
      worker: 'S2Planner',
      depth: 0,
      epoch: 1,
      mode: 'ClaudeStructuredSession',
      promptStatus: 'Delivered',
      outcome: 'Success (CAS: bafy2bzace4v3k...9a)',
      status: 'settled'
    },
    {
      attemptId: 'DISPATCH-8042-B',
      taskId: 'T-02',
      worker: 'Claude Code',
      depth: 1,
      epoch: 2,
      mode: 'PTY-Terminal-Mediated',
      promptStatus: 'Delivered',
      outcome: 'In-flight streaming (+38 -4 lines)',
      status: 'active'
    }
  ];

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

  const budgetUsed = task.budgetUsed ?? 0.084;
  const budgetLimit = task.budgetLimit ?? 1.0;
  const budgetPercent = Math.min(100, Math.round((budgetUsed / budgetLimit) * 100));

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm animate-in fade-in duration-150">
      <div 
        className="w-full max-w-3xl rounded-2xl overflow-hidden flex flex-col font-sans max-h-[88vh] shadow-2xl"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          border: '1px solid var(--color-border-default, #30363d)'
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div 
          className="px-6 py-4 flex items-center justify-between"
          style={{
            background: 'var(--color-surface-2, #1c2128)',
            borderBottom: '1px solid var(--color-border-default, #30363d)'
          }}
        >
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg flex items-center justify-center text-[#58a6ff]" style={{ background: 'rgba(88,166,255,0.15)', border: '1px solid rgba(88,166,255,0.3)' }}>
              <Layers className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-mono text-xs font-semibold text-white">{task.id}</span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-bold flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                  RUN #1 • ACTIVE
                </span>
                <span className="text-[10px] font-mono text-[#8b949e]">
                  DAG Convergence: Active
                </span>
              </div>
              <p className="text-xs text-neutral-200 font-medium mt-0.5 truncate max-w-lg">
                {task.title}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-white/5 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Tab Navigation (Orca ADE Standards) */}
        <div 
          className="flex items-center px-6 gap-2 text-xs font-medium"
          style={{
            background: 'var(--color-surface-1, #161b22)',
            borderBottom: '1px solid var(--color-border-muted, #21262d)'
          }}
        >
          <button
            onClick={() => setActiveTab('dag')}
            className="py-2.5 px-3 border-b-2 transition flex items-center gap-1.5"
            style={{
              borderColor: activeTab === 'dag' ? 'var(--color-coding, #3fb950)' : 'transparent',
              color: activeTab === 'dag' ? 'white' : 'var(--color-fg-muted, #8b949e)'
            }}
          >
            <Layers className="w-3.5 h-3.5" />
            <span>Task DAG (Orca O6)</span>
          </button>
          <button
            onClick={() => setActiveTab('attempts')}
            className="py-2.5 px-3 border-b-2 transition flex items-center gap-1.5"
            style={{
              borderColor: activeTab === 'attempts' ? 'var(--color-coding, #3fb950)' : 'transparent',
              color: activeTab === 'attempts' ? 'white' : 'var(--color-fg-muted, #8b949e)'
            }}
          >
            <Activity className="w-3.5 h-3.5" />
            <span>Dispatch Attempts (2)</span>
          </button>
          <button
            onClick={() => setActiveTab('criteria')}
            className="py-2.5 px-3 border-b-2 transition flex items-center gap-1.5"
            style={{
              borderColor: activeTab === 'criteria' ? 'var(--color-coding, #3fb950)' : 'transparent',
              color: activeTab === 'criteria' ? 'white' : 'var(--color-fg-muted, #8b949e)'
            }}
          >
            <FileCheck className="w-3.5 h-3.5" />
            <span>Evidence Gates (INV-05)</span>
          </button>
          <button
            onClick={() => setActiveTab('fidelity')}
            className="py-2.5 px-3 border-b-2 transition flex items-center gap-1.5"
            style={{
              borderColor: activeTab === 'fidelity' ? 'var(--color-coding, #3fb950)' : 'transparent',
              color: activeTab === 'fidelity' ? 'white' : 'var(--color-fg-muted, #8b949e)'
            }}
          >
            <Terminal className="w-3.5 h-3.5" />
            <span>Launch Fidelity (O3)</span>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5 text-xs text-neutral-300">
          {/* TAB 1: ORCA TASK DAG */}
          {activeTab === 'dag' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h4 className="font-semibold text-white text-sm">Deterministic Task Graph Execution</h4>
                  <p className="text-[11px] text-[#8b949e]">
                    Tasks are promoted atomically when all dependencies resolve. Atomic claims prevent race conditions.
                  </p>
                </div>
                <span className="font-mono text-[10px] px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-bold">
                  1 Ready • 1 Active • 1 Blocked
                </span>
              </div>

              {/* Visual DAG Nodes */}
              <div className="space-y-3">
                {taskDagNodes.map((node, i) => (
                  <div
                    key={node.id}
                    className="p-3.5 rounded-xl transition flex items-center justify-between"
                    style={{
                      background: node.status === 'running' ? 'var(--color-surface-2, #1c2128)' : 'var(--color-canvas, #0d1117)',
                      border: `1px solid ${node.status === 'running' ? 'var(--color-coding, #3fb950)' : 'var(--color-border-default, #30363d)'}`
                    }}
                  >
                    <div className="flex items-center gap-3">
                      <div 
                        className="w-7 h-7 rounded-lg flex items-center justify-center font-mono font-bold text-xs"
                        style={{
                          background: node.status === 'completed' ? 'rgba(63,185,80,0.15)' : node.status === 'running' ? 'rgba(88,166,255,0.15)' : 'rgba(110,118,129,0.15)',
                          color: node.status === 'completed' ? '#3fb950' : node.status === 'running' ? '#58a6ff' : '#8b949e',
                        }}
                      >
                        {i + 1}
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-mono font-bold text-white text-[12px]">{node.id}</span>
                          <span className="font-medium text-white">{node.title}</span>
                        </div>
                        <div className="flex items-center gap-2 text-[10.5px] text-[#8b949e] font-mono mt-0.5">
                          <span>Assignee: {node.assignee}</span>
                          <span>•</span>
                          <span>Duration: {node.runtime}</span>
                          {node.deps.length > 0 && (
                            <>
                              <span>•</span>
                              <span>Depends on: <code className="text-[#3fb950]">{node.deps.join(', ')}</code></span>
                            </>
                          )}
                        </div>
                      </div>
                    </div>

                    <div>
                      {node.status === 'completed' && (
                        <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1">
                          <CheckCircle2 className="w-3 h-3" />
                          COMPLETED
                        </span>
                      )}
                      {node.status === 'running' && (
                        <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-blue-500/10 text-blue-400 border border-blue-500/20 flex items-center gap-1">
                          <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse"></span>
                          DISPATCHING
                        </span>
                      )}
                      {node.status === 'blocked' && (
                        <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-neutral-800 text-neutral-400 flex items-center gap-1">
                          <Clock className="w-3 h-3" />
                          WAITING DEPS
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>

              {/* Orca Convergence Check */}
              <div 
                className="p-3 rounded-lg flex items-center justify-between text-[11px] font-mono"
                style={{
                  background: 'var(--color-surface-2, #1c2128)',
                  border: '1px solid var(--color-border-muted, #21262d)'
                }}
              >
                <div className="flex items-center gap-2">
                  <ShieldCheck className="w-4 h-4 text-[#3fb950]" />
                  <span>DAG Convergence Monitor: Graph is progressing normally. 0 stuck deadlocks detected.</span>
                </div>
                <span className="text-[#3fb950]">Status: OK</span>
              </div>
            </div>
          )}

          {/* TAB 2: DISPATCH ATTEMPTS */}
          {activeTab === 'attempts' && (
            <div className="space-y-4">
              <div>
                <h4 className="font-semibold text-white text-sm">Orchestration Dispatch Attempts (Orca O6)</h4>
                <p className="text-[11px] text-[#8b949e]">
                  Atomic worker claims stamped with depth, consumer fencing generation, and prompt delivery receipts.
                </p>
              </div>

              <div className="space-y-3 font-mono text-[11px]">
                {dispatchAttempts.map((att) => (
                  <div 
                    key={att.attemptId}
                    className="p-3.5 rounded-xl space-y-2"
                    style={{
                      background: 'var(--color-canvas, #0d1117)',
                      border: '1px solid var(--color-border-default, #30363d)'
                    }}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="font-bold text-white">{att.attemptId}</span>
                        <span className="px-1.5 py-0.2 rounded bg-surface-2 text-neutral-300 border border-surface-border">
                          Task: {att.taskId}
                        </span>
                        <span className="text-[#3fb950]">Depth: {att.depth}</span>
                        <span className="text-[#58a6ff]">Epoch: {att.epoch}</span>
                      </div>
                      <span className={`px-2 py-0.5 rounded text-[10px] font-bold ${att.status === 'active' ? 'bg-emerald-500/10 text-emerald-400' : 'bg-neutral-800 text-neutral-400'}`}>
                        {att.status.toUpperCase()}
                      </span>
                    </div>

                    <div className="grid grid-cols-2 gap-2 text-[#8b949e] text-[10.5px]">
                      <div>Assignee: <span className="text-white">{att.worker}</span></div>
                      <div>Mode: <span className="text-white">{att.mode}</span></div>
                      <div>Prompt Delivery: <span className="text-[#3fb950]">{att.promptStatus}</span></div>
                      <div>Outcome: <span className="text-neutral-200">{att.outcome}</span></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* TAB 3: EVIDENCE GATES */}
          {activeTab === 'criteria' && (
            <div className="space-y-4">
              <div>
                <h4 className="font-semibold text-white text-sm">Completion Gate Criteria (INV-05)</h4>
                <p className="text-[11px] text-[#8b949e]">
                  Worker completion is an operational status; task acceptance requires verifiable evidence receipts.
                </p>
              </div>

              <div className="space-y-2.5">
                {defaultCriteria.map((crit) => (
                  <div 
                    key={crit.id}
                    className="p-3.5 rounded-xl space-y-1.5"
                    style={{
                      background: 'var(--color-canvas, #0d1117)',
                      border: '1px solid var(--color-border-default, #30363d)'
                    }}
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-[12px] font-medium text-neutral-200">
                        {crit.label}
                      </span>
                      {crit.status === 'pass' && (
                        <span className="px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/25 text-[10px] font-mono font-bold flex items-center gap-1">
                          <CheckCircle2 className="w-3 h-3" />
                          PASS
                        </span>
                      )}
                      {crit.status === 'uncertain' && (
                        <span className="px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/25 text-[10px] font-mono font-bold flex items-center gap-1" title="Explicit Uncertainty (INV-06)">
                          <AlertCircle className="w-3 h-3" />
                          UNCERTAIN (INV-06)
                        </span>
                      )}
                    </div>
                    {crit.evidenceHash && (
                      <div className="text-[10px] font-mono text-cyan-400/90 bg-cyan-950/20 px-2 py-1 rounded border border-cyan-800/30 flex items-center justify-between truncate">
                        <span className="truncate">Evidence CAS: {crit.evidenceHash}</span>
                        <ExternalLink className="w-3 h-3 shrink-0 ml-1 text-cyan-500" />
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* TAB 4: LAUNCH FIDELITY */}
          {activeTab === 'fidelity' && (
            <div className="space-y-4">
              <div>
                <h4 className="font-semibold text-white text-sm">Agent Launch Fidelity & Surface Selection (Orca O3)</h4>
                <p className="text-[11px] text-[#8b949e]">
                  Displays the contract truth between requested launch mode and actual executed surface.
                </p>
              </div>

              <div 
                className="p-4 rounded-xl space-y-3 font-mono text-[11px]"
                style={{
                  background: 'var(--color-canvas, #0d1117)',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                <div className="flex items-center justify-between pb-2" style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}>
                  <span className="text-[#8b949e]">Requested Mode</span>
                  <span className="text-white font-semibold">ClaudeStructuredSession</span>
                </div>
                <div className="flex items-center justify-between pb-2" style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}>
                  <span className="text-[#8b949e]">Actual Mode</span>
                  <span className="text-[#3fb950] font-semibold">PTY-Terminal-Mediated (Local Subprocess)</span>
                </div>
                <div className="flex items-center justify-between pb-2" style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}>
                  <span className="text-[#8b949e]">Downgrade / Route Rationale</span>
                  <span className="text-neutral-300">Custos PermitGate AST tool-call interception requested</span>
                </div>
                <div className="flex items-center justify-between pb-2" style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}>
                  <span className="text-[#8b949e]">Tool Assurance Level</span>
                  <span className="text-[#3fb950] font-bold">[Custos-Mediated AST]</span>
                </div>
                <div className="flex items-center justify-between pb-2" style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}>
                  <span className="text-[#8b949e]">Prompt Delivery Receipt</span>
                  <span className="text-[#58a6ff]">receipt://sha256_94f83b1029c</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[#8b949e]">Host Route</span>
                  <span className="text-white">Local macOS (darwin-arm64)</span>
                </div>
              </div>
            </div>
          )}

          {/* Budget Meter */}
          <div 
            className="p-4 rounded-xl space-y-2"
            style={{
              background: 'var(--color-surface-2, #1c2128)',
              border: '1px solid var(--color-border-default, #30363d)'
            }}
          >
            <div className="flex items-center justify-between text-xs">
              <span className="font-semibold text-white flex items-center gap-1.5">
                <DollarSign className="w-3.5 h-3.5 text-amber-400" />
                Budget & Token Telemetry (Orca O10)
              </span>
              <span className="font-mono text-neutral-300">
                ${budgetUsed.toFixed(2)} / ${budgetLimit.toFixed(2)} ({budgetPercent}%)
              </span>
            </div>
            <div className="w-full h-2 rounded-full bg-[#0d1117] overflow-hidden">
              <div 
                className="h-full bg-gradient-to-r from-emerald-500 to-amber-500 rounded-full transition-all duration-300"
                style={{ width: `${budgetPercent}%` }}
              />
            </div>
            <div className="flex items-center justify-between text-[10px] text-neutral-500 font-mono">
              <span>Tokens: 18,420 (Prompt: 14,200 · Completion: 4,220)</span>
              <span>Latency p95: 184ms</span>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div 
          className="px-6 py-3.5 flex items-center justify-between"
          style={{
            background: 'var(--color-surface-2, #1c2128)',
            borderTop: '1px solid var(--color-border-default, #30363d)'
          }}
        >
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
            className="px-4 py-1.5 rounded-lg text-white text-xs font-medium transition"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              border: '1px solid var(--color-border-default, #30363d)'
            }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

export default TaskDetailsModal;
