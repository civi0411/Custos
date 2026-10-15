import React, { useState, useEffect } from 'react';
import { CreateTaskParams, Criterion, ExecutionWorkspace } from '@/types/domain';
import { X, Plus, Trash2, ListChecks, FolderGit2, Sparkles } from 'lucide-react';

interface CreateTaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (params: CreateTaskParams) => Promise<void>;
  workspaces: ExecutionWorkspace[];
  currentWorkspaceId?: string;
  initialTitle?: string;
  initialDescription?: string;
  initialWorkspaceId?: string;
  initialSessionId?: string;
  initialPackId?: 'code' | 'research' | 'copilot';
}

export const CreateTaskModal: React.FC<CreateTaskModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  workspaces,
  currentWorkspaceId,
  initialTitle,
  initialDescription,
  initialWorkspaceId,
  initialSessionId,
  initialPackId,
}) => {
  const [title, setTitle] = useState(initialTitle || '');
  const [packId, setPackId] = useState<'code' | 'research' | 'copilot'>(initialPackId || 'code');
  const [description, setDescription] = useState(initialDescription || '');
  const [workspaceId, setWorkspaceId] = useState(initialWorkspaceId || currentWorkspaceId || '');
  const [criteria, setCriteria] = useState<Criterion[]>([
    { name: '', description: '', mandatory: true },
  ]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      if (initialTitle !== undefined) setTitle(initialTitle);
      if (initialDescription !== undefined) setDescription(initialDescription);
      if (initialPackId !== undefined) setPackId(initialPackId);
      if (initialWorkspaceId !== undefined) setWorkspaceId(initialWorkspaceId);
      else if (currentWorkspaceId !== undefined) setWorkspaceId(currentWorkspaceId);
    }
  }, [isOpen, initialTitle, initialDescription, initialPackId, initialWorkspaceId, currentWorkspaceId]);

  if (!isOpen) return null;

  const handleAddCriterion = () => {
    setCriteria([...criteria, { name: '', description: '', mandatory: true }]);
  };

  const handleRemoveCriterion = (index: number) => {
    setCriteria(criteria.filter((_, i) => i !== index));
  };

  const handleCriterionChange = (index: number, field: keyof Criterion, value: any) => {
    const next = [...criteria];
    next[index] = { ...next[index], [field]: value };
    setCriteria(next);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) {
      setError('Task title is required');
      return;
    }

    setIsSubmitting(true);
    setError(null);

    const validCriteria = criteria
      .filter((c) => c.name.trim().length > 0)
      .map((c) => ({
        name: c.name.trim(),
        description: c.description?.trim() || undefined,
        mandatory: c.mandatory,
      }));

    try {
      await onSubmit({
        title: title.trim(),
        contract: {
          pack_id: packId,
          description: description.trim() || undefined,
          criteria: validCriteria.length > 0 ? validCriteria : undefined,
        },
        metadata: {
          ...(workspaceId ? { workspace_id: workspaceId } : {}),
          ...(initialSessionId ? { session_id: initialSessionId, conversation_session_id: initialSessionId } : {}),
        },
      });

      // Reset form
      setTitle('');
      setDescription('');
      setCriteria([{ name: '', description: '', mandatory: true }]);
      onClose();
    } catch (err: any) {
      setError(err?.message || 'Failed to create task');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
      <div className="relative w-full max-w-xl rounded-2xl border border-border-default bg-surface-1 shadow-2xl flex flex-col max-h-[90vh]">
        {/* Modal Header */}
        <div className="flex items-center justify-between border-b border-border-default px-6 py-4">
          <div className="flex items-center gap-2">
            <Sparkles className="size-4 text-workbench-accent" />
            <h2 className="text-sm font-semibold text-fg-editor">Create New Task</h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-lg p-1 text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition-colors"
          >
            <X className="size-4" />
          </button>
        </div>

        {/* Modal Body Form */}
        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-6 space-y-5 scrollbar-sleek">
          {error && (
            <div className="rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-400">
              {error}
            </div>
          )}

          {/* Title Input */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-fg-editor">
              Task Title <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              required
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g. Implement local AI inference pipeline"
              className="w-full rounded-lg border border-border-default bg-surface-2 px-3 py-2 text-xs text-fg-editor placeholder:text-fg-muted/60 focus:border-border-emphasis focus:outline-none"
            />
          </div>

          {/* Pack / Domain Selector */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-fg-editor">Domain Pack</label>
            <div className="grid grid-cols-3 gap-2">
              {[
                { id: 'code', label: 'Engineering', desc: 'Coding & Worktrees' },
                { id: 'research', label: 'Research', desc: 'Science & Papers' },
                { id: 'copilot', label: 'Assistant', desc: 'Planning & Tools' },
              ].map((p) => (
                <button
                  key={p.id}
                  type="button"
                  onClick={() => setPackId(p.id as any)}
                  className={`rounded-xl border p-2.5 text-left transition-all ${
                    packId === p.id
                      ? 'border-workbench-accent bg-workbench-accent/10 shadow-xs'
                      : 'border-border-default bg-surface-2 hover:border-border-emphasis'
                  }`}
                >
                  <div
                    className={`text-xs font-semibold ${
                      packId === p.id ? 'text-workbench-accent' : 'text-fg-editor'
                    }`}
                  >
                    {p.label}
                  </div>
                  <div className="text-[10px] text-fg-muted mt-0.5">{p.desc}</div>
                </button>
              ))}
            </div>
          </div>

          {/* Specification / Description */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-fg-editor">Specification & Objective</label>
            <textarea
              rows={4}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Describe the problem, requirements, acceptance criteria, or constraints..."
              className="w-full rounded-lg border border-border-default bg-surface-2 p-3 text-xs text-fg-editor placeholder:text-fg-muted/60 focus:border-border-emphasis focus:outline-none resize-none font-mono"
            />
          </div>

          {/* Bound Execution Workspace */}
          {workspaces.length > 0 && (
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-fg-editor flex items-center gap-1.5">
                <FolderGit2 className="size-3.5 text-workbench-accent" />
                <span>Execution Workspace (Optional)</span>
              </label>
              <select
                value={workspaceId}
                onChange={(e) => setWorkspaceId(e.target.value)}
                className="w-full rounded-lg border border-border-default bg-surface-2 px-3 py-2 text-xs text-fg-editor focus:border-border-emphasis focus:outline-none"
              >
                <option value="">(None - pick later)</option>
                {workspaces.map((w) => {
                  const br = w.active_branch || w.activeBranch || w.branch;
                  return (
                    <option key={w.id} value={w.id}>
                      {w.name} {br ? `[${br}]` : ''}
                    </option>
                  );
                })}
              </select>
            </div>
          )}

          {/* Acceptance Criteria Builder (Orca Criteria Pattern) */}
          <div className="space-y-2.5 pt-2 border-t border-border-muted">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-fg-editor flex items-center gap-1.5">
                <ListChecks className="size-3.5 text-workbench-accent" />
                <span>Acceptance Criteria</span>
              </label>
              <button
                type="button"
                onClick={handleAddCriterion}
                className="inline-flex items-center gap-1 text-[11px] font-medium text-workbench-accent hover:underline"
              >
                <Plus className="size-3" />
                <span>Add Criterion</span>
              </button>
            </div>

            <div className="space-y-2">
              {criteria.map((c, index) => (
                <div
                  key={index}
                  className="flex items-start gap-2 rounded-lg border border-border-muted/60 bg-surface-2 p-2.5"
                >
                  <div className="flex-1 space-y-1.5">
                    <input
                      type="text"
                      value={c.name}
                      onChange={(e) => handleCriterionChange(index, 'name', e.target.value)}
                      placeholder={`Criterion #${index + 1} (e.g. Unit tests pass with 0 errors)`}
                      className="w-full rounded border border-border-muted bg-surface-1 px-2.5 py-1 text-xs text-fg-editor placeholder:text-fg-muted/60 focus:outline-none"
                    />
                    <input
                      type="text"
                      value={c.description || ''}
                      onChange={(e) =>
                        handleCriterionChange(index, 'description', e.target.value)
                      }
                      placeholder="Optional details or verification method"
                      className="w-full rounded border border-border-muted bg-surface-1 px-2.5 py-0.5 text-[11px] text-fg-muted placeholder:text-fg-muted/40 focus:outline-none"
                    />
                  </div>

                  <div className="flex flex-col items-end gap-2 pt-0.5">
                    <label className="flex items-center gap-1 text-[10px] text-fg-muted select-none cursor-pointer">
                      <input
                        type="checkbox"
                        checked={c.mandatory}
                        onChange={(e) =>
                          handleCriterionChange(index, 'mandatory', e.target.checked)
                        }
                        className="rounded border-border-default text-workbench-accent"
                      />
                      <span>Required</span>
                    </label>

                    {criteria.length > 1 && (
                      <button
                        type="button"
                        onClick={() => handleRemoveCriterion(index)}
                        className="text-fg-muted hover:text-red-400 p-0.5"
                        title="Remove criterion"
                      >
                        <Trash2 className="size-3" />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Modal Footer */}
          <div className="flex items-center justify-end gap-2.5 pt-4 border-t border-border-default">
            <button
              type="button"
              onClick={onClose}
              disabled={isSubmitting}
              className="rounded-lg border border-border-default bg-surface-2 px-4 py-2 text-xs font-medium text-fg-muted hover:text-fg-editor transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="rounded-lg bg-workbench-accent px-5 py-2 text-xs font-semibold text-white shadow-xs hover:opacity-90 active:scale-98 transition-all disabled:opacity-50"
            >
              {isSubmitting ? 'Creating...' : 'Create Task'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
