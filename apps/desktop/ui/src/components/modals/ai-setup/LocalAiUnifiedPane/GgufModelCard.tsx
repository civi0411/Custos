import React from 'react';
import {
  Play,
  PowerOff,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Sliders,
  ChevronDown,
  ChevronUp,
  Check,
} from 'lucide-react';
import type {
  LocalModelDescriptor,
  LoadedModelInstance,
  ModelCompatibility,
  LocalModelLoadProfile,
  LocalModelPreflightResponse,
} from '@/types/domain';
import { formatBytes } from './types';
import { ModelConfigDrawer } from './ModelConfigDrawer';

export interface GgufModelCardProps {
  model: LocalModelDescriptor;
  instance?: LoadedModelInstance;
  isAppleSilicon: boolean;
  isOperating: boolean;
  isConfigOpen: boolean;
  loadError?: string;
  isCurrentActive: boolean;
  verifyingPath: string | null;
  onSetActive: (path: string, name: string) => void;
  onToggleConfig: (path: string) => void;
  onDirectLoad: (model: LocalModelDescriptor) => void;
  onUnload: (path: string, name?: string) => void;
  onVerify: (path: string) => void;
  editingProfile: LocalModelLoadProfile | null;
  setEditingProfile: React.Dispatch<React.SetStateAction<LocalModelLoadProfile | null>>;
  profileEstimate: LocalModelPreflightResponse | null;
  estimatingProfile: boolean;
  onEstimateProfile: () => void;
  onSaveProfileOnly: () => void;
  onSaveAndLoadProfile: () => void;
  savingProfile: boolean;
}

