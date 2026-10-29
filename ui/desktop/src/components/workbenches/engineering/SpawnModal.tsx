import React, { useState } from 'react';
import { Plus, X, Loader2 } from 'lucide-react';
import type { Task } from '@/types/domain';

export interface SpawnModalProps {
  tasks: Task[];
  onSpawn: (taskId: string, count: number) => Promise<void>;
  onClose: () => void;
}

export const SpawnModal: React.FC<SpawnModalProps> = ({ tasks, onSpawn, onClose }) => {
  const [spawnTaskId, setSpawnTaskId] = useState('');
  const [spawnCount, setSpawnCount] = useState(2);
  const [spawning, setSpawning] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!spawnTaskId || spawning) return;
    setSpawning(true);
    try {
      await onSpawn(spawnTaskId, spawnCount);
    } finally {
      setSpawning(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
      <div className="w-full max-w-md rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-5 shadow-2xl">
        <div className="flex items-center justify-between border-b border-[var(--color-border-subtle)] pb-3">
          <div className="flex items-center gap-2">
            <Plus className="h-4 w-4 text-white/80" />
            <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
              Spawn Parallel Candidates
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded p-1 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4 text-xs">
          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Existing Task
            </label>
            <select
              required
              value={spawnTaskId}
              onChange={(e) => setSpawnTaskId(e.target.value)}
              className="w-full rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] focus:outline-none focus:border-white/30"
            >
              <option value="">Select a Task…</option>
              {tasks.map((task) => (
                <option key={task.id} value={task.id}>
                  {task.title} · {task.id}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
              Number of Candidates (Race)
            </label>
            <div className="flex gap-2">
              {[2, 3, 4].map((n) => (
                <button
                  key={n}
                  type="button"
                  onClick={() => setSpawnCount(n)}
                  className={`flex-1 rounded border py-1.5 font-medium transition ${
                    spawnCount === n
                      ? 'border-white/30 bg-white/10 text-white'
                      : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]'
                  }`}
                >
                  {n} Candidates
                </button>
              ))}
            </div>
          </div>

          <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-border-subtle)]">
            <button
              type="button"
              onClick={onClose}
              className="rounded px-3 py-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={spawning || !spawnTaskId || !tasks.some((task) => task.id === spawnTaskId)}
              className="flex items-center gap-1.5 rounded-md bg-white hover:bg-white/90 px-4 py-1.5 font-semibold text-black disabled:opacity-40 transition shadow-sm"
            >
              {spawning && <Loader2 className="h-3 w-3 animate-spin text-black" />}
              <span>Spawn Worktrees</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
