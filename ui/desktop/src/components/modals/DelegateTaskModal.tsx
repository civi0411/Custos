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
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-sm animate-in fade-in duration-150">
      <div 
        className="w-full max-w-2xl bg-[#0d1017] border border-[#202637] rounded-2xl shadow-2xl overflow-hidden flex flex-col font-sans max-h-[90vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-[#1c2232] flex items-center justify-between bg-[#111520]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/25 flex items-center justify-center text-emerald-400">
              <ShieldCheck className="w-4 h-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-bold text-white tracking-tight">Delegate S2 Autonomous Task</h2>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                  INV-02 Sandbox Bound
                </span>
              </div>
              <p className="text-[11px] text-neutral-400 mt-0.5">
                Binds session <code className="text-neutral-300 font-mono">#{sessionId}</code>{activeSessionName ? ` (${activeSessionName})` : ''} to a supervised background execution task
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

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-6 space-y-5 text-xs text-neutral-300">
          {/* 1. Title & Goal */}
          <div className="space-y-3">
            <div>
              <label className="block text-[11px] font-semibold text-neutral-200 uppercase tracking-wider mb-1">
                Task Title
              </label>
              <input
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="e.g. Implement Invariant Gate Ticket Verification"
                className="w-full bg-[#151926] border border-[#252c40] rounded-xl px-3.5 py-2 text-white placeholder-neutral-500 focus:outline-none focus:border-brand-blue font-sans text-xs transition"
                required
              />
            </div>

            <div>
              <label className="block text-[11px] font-semibold text-neutral-200 uppercase tracking-wider mb-1">
                Autonomous Goal & Scope Specification
              </label>
              <textarea
                value={goal}
                onChange={(e) => setGoal(e.target.value)}
                rows={3}
                placeholder="Describe what the agent should accomplish..."
                className="w-full bg-[#151926] border border-[#252c40] rounded-xl p-3 text-white placeholder-neutral-500 focus:outline-none focus:border-brand-blue font-sans text-xs resize-none transition"
                required
              />
            </div>
          </div>

          {/* 2. Boundary Controls (INV-02 Invariants) */}
          <div className="p-4 rounded-xl bg-[#111522] border border-[#202639] space-y-3">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-white flex items-center gap-1.5 text-[11px]">
                <Lock className="w-3.5 h-3.5 text-amber-400" />
                Invariant Boundary Constraints (INV-02)
              </span>
              <span className="text-[10px] text-neutral-500 font-mono">Immutable by Agent</span>
            </div>

            <div className="grid grid-cols-2 gap-3 pt-1">
              {/* LocalOnly Fence Toggle */}
              <div 
                onClick={() => setLocalOnly(!localOnly)}
                className={`p-3 rounded-xl border cursor-pointer transition flex items-start gap-2.5 ${
                  localOnly
                    ? 'bg-emerald-500/5 border-emerald-500/30 text-white'
                    : 'bg-[#151926] border-[#252c40] text-neutral-400 hover:border-neutral-500'
                }`}
              >
                <input
                  type="checkbox"
                  checked={localOnly}
                  onChange={() => {}}
                  className="mt-0.5 rounded border-neutral-600 bg-neutral-800 text-emerald-500"
                />
                <div>
                  <div className="font-semibold text-[11.5px] text-white">LocalOnly Fence</div>
                  <div className="text-[10.5px] text-neutral-400 leading-normal mt-0.5">
                    Strict zero-network sandbox. All external I/O requires explicit Permit approval.
                  </div>
                </div>
              </div>

              {/* Strategy: Single Model Baseline (INV-10) vs Multi-Worker */}
              <div 
                onClick={() => setStrategy(strategy === 'single-model' ? 'multi-worker' : 'single-model')}
                className="p-3 rounded-xl border border-[#252c40] bg-[#151926] cursor-pointer hover:border-neutral-500 transition flex items-start gap-2.5"
              >
                <Layers className="w-4 h-4 text-purple-400 mt-0.5 shrink-0" />
                <div>
                  <div className="flex items-center gap-1.5">
                    <span className="font-semibold text-[11.5px] text-white">
                      {strategy === 'single-model' ? 'Single Model Baseline' : 'Multi-Worker Slice'}
                    </span>
                    <span className="text-[9px] font-mono px-1 rounded bg-purple-500/20 text-purple-300">
                      INV-10
                    </span>
                  </div>
                  <div className="text-[10.5px] text-neutral-400 leading-normal mt-0.5">
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
                <label className="block text-[10.5px] font-medium text-neutral-400 mb-1">
                  Model Pin
                </label>
                <select
                  value={selectedModel}
                  onChange={(e) => setSelectedModel(e.target.value)}
                  className="w-full bg-[#151926] border border-[#252c40] rounded-lg px-2.5 py-1.5 text-xs text-white focus:outline-none focus:border-brand-blue"
                >
                  <option value="claude-3-7-sonnet">Claude 3.7 Sonnet</option>
                  <option value="o3-mini">OpenAI o3-mini</option>
                  <option value="gemini-2.5-pro">Gemini 2.5 Pro</option>
                  <option value="deepseek-v3">DeepSeek V3 (Local)</option>
                </select>
              </div>

              <div>
                <label className="block text-[10.5px] font-medium text-neutral-400 mb-1">
                  Budget Cap (USD)
                </label>
                <div className="relative">
                  <DollarSign className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
                  <input
                    type="number"
                    step="0.25"
                    min="0.25"
                    max="50"
                    value={budgetUsd}
                    onChange={(e) => setBudgetUsd(parseFloat(e.target.value) || 0)}
                    className="w-full bg-[#151926] border border-[#252c40] rounded-lg pl-7 pr-2.5 py-1.5 text-xs text-white focus:outline-none focus:border-brand-blue font-mono"
                  />
                </div>
              </div>

              <div>
                <label className="block text-[10.5px] font-medium text-neutral-400 mb-1">
                  Max Turns
                </label>
                <div className="relative">
                  <RotateCw className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
                  <input
                    type="number"
                    min="1"
                    max="50"
                    value={maxTurns}
                    onChange={(e) => setMaxTurns(parseInt(e.target.value, 10) || 1)}
                    className="w-full bg-[#151926] border border-[#252c40] rounded-lg pl-7 pr-2.5 py-1.5 text-xs text-white focus:outline-none focus:border-brand-blue font-mono"
                  />
                </div>
              </div>
            </div>
          </div>

          {/* 3. Evidence-Backed Acceptance Criteria (INV-05) */}
          <div className="space-y-2.5">
            <div className="flex items-center justify-between">
              <label className="text-[11px] font-semibold text-neutral-200 uppercase tracking-wider flex items-center gap-1.5">
                <CheckSquare className="w-3.5 h-3.5 text-cyan-400" />
                Acceptance Criteria & Evidence Gate (INV-05)
              </label>
              <span className="text-[10px] text-cyan-400 font-mono">Requires Valid Evidence</span>
            </div>

            <div className="space-y-1.5">
              {criteria.map((item, idx) => (
                <div 
                  key={idx} 
                  className="flex items-center justify-between p-2 rounded-lg bg-[#141824] border border-[#202636] text-[11.5px]"
                >
                  <div className="flex items-center gap-2 truncate">
                    <span className="w-4 h-4 rounded bg-cyan-500/15 text-cyan-400 font-mono text-[10px] flex items-center justify-center font-bold">
                      {idx + 1}
                    </span>
                    <span className="text-neutral-200 truncate">{item}</span>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleRemoveCriterion(idx)}
                    className="text-neutral-500 hover:text-red-400 p-1 rounded transition"
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
                className="flex-1 bg-[#151926] border border-[#252c40] rounded-lg px-3 py-1.5 text-xs text-white placeholder-neutral-500 focus:outline-none focus:border-brand-blue"
              />
              <button
                type="button"
                onClick={handleAddCriterion}
                className="px-3 py-1.5 rounded-lg bg-[#1e2538] hover:bg-[#252e46] text-neutral-200 hover:text-white text-xs font-medium border border-[#2d3650] transition"
              >
                + Add
              </button>
            </div>
          </div>
        </form>

        {/* Footer Actions */}
        <div className="px-6 py-3.5 border-t border-[#1c2232] bg-[#111520] flex items-center justify-between">
          <div className="flex items-center gap-2 text-[10.5px] text-neutral-400">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>S2 Autonomous Daemon Ready • Turn Anchor Registered</span>
          </div>

          <div className="flex items-center gap-2.5">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-1.5 rounded-xl border border-[#252c40] text-neutral-300 hover:text-white hover:bg-white/5 transition text-xs font-medium"
            >
              Cancel
            </button>
            <button
              onClick={handleSubmit}
              className="px-5 py-1.5 rounded-xl bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-medium text-xs shadow-lg shadow-emerald-900/30 flex items-center gap-1.5 transition"
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
