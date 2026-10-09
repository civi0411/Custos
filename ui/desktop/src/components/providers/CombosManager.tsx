import React, { useState } from 'react';
import {
  Layers,
  Plus,
  Play,
  Trash2,
  Edit2,
  ArrowRight,
  Shuffle,
  ShieldCheck,
  CheckCircle2,
  MoveUp,
  MoveDown,
  X,
  DollarSign
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { ModelCombo, ComboStrategy, ComboModelTarget, ProviderServiceId } from '@/types';
import { CANONICAL_MODELS, ProviderIcon, getProviderInfo } from '@/components/chat/ModelSelector';

export const CombosManager: React.FC = () => {
  const { combos, handleCreateCombo, handleUpdateCombo, handleDeleteCombo, showToast } = useAppContext();

  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [editingCombo, setEditingCombo] = useState<ModelCombo | null>(null);

  // Form State
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [strategy, setStrategy] = useState<ComboStrategy>('fallback');
  const [targets, setTargets] = useState<ComboModelTarget[]>([]);
  const [selectedAddModel, setSelectedAddModel] = useState('');

  // Simulation State
  const [simulatingComboId, setSimulatingComboId] = useState<string | null>(null);
  const [simResult, setSimResult] = useState<{
    comboId: string;
    step: string;
    message: string;
    success: boolean;
  } | null>(null);

  const openCreateModal = () => {
    setName('');
    setDescription('');
    setStrategy('fallback');
    setTargets([
      {
        modelId: 'claude-3-7-sonnet',
        modelName: 'Claude 3.7 Sonnet',
        providerId: 'anthropic',
        providerName: 'Anthropic',
        priority: 1,
      },
      {
        modelId: 'gpt-4o',
        modelName: 'GPT-4o',
        providerId: 'openai',
        providerName: 'OpenAI',
        priority: 2,
      },
    ]);
    setSelectedAddModel(CANONICAL_MODELS[2]?.id || 'gemini-2.5-flash');
    setEditingCombo(null);
    setIsCreateModalOpen(true);
  };

  const openEditModal = (combo: ModelCombo) => {
    setEditingCombo(combo);
    setName(combo.name);
    setDescription(combo.description);
    setStrategy(combo.strategy);
    setTargets([...combo.targets]);
    setSelectedAddModel(CANONICAL_MODELS[0]?.id || '');
    setIsCreateModalOpen(true);
  };

  const handleAddTarget = () => {
    if (!selectedAddModel) return;
    const model = CANONICAL_MODELS.find((m) => m.id === selectedAddModel);
    if (!model) return;

    if (targets.some((t) => t.modelId === model.id)) {
      showToast('Model is already in this combo pipeline');
      return;
    }

    const newTarget: ComboModelTarget = {
      modelId: model.id,
      modelName: model.name,
      providerId: model.provider as ProviderServiceId,
      providerName: model.providerName,
      priority: targets.length + 1,
      weight: strategy === 'round_robin' ? Math.round(100 / (targets.length + 1)) : undefined,
    };

    setTargets([...targets, newTarget]);
  };

  const handleRemoveTarget = (index: number) => {
    const next = targets.filter((_, i) => i !== index).map((t, idx) => ({
      ...t,
      priority: idx + 1,
    }));
    setTargets(next);
  };

  const handleMoveTarget = (index: number, direction: 'up' | 'down') => {
    if (direction === 'up' && index === 0) return;
    if (direction === 'down' && index === targets.length - 1) return;

    const newTargets = [...targets];
    const targetIdx = direction === 'up' ? index - 1 : index + 1;
    const temp = newTargets[index];
    newTargets[index] = newTargets[targetIdx];
    newTargets[targetIdx] = temp;

    // Re-index priorities
    setTargets(newTargets.map((t, idx) => ({ ...t, priority: idx + 1 })));
  };

  const handleSave = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      showToast('Please provide a combo name');
      return;
    }
    if (targets.length === 0) {
      showToast('Add at least one model target to the combo');
      return;
    }

    if (editingCombo) {
      handleUpdateCombo(editingCombo.id, {
        name,
        description,
        strategy,
        targets,
      });
    } else {
      handleCreateCombo({
        name,
        description,
        strategy,
        targets,
        enabled: true,
      });
    }

    setIsCreateModalOpen(false);
  };

  const handleSimulateRoute = (combo: ModelCombo) => {
    setSimulatingComboId(combo.id);
    setSimResult(null);

    setTimeout(() => {
      if (combo.strategy === 'fallback') {
        const primary = combo.targets[0]?.modelName || 'Primary Model';
        const fallback = combo.targets[1]?.modelName || 'Fallback Model';
        setSimResult({
          comboId: combo.id,
          step: 'Failover Triggered',
          message: `Diverted from ${primary} (simulated HTTP 429 Rate Limit) -> Route dispatched to ${fallback} (HTTP 200 OK, latency: 41ms)`,
          success: true,
        });
      } else if (combo.strategy === 'round_robin') {
        const chosen = combo.targets[Math.floor(Math.random() * combo.targets.length)]?.modelName || 'Target';
        setSimResult({
          comboId: combo.id,
          step: 'Balanced Route Dispatched',
          message: `Load balancer routed request to ${chosen} based on weight distribution (HTTP 200 OK, latency: 33ms)`,
          success: true,
        });
      } else {
        const cheapest = combo.targets[combo.targets.length - 1]?.modelName || 'Cost-efficient target';
        setSimResult({
          comboId: combo.id,
          step: 'Cost Optimizer Routed',
          message: `Dispatched to ${cheapest} saving ~74% token cost per million (HTTP 200 OK, latency: 29ms)`,
          success: true,
        });
      }
      setSimulatingComboId(null);
    }, 600);
  };

  return (
    <div className="space-y-6">
      {/* Top Action Bar */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 p-4 rounded-xl border border-border-default bg-surface-1">
        <div>
          <h2 className="text-sm font-semibold text-fg-editor flex items-center gap-2">
            <Layers className="w-4 h-4 text-workbench-accent" />
            <span>Virtual Model Routing Combos</span>
            <span className="text-[11px] font-mono px-2 py-0.5 rounded-full bg-surface-2 border border-border-muted text-fg-subtle">
              {combos.length} Combos
            </span>
          </h2>
          <p className="text-xs text-fg-muted mt-0.5">
            OmniRoute / 9Router architecture: Combine models into high-availability fallback chains and load balancers.
          </p>
        </div>

        <button
          onClick={openCreateModal}
          className="workbench-primary-action px-3 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 cursor-pointer shrink-0 shadow-xs"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>New Routing Combo</span>
        </button>
      </div>

      {/* Combos Grid */}
      <div className="space-y-4">
        {combos.length === 0 ? (
          <div className="p-8 text-center rounded-xl border border-dashed border-border-default bg-surface-1/50 space-y-3">
            <Layers className="w-8 h-8 mx-auto text-fg-subtle" />
            <h3 className="text-sm font-medium text-fg-editor">No routing combos configured</h3>
            <p className="text-xs text-fg-muted max-w-md mx-auto">
              Create a combo to automatically fail over across multiple providers when rate limits or outages occur.
            </p>
            <button
              onClick={openCreateModal}
              className="px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border-default text-xs font-medium rounded-lg text-fg-editor transition cursor-pointer"
            >
              Create first combo
            </button>
          </div>
        ) : (
          combos.map((combo) => (
            <div
              key={combo.id}
              className="p-4 sm:p-5 rounded-xl border border-border-default bg-surface-1 hover:border-border-emphasis/60 transition space-y-4 shadow-xs"
            >
              {/* Header */}
              <div className="flex items-start justify-between gap-3">
                <div className="space-y-1">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-sm font-semibold text-fg-editor">{combo.name}</span>
                    <span
                      className={`text-[10px] font-medium px-2 py-0.5 rounded-md border flex items-center gap-1 ${
                        combo.strategy === 'fallback'
                          ? 'bg-amber-500/10 text-amber-400 border-amber-500/20'
                          : combo.strategy === 'round_robin'
                          ? 'bg-sky-500/10 text-sky-400 border-sky-500/20'
                          : 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                      }`}
                    >
                      {combo.strategy === 'fallback' && <ShieldCheck className="w-3 h-3" />}
                      {combo.strategy === 'round_robin' && <Shuffle className="w-3 h-3" />}
                      {combo.strategy === 'cost_optimized' && <DollarSign className="w-3 h-3" />}
                      <span className="capitalize">{combo.strategy.replace('_', ' ')}</span>
                    </span>

                    <span className="text-[10px] font-mono text-fg-subtle bg-surface-2 px-1.5 py-0.5 rounded border border-border-muted">
                      ID: {combo.id}
                    </span>
                  </div>

                  <p className="text-xs text-fg-muted">{combo.description || 'No description provided'}</p>
                </div>

                {/* Top Actions */}
                <div className="flex items-center gap-1.5 shrink-0">
                  <button
                    onClick={() => handleSimulateRoute(combo)}
                    disabled={simulatingComboId === combo.id}
                    className="px-2.5 py-1 rounded-md text-[11px] font-medium bg-surface-2 hover:bg-surface-3 text-fg-editor border border-border-default hover:border-border-emphasis transition flex items-center gap-1 cursor-pointer disabled:opacity-50"
                    title="Simulate route failover and inspect response"
                  >
                    <Play className={`w-3 h-3 text-emerald-400 ${simulatingComboId === combo.id ? 'animate-spin' : ''}`} />
                    <span>{simulatingComboId === combo.id ? 'Simulating...' : 'Test Route'}</span>
                  </button>

                  <button
                    onClick={() => openEditModal(combo)}
                    className="p-1.5 rounded-md hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition cursor-pointer"
                    title="Edit combo pipeline"
                  >
                    <Edit2 className="w-3.5 h-3.5" />
                  </button>

                  <button
                    onClick={() => {
                      if (window.confirm(`Delete combo "${combo.name}"?`)) {
                        handleDeleteCombo(combo.id);
                      }
                    }}
                    className="p-1.5 rounded-md hover:bg-rose-500/10 text-fg-muted hover:text-rose-400 transition cursor-pointer"
                    title="Delete combo"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              {/* Simulation Feedback Alert */}
              {simResult && simResult.comboId === combo.id && (
                <div className="p-3 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-xs text-emerald-300 flex items-start gap-2 animate-in fade-in duration-200">
                  <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400 mt-0.5" />
                  <div>
                    <span className="font-semibold">{simResult.step}: </span>
                    <span>{simResult.message}</span>
                  </div>
                </div>
              )}

              {/* Pipeline Sequence Visualization */}
              <div className="pt-2 border-t border-border-muted/60">
                <div className="text-[10px] uppercase font-mono tracking-wider text-fg-subtle mb-2">
                  {combo.strategy === 'fallback'
                    ? 'Failover Pipeline (Order of execution)'
                    : combo.strategy === 'round_robin'
                    ? 'Load Balancer Targets'
                    : 'Cost Optimization Order'}
                </div>

                <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-thin">
                  {combo.targets.map((target, idx) => {
                    const info = getProviderInfo(target.modelId);
                    const isLast = idx === combo.targets.length - 1;

                    return (
                      <React.Fragment key={target.modelId + idx}>
                        <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-surface-2/80 border border-border-default shrink-0 min-w-[150px]">
                          <ProviderIcon provider={info.provider} size={15} />
                          <div className="min-w-0">
                            <div className="flex items-center gap-1.5">
                              <span className="text-[10px] font-mono font-bold text-fg-subtle">
                                #{target.priority}
                              </span>
                              <span className="text-xs font-medium text-fg-editor truncate max-w-[110px]">
                                {target.modelName}
                              </span>
                            </div>
                            <span className="text-[10px] text-fg-muted truncate block">
                              {target.providerName}
                              {target.weight ? ` · ${target.weight}% weight` : ''}
                            </span>
                          </div>
                        </div>

                        {!isLast && (
                          <div className="text-fg-subtle shrink-0">
                            {combo.strategy === 'fallback' ? (
                              <ArrowRight className="w-3.5 h-3.5 text-amber-400" />
                            ) : (
                              <Shuffle className="w-3.5 h-3.5 text-sky-400" />
                            )}
                          </div>
                        )}
                      </React.Fragment>
                    );
                  })}
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Create / Edit Combo Modal */}
      {isCreateModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
          <div className="flex max-h-[90vh] w-full max-w-xl flex-col overflow-hidden rounded-xl border border-border-default bg-surface-1 shadow-2xl">
            <header className="flex h-13 shrink-0 items-center justify-between border-b border-border-muted px-5">
              <div className="flex items-center gap-2">
                <Layers className="w-4 h-4 text-workbench-accent" />
                <h3 className="text-sm font-semibold text-fg-editor">
                  {editingCombo ? 'Edit Routing Combo' : 'Create New Routing Combo'}
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setIsCreateModalOpen(false)}
                className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </header>

            <form onSubmit={handleSave} className="flex flex-col flex-1 overflow-y-auto">
              <div className="p-5 space-y-4 text-xs">
                {/* Name */}
                <div className="space-y-1">
                  <label className="font-medium text-fg-editor">Combo Name</label>
                  <input
                    type="text"
                    required
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="e.g. Primary Coding + Failover"
                    className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none text-xs transition"
                  />
                </div>

                {/* Description */}
                <div className="space-y-1">
                  <label className="font-medium text-fg-editor">Description</label>
                  <input
                    type="text"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                    placeholder="e.g. Try Claude 3.7 first; divert to GPT-4o on rate limit"
                    className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none text-xs transition"
                  />
                </div>

                {/* Strategy */}
                <div className="space-y-1">
                  <label className="font-medium text-fg-editor">Routing Strategy</label>
                  <div className="grid grid-cols-3 gap-2">
                    {[
                      { id: 'fallback', label: 'Fallback Chain', desc: 'Auto failover on 429/5xx' },
                      { id: 'round_robin', label: 'Round Robin', desc: 'Even load balancing' },
                      { id: 'cost_optimized', label: 'Cost Optimized', desc: 'Cheapest model first' },
                    ].map((s) => (
                      <button
                        key={s.id}
                        type="button"
                        onClick={() => setStrategy(s.id as ComboStrategy)}
                        className={`p-2.5 rounded-lg border text-left transition cursor-pointer ${
                          strategy === s.id
                            ? 'bg-surface-2 border-workbench-accent text-fg-editor font-medium'
                            : 'bg-surface-0 border-border-default text-fg-muted hover:border-border-emphasis'
                        }`}
                      >
                        <div className="text-xs font-semibold">{s.label}</div>
                        <div className="text-[10px] text-fg-subtle mt-0.5">{s.desc}</div>
                      </button>
                    ))}
                  </div>
                </div>

                {/* Targets Builder */}
                <div className="space-y-2 pt-2">
                  <div className="flex items-center justify-between">
                    <label className="font-medium text-fg-editor">Pipeline Target Models</label>
                    <span className="text-[11px] text-fg-subtle font-mono">
                      {targets.length} targets configured
                    </span>
                  </div>

                  <div className="space-y-2">
                    {targets.map((target, idx) => {
                      const info = getProviderInfo(target.modelId);
                      return (
                        <div
                          key={target.modelId + idx}
                          className="flex items-center justify-between p-2.5 rounded-lg bg-surface-0 border border-border-default"
                        >
                          <div className="flex items-center gap-2.5">
                            <span className="w-5 h-5 rounded-full bg-surface-2 flex items-center justify-center font-mono text-[10px] font-bold text-fg-subtle">
                              {idx + 1}
                            </span>
                            <ProviderIcon provider={info.provider} size={14} />
                            <div>
                              <div className="font-medium text-fg-editor">{target.modelName}</div>
                              <div className="text-[10px] text-fg-muted font-mono">{target.modelId}</div>
                            </div>
                          </div>

                          <div className="flex items-center gap-1">
                            <button
                              type="button"
                              onClick={() => handleMoveTarget(idx, 'up')}
                              disabled={idx === 0}
                              className="p-1 hover:bg-surface-2 rounded text-fg-muted hover:text-fg-editor disabled:opacity-30 cursor-pointer"
                              title="Move priority up"
                            >
                              <MoveUp className="w-3.5 h-3.5" />
                            </button>
                            <button
                              type="button"
                              onClick={() => handleMoveTarget(idx, 'down')}
                              disabled={idx === targets.length - 1}
                              className="p-1 hover:bg-surface-2 rounded text-fg-muted hover:text-fg-editor disabled:opacity-30 cursor-pointer"
                              title="Move priority down"
                            >
                              <MoveDown className="w-3.5 h-3.5" />
                            </button>
                            <button
                              type="button"
                              onClick={() => handleRemoveTarget(idx)}
                              className="p-1 hover:bg-rose-500/10 rounded text-fg-muted hover:text-rose-400 cursor-pointer ml-1"
                              title="Remove target"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        </div>
                      );
                    })}
                  </div>

                  {/* Add Target Row */}
                  <div className="flex gap-2 pt-1">
                    <select
                      value={selectedAddModel}
                      onChange={(e) => setSelectedAddModel(e.target.value)}
                      className="flex-1 bg-surface-0 border border-border-default rounded-lg px-3 py-1.5 text-xs text-fg-editor focus:outline-none"
                    >
                      {CANONICAL_MODELS.map((m) => (
                        <option key={m.id} value={m.id}>
                          {m.name} ({m.providerName})
                        </option>
                      ))}
                    </select>
                    <button
                      type="button"
                      onClick={handleAddTarget}
                      className="px-3 py-1.5 rounded-lg bg-surface-2 hover:bg-surface-3 border border-border-default text-xs font-medium text-fg-editor flex items-center gap-1 cursor-pointer"
                    >
                      <Plus className="w-3.5 h-3.5" />
                      <span>Add Target</span>
                    </button>
                  </div>
                </div>
              </div>

              {/* Footer */}
              <div className="h-14 border-t border-border-muted px-5 flex items-center justify-end gap-2 bg-surface-1 shrink-0">
                <button
                  type="button"
                  onClick={() => setIsCreateModalOpen(false)}
                  className="px-3.5 py-1.5 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="workbench-primary-action px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer"
                >
                  {editingCombo ? 'Save Changes' : 'Create Combo'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