export const GgufModelCard: React.FC<GgufModelCardProps> = ({
  model: m,
  instance,
  isAppleSilicon,
  isOperating,
  isConfigOpen,
  loadError,
  isCurrentActive,
  verifyingPath,
  onSetActive,
  onToggleConfig,
  onDirectLoad,
  onUnload,
  onVerify,
  editingProfile,
  setEditingProfile,
  profileEstimate,
  estimatingProfile,
  onEstimateProfile,
  onSaveProfileOnly,
  onSaveAndLoadProfile,
  savingProfile,
}) => {
  const isLoaded = Boolean(instance);

  const renderCompatBadge = (compat: ModelCompatibility) => {
    if (!isAppleSilicon) {
      return (
        <span className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-subtle)]">
          CPU
        </span>
      );
    }
    switch (compat) {
      case 'green':
        return (
          <span className="inline-flex items-center gap-1 rounded border border-emerald-500/25 bg-emerald-500/10 px-1.5 py-0.5 text-[9px] font-mono text-emerald-400">
            <CheckCircle2 size={10} /> Fits VRAM
          </span>
        );
      case 'yellow':
        return (
          <span className="inline-flex items-center gap-1 rounded border border-amber-500/25 bg-amber-500/10 px-1.5 py-0.5 text-[9px] font-mono text-amber-400">
            <AlertTriangle size={10} /> Partial Offload
          </span>
        );
      case 'red':
        return (
          <span className="inline-flex items-center gap-1 rounded border border-rose-500/25 bg-rose-500/10 px-1.5 py-0.5 text-[9px] font-mono text-rose-400">
            <XCircle size={10} /> Exceeds VRAM
          </span>
        );
    }
  };

  return (
    <div
      className={`rounded-xl border transition-all ${
        isCurrentActive
          ? 'border-sky-500/40 bg-[var(--color-surface-0)] shadow-xs ring-1 ring-sky-500/20'
          : isLoaded
          ? 'border-emerald-500/35 bg-[var(--color-surface-0)] shadow-xs'
          : isConfigOpen
          ? 'border-[var(--color-border-emphasis)] bg-[var(--color-surface-0)] shadow-xs'
          : 'border-[var(--color-border-default)] bg-[var(--color-surface-0)] hover:border-[var(--color-border-emphasis)]'
      }`}
    >
      {/* Model Row Header */}
      <div className="flex items-center justify-between gap-3 p-3.5 text-xs">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            {/* Status Dot */}
            <span
              className={`size-2 rounded-full shrink-0 ${
                isLoaded ? 'bg-emerald-400 ring-2 ring-emerald-400/20' : 'bg-[var(--color-fg-subtle)]/50'
              }`}
            />
            <span className="font-semibold text-[var(--color-editor-fg)] font-mono truncate">
              {m.model_name || m.file_name}
            </span>
            <span className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-muted)]">
              GGUF
            </span>
            {isLoaded ? (
              <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[9px] text-emerald-400 font-medium">
                <span className="size-1 rounded-full bg-emerald-400" />
                in VRAM ({instance?.context_limit.toLocaleString()} ctx)
              </span>
            ) : loadError ? (
              <span className="rounded-md border border-rose-500/30 bg-rose-500/10 px-2 py-0.5 font-mono text-[9px] text-rose-400">
                Load Failed
              </span>
            ) : (
              renderCompatBadge(m.compatibility)
            )}
            {isCurrentActive && (
              <span className="inline-flex items-center gap-1 rounded-md border border-sky-500/30 bg-sky-500/10 px-2 py-0.5 font-mono text-[9px] text-sky-300 font-medium">
                <Check className="size-2.5" />
                Active Model
              </span>
            )}
          </div>
          <div className="font-mono text-[10px] text-[var(--color-fg-subtle)] mt-1 flex flex-wrap items-center gap-2">
            <span className="text-[var(--color-fg-muted)] font-medium">{formatBytes(m.file_size_bytes)}</span>
            <span>•</span>
            <span>{m.quantization || 'Q4_K_M'}</span>
            <span>•</span>
            <span>{m.architecture || 'gguf'}</span>
            <span>•</span>
            <span className="truncate max-w-[280px]" title={m.file_path}>
              {m.file_path}
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {/* Set Active Model Button */}
          {isCurrentActive ? (
            <span className="flex items-center gap-1 rounded-lg border border-sky-500/30 bg-sky-500/10 px-2.5 py-1 font-mono text-[11px] text-sky-300 font-medium">
              <Check className="size-3" /> Active
            </span>
          ) : (
            <button
              type="button"
              onClick={() => onSetActive(`local/${m.file_path}`, m.model_name || m.file_name)}
              className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:border-[var(--color-border-emphasis)] hover:bg-[var(--color-surface-3)] transition cursor-pointer"
            >
              Set Active
            </button>
          )}

          {/* Settings Toggle */}
          <button
            type="button"
            onClick={() => onToggleConfig(m.file_path)}
            className={`flex items-center gap-1 rounded-lg border px-2.5 py-1 font-mono text-[11px] transition cursor-pointer ${
              isConfigOpen
                ? 'border-[var(--color-border-emphasis)] bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] font-medium'
                : 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)]'
            }`}
          >
            <Sliders className="size-3 text-[var(--color-fg-subtle)]" />
            <span>Settings</span>
            {isConfigOpen ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
          </button>

          {isLoaded ? (
            <>
              <button
                type="button"
                onClick={() => onVerify(m.file_path)}
                disabled={verifyingPath === m.file_path}
                className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 transition cursor-pointer"
              >
                {verifyingPath === m.file_path ? 'Verifying…' : 'Verify'}
              </button>
              <button
                type="button"
                onClick={() => onUnload(m.file_path, m.model_name)}
                disabled={isOperating}
                className="flex items-center gap-1 rounded-lg border border-rose-500/25 bg-rose-500/10 px-2.5 py-1 font-mono text-[11px] text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/40 transition cursor-pointer font-medium"
              >
                <PowerOff className="size-3" /> Unload
              </button>
            </>
          ) : (
            <button
              type="button"
              onClick={() => onDirectLoad(m)}
              disabled={isOperating}
              className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1 font-mono text-[11px] font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 transition cursor-pointer shadow-xs"
            >
              <Play className="size-3 fill-current" />
              {isOperating ? 'Loading…' : 'Load to VRAM'}
            </button>
          )}
        </div>
      </div>

      {/* Per-Model Config Drawer */}
      {isConfigOpen && editingProfile && (
        <ModelConfigDrawer
          editingProfile={editingProfile}
          setEditingProfile={setEditingProfile}
          profileEstimate={profileEstimate}
          estimatingProfile={estimatingProfile}
          onEstimateProfile={onEstimateProfile}
          onSaveProfileOnly={onSaveProfileOnly}
          onSaveAndLoadProfile={onSaveAndLoadProfile}
          savingProfile={savingProfile}
          isOperating={isOperating}
          isLoaded={isLoaded}
        />
      )}
    </div>
  );
};
