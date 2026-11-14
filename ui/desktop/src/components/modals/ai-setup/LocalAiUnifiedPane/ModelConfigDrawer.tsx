import React from 'react';
import { Save } from 'lucide-react';
import type { LocalModelLoadProfile, LocalModelPreflightResponse } from '@/types/domain';
import { formatBytes } from './types';

export interface ModelConfigDrawerProps {
  editingProfile: LocalModelLoadProfile;
  setEditingProfile: React.Dispatch<React.SetStateAction<LocalModelLoadProfile | null>>;
  profileEstimate: LocalModelPreflightResponse | null;
  estimatingProfile: boolean;
  onEstimateProfile: () => void;
  onSaveProfileOnly: () => void;
  onSaveAndLoadProfile: () => void;
  savingProfile: boolean;
  isOperating: boolean;
  isLoaded: boolean;
}

export const ModelConfigDrawer: React.FC<ModelConfigDrawerProps> = ({
  editingProfile,
  setEditingProfile,
  profileEstimate,
  estimatingProfile,
  onEstimateProfile,
  onSaveProfileOnly,
  onSaveAndLoadProfile,
  savingProfile,
  isOperating,
  isLoaded,
}) => {
  return (
    <div className="border-t border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 space-y-3.5 rounded-b-xl">
      <div className="flex items-center justify-between text-[11px] font-mono text-[var(--color-fg-muted)]">
        <span className="font-medium text-[var(--color-editor-fg)]">VRAM Offload & Execution Configuration</span>
        <span className="text-[10px] text-[var(--color-fg-subtle)]">Config applies directly on load</span>
      </div>

      {/* Inputs Grid */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {/* Context Size */}
        <div>
          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">Context Window</label>
          <select
            value={editingProfile.context_size}
            onChange={(e) => setEditingProfile({ ...editingProfile, context_size: Number(e.target.value) })}
            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] cursor-pointer"
          >
            <option value={4096}>4,096 tokens (Fast)</option>
            <option value={8192}>8,192 tokens</option>
            <option value={16384}>16,384 tokens (Balanced)</option>
            <option value={32768}>32,768 tokens</option>
            <option value={65536}>65,536 tokens</option>
            <option value={131072}>131,072 tokens (Long context)</option>
          </select>
        </div>

        {/* GPU Mode */}
        <div>
          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">GPU Offload Mode</label>
          <select
            value={editingProfile.gpu_mode}
            onChange={(e) => setEditingProfile({ ...editingProfile, gpu_mode: e.target.value as any })}
            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] cursor-pointer"
          >
            <option value="auto">Auto (Full Metal GPU)</option>
            <option value="max">Max Offload</option>
            <option value="off">CPU Only</option>
            <option value="manual">Manual Layers</option>
          </select>
        </div>

        {/* Eval Batch */}
        <div>
          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">Evaluation Batch</label>
          <input
            type="number"
            value={editingProfile.eval_batch_size || 512}
            onChange={(e) => setEditingProfile({ ...editingProfile, eval_batch_size: Number(e.target.value) })}
            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
          />
        </div>

        {/* CPU Threads */}
        <div>
          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">CPU Threads</label>
          <input
            type="number"
            placeholder="Auto"
            value={editingProfile.cpu_threads || ''}
            onChange={(e) => setEditingProfile({ ...editingProfile, cpu_threads: e.target.value ? Number(e.target.value) : undefined })}
            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
          />
        </div>
      </div>

      {/* Checkboxes */}
      <div className="flex flex-wrap items-center gap-5 text-[11px] text-[var(--color-fg-muted)] pt-1 font-mono">
        <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
          <input
            type="checkbox"
            checked={editingProfile.flash_attention ?? true}
            onChange={(e) => setEditingProfile({ ...editingProfile, flash_attention: e.target.checked })}
            className="accent-[var(--color-editor-fg)] rounded"
          />
          <span>Flash Attention</span>
        </label>
        <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
          <input
            type="checkbox"
            checked={editingProfile.mmap ?? true}
            onChange={(e) => setEditingProfile({ ...editingProfile, mmap: e.target.checked })}
            className="accent-[var(--color-editor-fg)] rounded"
          />
          <span>Memory Map (mmap)</span>
        </label>
        <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
          <input
            type="checkbox"
            checked={editingProfile.mlock ?? false}
            onChange={(e) => setEditingProfile({ ...editingProfile, mlock: e.target.checked })}
            className="accent-[var(--color-editor-fg)] rounded"
          />
          <span>Lock VRAM (mlock)</span>
        </label>
      </div>

      {/* Estimate Preview */}
      {profileEstimate && (
        <div className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-2.5 text-[11px] font-mono text-[var(--color-fg-muted)] flex flex-wrap items-center gap-4">
          <span>RAM Est: <strong className="text-[var(--color-editor-fg)]">{formatBytes(profileEstimate.estimated_memory_bytes)}</strong></span>
          <span>•</span>
          <span>Free: <strong className="text-[var(--color-editor-fg)]">{formatBytes(profileEstimate.free_ram_bytes)}</strong></span>
          <span>•</span>
          <span>Context: <strong className="text-[var(--color-editor-fg)]">{profileEstimate.context_limit.toLocaleString()}</strong></span>
        </div>
      )}

      {/* Action buttons inside drawer */}
      <div className="flex justify-end items-center gap-2 pt-2 border-t border-[var(--color-border-muted)]">
        <button
          type="button"
          onClick={onEstimateProfile}
          disabled={estimatingProfile}
          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 cursor-pointer"
        >
          {estimatingProfile ? 'Estimating…' : 'Estimate Memory'}
        </button>
        <button
          type="button"
          onClick={onSaveProfileOnly}
          disabled={savingProfile}
          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3.5 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 cursor-pointer"
        >
          Save Settings
        </button>
        <button
          type="button"
          onClick={onSaveAndLoadProfile}
          disabled={savingProfile || isOperating}
          className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-4 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 cursor-pointer shadow-xs"
        >
          <Save className="size-3" />
          {savingProfile
            ? 'Loading…'
            : isLoaded
            ? 'Save & Reload VRAM'
            : 'Save & Load to VRAM'}
        </button>
      </div>
    </div>
  );
};
