import React, { useState } from 'react';
import { 
  X, 
  ShieldCheck, 
  DollarSign, 
  RotateCw, 
  CheckSquare, 
  Layers, 
  Lock, 
  Play
} from 'lucide-react';

export interface DelegateTaskData {
  title: string;
  goal: string;
  model: string;
  localOnly: boolean;
  budgetUsd: number;
  maxTurns: number;
  criteria: string[];
  strategy: 'single-model' | 'multi-worker';
  budgetLimit?: number;
  pack?: string;
  autonomous?: boolean;
}

export interface DelegateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  sessionId?: string;
  activeSessionName?: string;
  onSubmit?: (taskData: DelegateTaskData) => void;
  onDelegateTask?: (taskData: DelegateTaskData) => void;
}

export const DelegateTaskModal: React.FC<DelegateTaskModalProps> = ({
  isOpen,
  onClose,
  sessionId = 'vi',
  activeSessionName,
  onSubmit,
  onDelegateTask
}) => {
  const [title, setTitle] = useState('Implement Invariant Ticket Gate in Dispatcher');
  const [goal, setGoal] = useState(
    'Refactor claim_ready_task in custos-runtime to acquire an atomic ticket under the Invariant Gate, verifying pre-conditions and generating CAS evidence.'
  );
  const [selectedModel, setSelectedModel] = useState('claude-3-7-sonnet');
  const [localOnly, setLocalOnly] = useState(true);
  const [budgetUsd, setBudgetUsd] = useState(1.50);
  const [maxTurns, setMaxTurns] = useState(8);
  const [strategy, setStrategy] = useState<'single-model' | 'multi-worker'>('single-model');
  const [criteria, setCriteria] = useState<string[]>([
    'Cargo build and unit tests pass with exit code 0',
    'PermitGate::acquire_ticket verified with zero-IO proof',
    'Evidence CAS hash registered in completion ledger'
  ]);
  const [newCriterion, setNewCriterion] = useState('');

  if (!isOpen) return null;

  const handleAddCriterion = () => {
    if (!newCriterion.trim()) return;
    setCriteria([...criteria, newCriterion.trim()]);
    setNewCriterion('');
  };

  const handleRemoveCriterion = (index: number) => {
    setCriteria(criteria.filter((_, i) => i !== index));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !goal.trim()) return;
    const taskPayload = {
      title,
      goal,
      model: selectedModel,
      localOnly,
      budgetUsd,
      budgetLimit: budgetUsd,
      maxTurns,
      criteria,
      strategy,
      pack: 'Coding/Refactor',
      autonomous: true
    };
    if (onSubmit) onSubmit(taskPayload);
    else if (onDelegateTask) onDelegateTask(taskPayload);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150">
      <div 
        className="w-full max-w-2xl bg-surface-1 border border-border-default rounded-2xl shadow-2xl overflow-hidden flex flex-col font-sans max-h-[90vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-border-muted flex items-center justify-between bg-surface-1">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-surface-2 border border-border-default flex items-center justify-center text-emerald-500">
              <ShieldCheck className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-semibold text-fg-editor">Delegate Autonomous Task</h2>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-500 border border-emerald-500/20 font-medium">
                  INV-02 Sandbox Bound
                </span>
              </div>
              <p className="text-xs text-fg-muted mt-0.5">
                Binds session <code className="text-fg-editor font-mono">#{sessionId}</code>{activeSessionName ? ` (${activeSessionName})` : ''} to a supervised background execution task
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-6 space-y-4 text-xs text-fg-editor">
          {/* 1. Title & Goal */}
          <div className="space-y-3">
            <div>
              <label className="block text-xs font-semibold text-fg-editor uppercase tracking-wider mb-1">
                Task Title
              </label>
              <input
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="e.g. Implement Invariant Gate Ticket Verification"
                className="w-full bg-surface-0 border border-border-default rounded-lg px-3.5 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none focus:border-border-emphasis font-sans text-xs transition"
                required
              />
            </div>

            <div>
              <label className="block text-xs font-semibold text-fg-editor uppercase tracking-wider mb-1">
                Autonomous Goal & Scope Specification
              </label>
              <textarea
                value={goal}
                onChange={(e) => setGoal(e.target.value)}
                rows={3}
                placeholder="Describe what the agent should accomplish..."
                className="w-full bg-surface-0 border border-border-default rounded-lg p-3 text-fg-editor placeholder-fg-subtle focus:outline-none focus:border-border-emphasis font-sans text-xs resize-none transition"
                required
              />
            </div>
          </div>

          {/* 2. Boundary Controls (INV-02 Invariants) */}
          <div className="p-4 rounded-xl bg-surface-0 border border-border-muted space-y-3">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-fg-editor flex items-center gap-1.5 text-xs">
                <Lock className="w-3.5 h-3.5 workbench-accent" />
                Invariant Boundary Constraints (INV-02)
              </span>
              <span className="text-[10px] text-fg-subtle font-mono">Immutable by Agent</span>
            </div>

            <div className="grid grid-cols-2 gap-3 pt-1">
              {/* LocalOnly Fence Toggle */}
              <div 
                onClick={() => setLocalOnly(!localOnly)}
                className={`p-3 rounded-xl border cursor-pointer transition flex items-start gap-2.5 ${
                  localOnly
                    ? 'bg-surface-2 border-border-emphasis text-fg-editor'
                    : 'bg-surface-1 border-border-muted text-fg-muted hover:border-border-default'
                }`}
              >
                <input
                  type="checkbox"
                  checked={localOnly}
                  onChange={() => {}}
                  className="mt-0.5 rounded border-border-default text-[var(--workbench-accent)] accent-[var(--workbench-accent)]"
                />
                <div>
                  <div className="font-semibold text-xs text-fg-editor">LocalOnly Fence</div>
                  <div className="text-[11px] text-fg-muted leading-normal mt-0.5">
                    Strict zero-network sandbox. All external I/O requires explicit Permit approval.
                  </div>
                </div>
              </div>

              {/* Strategy: Single Model Baseline (INV-10) vs Multi-Worker */}
              <div 
                onClick={() => setStrategy(strategy === 'single-model' ? 'multi-worker' : 'single-model')}
                className="p-3 rounded-xl border border-border-default bg-surface-1 cursor-pointer hover:border-border-emphasis transition flex items-start gap-2.5"
              >
                <Layers className="w-4 h-4 text-purple-400 mt-0.5 shrink-0" />
                <div>
                  <div className="flex items-center gap-1.5">
                    <span className="font-semibold text-xs text-fg-editor">
                      {strategy === 'single-model' ? 'Single Model Baseline' : 'Multi-Worker Slice'}
                    </span>
                    <span className="text-[9px] font-mono px-1 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20">
                      INV-10
                    </span>
                  </div>
                  <div className="text-[11px] text-fg-muted leading-normal mt-0.5">
                    {strategy === 'single-model'
                      ? 'Single frontier model holds full Task context without split overhead.'
                      : 'Decomposed sub-tasks with strict permission diminishment.'}
                  </div>
                </div>
              </div>
            </div>

            {/* Model Pinning & Budget Grid */}
            <div className="grid grid-cols-3 gap-3 pt-2">
              <div>
                <label className="block text-[11px] font-medium text-fg-muted mb-1">
                  Model Pin
                </label>
                <select
                  value={selectedModel}
                  onChange={(e) => setSelectedModel(e.target.value)}
                  className="w-full bg-surface-1 border border-border-default rounded-lg px-2.5 py-1.5 text-xs text-fg-editor focus:outline-none focus:border-border-emphasis"
                >
                  <option value="claude-3-7-sonnet">Claude 3.7 Sonnet</option>
                  <option value="o3-mini">OpenAI o3-mini</option>
                  <option value="gemini-2.5-pro">Gemini 2.5 Pro</option>
                  <option value="deepseek-v3">DeepSeek V3 (Local)</option>
                </select>
              </div>

              <div>
                <label className="block text-[11px] font-medium text-fg-muted mb-1">
                  Budget Cap (USD)
                </label>
                <div className="relative">
                  <DollarSign className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle" />
                  <input
                    type="number"
                    step="0.25"
                    min="0.25"
                    max="50"
                    value={budgetUsd}
                    onChange={(e) => setBudgetUsd(parseFloat(e.target.value) || 0)}
                    className="w-full bg-surface-1 border border-border-default rounded-lg pl-7 pr-2.5 py-1.5 text-xs text-fg-editor focus:outline-none focus:border-border-emphasis font-mono"
                  />
                </div>
              </div>

              <div>
                <label className="block text-[11px] font-medium text-fg-muted mb-1">
                  Max Turns
                </label>
                <div className="relative">
                  <RotateCw className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle" />
                  <input
                    type="number"
                    min="1"
                    max="50"
                    value={maxTurns}
                    onChange={(e) => setMaxTurns(parseInt(e.target.value, 10) || 1)}
                    className="w-full bg-surface-1 border border-border-default rounded-lg pl-7 pr-2.5 py-1.5 text-xs text-fg-editor focus:outline-none focus:border-border-emphasis font-mono"
                  />
                </div>
              </div>
            </div>
          </div>

          {/* 3. Evidence-Backed Acceptance Criteria (INV-05) */}
          <div className="space-y-2.5">
            <div className="flex items-center justify-between">
              <label className="text-xs font-semibold text-fg-editor uppercase tracking-wider flex items-center gap-1.5">
                <CheckSquare className="w-3.5 h-3.5 workbench-accent" />
                Acceptance Criteria & Evidence Gate (INV-05)
              </label>
              <span className="text-[10px] text-fg-subtle font-mono">Requires Valid Evidence</span>
            </div>

            <div className="space-y-1.5">
              {criteria.map((item, idx) => (
                <div 
                  key={idx} 
                  className="flex items-center justify-between p-2 rounded-lg bg-surface-0 border border-border-muted text-xs"
                >
                  <div className="flex items-center gap-2 truncate">
                    <span className="w-4 h-4 rounded bg-surface-2 text-fg-editor font-mono text-[10px] flex items-center justify-center font-bold">
                      {idx + 1}
                    </span>
                    <span className="text-fg-editor truncate">{item}</span>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleRemoveCriterion(idx)}
                    className="text-fg-muted hover:text-rose-500 p-1 rounded transition"
                  >
                    <X className="w-3 h-3" />
                  </button>
                </div>
              ))}
            </div>

            {/* Add Criterion Input */}
            <div className="flex items-center gap-2 pt-1">
              <input
                type="text"
                value={newCriterion}
                onChange={(e) => setNewCriterion(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); handleAddCriterion(); } }}
                placeholder="Add verification criterion (e.g., 'Integration test passes with CAS receipt')..."
                className="flex-1 bg-surface-0 border border-border-default rounded-lg px-3 py-1.5 text-xs text-fg-editor placeholder-fg-subtle focus:outline-none focus:border-border-emphasis"
              />
              <button
                type="button"
                onClick={handleAddCriterion}
                className="px-3 py-1.5 rounded-lg bg-surface-2 hover:bg-surface-3 text-fg-editor text-xs font-medium border border-border-default transition"
              >
                + Add
              </button>
            </div>
          </div>
        </form>

        {/* Footer Actions */}
        <div className="px-6 py-3.5 border-t border-border-muted bg-surface-1 flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs text-fg-muted">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            <span>S2 Autonomous Daemon Ready • Turn Anchor Registered</span>
          </div>

          <div className="flex items-center gap-2.5">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-1.5 rounded-lg border border-border-default text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition text-xs font-medium"
            >
              Cancel
            </button>
            <button
              onClick={handleSubmit}
              className="workbench-primary-action px-5 py-1.5 rounded-lg font-medium text-xs flex items-center gap-1.5 transition"
            >
              <Play className="w-3.5 h-3.5 fill-current" />
              <span>Launch Autonomous Task</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
