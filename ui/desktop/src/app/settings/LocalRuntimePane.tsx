import React, { useState, useEffect, useCallback } from 'react';
import {
  Cpu,
  Folder,
  FolderPlus,
  RefreshCw,
  Trash2,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Play,
  PowerOff,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type {
  LocalModelFolder,
  LocalModelDescriptor,
  LocalHardwareProfile,
  ModelCompatibility,
  LoadedModelInstance,
} from '@/types/domain';

interface LocalRuntimePaneProps {
  onNavigateToProviders?: () => void;
  onShowToast?: (msg: string) => void;
}

export const LocalRuntimePane: React.FC<LocalRuntimePaneProps> = ({
  onNavigateToProviders,
  onShowToast,
}) => {
  const [folders, setFolders] = useState<LocalModelFolder[]>([]);
  const [models, setModels] = useState<LocalModelDescriptor[]>([]);
  const [hardware, setHardware] = useState<LocalHardwareProfile | null>(null);
  const [instances, setInstances] = useState<LoadedModelInstance[]>([]);
  const [loading, setLoading] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [loadingModelPath, setLoadingModelPath] = useState<string | null>(null);
  const [modelLoadErrors, setModelLoadErrors] = useState<Record<string, string>>({});
  const [showAddFolder, setShowAddFolder] = useState(false);
  const [newFolderPath, setNewFolderPath] = useState('');
  const [newFolderLabel, setNewFolderLabel] = useState('');
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const [foldersRes, modelsRes, hwRes, instancesRes] = await Promise.all([
        daemonClient.listLocalFolders(),
        daemonClient.listLocalModels(),
        daemonClient.getLocalHardware(),
        daemonClient.listLoadedInstances(),
      ]);
      setFolders(foldersRes || []);
      setModels(modelsRes.models || []);
      setHardware(hwRes);
      setInstances(instancesRes.instances || []);

    } catch (err: any) {
      console.error('Failed to load local models data:', err);
      setErrorMsg(err.message || 'Failed to load local models');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const handleScan = async (folderId?: string) => {
    setScanning(true);
    setErrorMsg(null);
    try {
      const res = await daemonClient.scanLocalModels(folderId);
      const inventory = await daemonClient.listLocalModels();
      setModels(inventory.models || []);
      if (res.hardware) setHardware(res.hardware);
      onShowToast?.(`Scan complete: found ${res.models?.length || 0} local model(s).`);
    } catch (err: any) {
      setErrorMsg(err.message || 'Scan failed');
    } finally {
      setScanning(false);
    }
  };

  const handleLoadModel = async (model: LocalModelDescriptor) => {
    setLoadingModelPath(model.file_path);
    setErrorMsg(null);
    setModelLoadErrors((previous) => {
      const next = { ...previous };
      delete next[model.file_path];
      return next;
    });
    try {
      const preflight = await daemonClient.preflightLocalModel(model.file_path, 8192);
      if (!preflight.can_load) {
        throw new Error(
          preflight.issues.join('. ') || 'Model exceeds system memory limits'
        );
      }
      await daemonClient.loadLocalModel(model.file_path, preflight.context_limit, preflight.recommended_gpu_layers);
      await loadData();
      onShowToast?.(`Model "${model.model_name || model.file_name}" loaded by Custos.`);
    } catch (err: any) {
      console.error('Model load error:', err);
      setModelLoadErrors((previous) => ({
        ...previous,
        [model.file_path]: err.message || 'Failed to load model',
      }));
    } finally {
      setLoadingModelPath(null);
    }
  };

  const handleUnloadModel = async (model: LocalModelDescriptor) => {
    setLoadingModelPath(model.file_path);
    setErrorMsg(null);
    try {
      await daemonClient.unloadLocalModel(model.file_path);
      await loadData();
      onShowToast?.(`Model "${model.model_name || model.file_name}" unloaded from memory.`);
    } catch (err: any) {
      console.error('Model unload error:', err);
      setErrorMsg(err.message || 'Failed to unload model');
    } finally {
      setLoadingModelPath(null);
    }
  };

  const handleAddFolder = async () => {
    if (!newFolderPath.trim()) return;
    setErrorMsg(null);
    try {
      await daemonClient.addLocalFolder(
        newFolderPath.trim(),
        newFolderLabel.trim() || undefined
      );
      setNewFolderPath('');
      setNewFolderLabel('');
      setShowAddFolder(false);
      await loadData();
      onShowToast?.('Model directory added.');
    } catch (err: any) {
      setErrorMsg(err.message || 'Failed to add folder');
    }
  };

  const handlePickFolder = async () => {
    setErrorMsg(null);
    try {
      const selected = await daemonClient.pickFolder('model');
      if (selected) setNewFolderPath(selected);
    } catch (error) {
      setErrorMsg(error instanceof Error ? error.message : String(error));
    }
  };

  const handleRemoveFolder = async (id: string) => {
    setErrorMsg(null);
    try {
      await daemonClient.removeLocalFolder(id);
      await loadData();
      onShowToast?.('Model directory removed.');
    } catch (err: any) {
      setErrorMsg(err.message || 'Failed to remove folder');
    }
  };

  const formatBytes = (bytes: number) => {
    if (!bytes || bytes <= 0) return '0 B';
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(0)} MB`;
  };

  const renderCompatBadge = (compat: ModelCompatibility) => {
    if (!hardware?.is_apple_silicon) {
      return (
        <span className="text-[10px] text-[var(--color-fg-subtle)]">
          Hardware estimate unavailable
        </span>
      );
    }
    switch (compat) {
      case 'green':
        return (
          <span className="inline-flex items-center gap-1 rounded bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-400 border border-emerald-500/20">
            <CheckCircle2 size={11} />
            Estimated to fit
          </span>
        );
      case 'yellow':
        return (
          <span className="inline-flex items-center gap-1 rounded bg-amber-500/10 px-2 py-0.5 text-[10px] font-medium text-amber-400 border border-amber-500/20">
            <AlertTriangle size={11} />
            May need offload
          </span>
        );
      case 'red':
        return (
          <span className="inline-flex items-center gap-1 rounded bg-rose-500/10 px-2 py-0.5 text-[10px] font-medium text-rose-400 border border-rose-500/20">
            <XCircle size={11} />
            May exceed memory
          </span>
        );
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">
          Local AI & Native Runtimes
        </h2>
        <p className="mt-1 text-xs text-[var(--color-fg-muted)]">
          Manage local GGUF models, hardware acceleration, and memory residency on this machine.
        </p>
      </div>

      {errorMsg && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-300">
          {errorMsg}
        </div>
      )}

      {/* ── Section 1: Hardware Telemetry ── */}
      {hardware && (
        <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Cpu className="h-4 w-4 text-teal-400" />
              <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
                Local Hardware Profile
              </span>
            </div>
            <span className="rounded bg-teal-500/10 px-2 py-0.5 font-mono text-[10px] font-medium text-teal-400 border border-teal-500/20">
              {hardware.is_apple_silicon ? 'Apple Silicon · Metal-capable' : 'Standard Host'}
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="rounded-lg bg-[var(--color-surface-2)] p-2.5 border border-[var(--color-border-muted)]">
              <div className="text-[10px] text-[var(--color-fg-subtle)] uppercase tracking-wider font-semibold">
                Processor
              </div>
              <div className="mt-1 font-mono text-xs font-semibold text-[var(--color-editor-fg)] truncate">
                {hardware.chip_name || 'Generic CPU'}
              </div>
            </div>

            <div className="rounded-lg bg-[var(--color-surface-2)] p-2.5 border border-[var(--color-border-muted)]">
              <div className="text-[10px] text-[var(--color-fg-subtle)] uppercase tracking-wider font-semibold">
                Unified Memory
              </div>
              <div className="mt-1 font-mono text-xs font-semibold text-cyan-400">
                {formatBytes(hardware.total_ram_bytes)}
              </div>
            </div>

            <div className="rounded-lg bg-[var(--color-surface-2)] p-2.5 border border-[var(--color-border-muted)]">
              <div className="text-[10px] text-[var(--color-fg-subtle)] uppercase tracking-wider font-semibold">
                Estimated Metal VRAM
              </div>
              <div className="mt-1 font-mono text-xs font-semibold text-emerald-400">
                {formatBytes(hardware.metal_vram_limit_bytes)}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* ── Section 2: Model Directories ── */}
      <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-xs font-semibold text-[var(--color-editor-fg)]">
              Model Directories
            </div>
            <div className="text-[11px] text-[var(--color-fg-muted)]">
              Folders watched for GGUF model files
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => handleScan()}
              disabled={scanning}
              className="flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition disabled:opacity-50"
            >
              <RefreshCw size={12} className={scanning ? 'animate-spin' : ''} />
              {scanning ? 'Scanning…' : 'Scan Models'}
            </button>
            <button
              type="button"
              onClick={() => setShowAddFolder((prev) => !prev)}
              className="flex items-center gap-1.5 rounded-md bg-teal-600 px-2.5 py-1.5 text-xs font-medium text-white hover:bg-teal-500 transition"
            >
              <FolderPlus size={12} />
              Add Directory
            </button>
          </div>
        </div>

        {/* Add Folder Inline Form */}
        {showAddFolder && (
          <div className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-3 space-y-2.5">
            <div className="text-xs font-medium text-[var(--color-editor-fg)]">
              Add GGUF Search Folder
            </div>
            <div className="space-y-2">
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder="Choose a folder containing GGUF files"
                  value={newFolderPath}
                  onChange={(e) => setNewFolderPath(e.target.value)}
                  className="min-w-0 flex-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] outline-none"
                />
                <button type="button" onClick={handlePickFolder} className="rounded-md border border-[var(--color-border-default)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)]">
                  Browse…
                </button>
              </div>
              <input
                type="text"
                placeholder="Folder label (optional)"
                value={newFolderLabel}
                onChange={(e) => setNewFolderLabel(e.target.value)}
                className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] outline-none"
              />
            </div>
            <div className="flex items-center justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => {
                  setShowAddFolder(false);
                  setNewFolderPath('');
                  setNewFolderLabel('');
                }}
                className="rounded px-2.5 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleAddFolder}
                disabled={!newFolderPath.trim()}
                className="rounded bg-teal-600 px-3 py-1 text-xs font-medium text-white hover:bg-teal-500 disabled:opacity-50"
              >
                Register Folder
              </button>
            </div>
          </div>
        )}

        {/* Folders List */}
        <div className="space-y-1.5">
          {folders.length === 0 ? (
            <div className="py-3 text-center text-xs text-[var(--color-fg-subtle)]">
              No model folders registered. Add a folder to discover its GGUF files.
            </div>
          ) : (
            folders.map((folder) => (
              <div
                key={folder.id}
                className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)]/60 px-3 py-2 text-xs"
              >
                <div className="flex items-center gap-2.5 min-w-0 flex-1">
                  <Folder className="h-4 w-4 text-teal-400 shrink-0" />
                  <div className="min-w-0">
                    <span className="font-medium text-[var(--color-editor-fg)]">
                      {folder.label || folder.folder_path.split('/').pop() || 'Models'}
                    </span>
                    <span className="ml-2 font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                      {folder.folder_path}
                    </span>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => handleRemoveFolder(folder.id)}
                  className="p-1 text-[var(--color-fg-subtle)] hover:text-rose-400 transition"
                  title="Remove folder"
                >
                  <Trash2 size={13} />
                </button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* ── Section 3: Discovered Models Inventory ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-xs font-semibold text-[var(--color-editor-fg)]">
              Local GGUF Models ({models.length})
            </div>
            <div className="text-[11px] text-[var(--color-fg-muted)]">
              Scanning only finds GGUF files. Press Load to verify that this runtime can open the model weights.
            </div>
          </div>
        </div>

        {models.length === 0 ? (
          <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-6 text-center text-xs text-[var(--color-fg-subtle)]">
            {loading ? 'Inspecting local model files…' : 'No GGUF models detected in registered directories. Add a folder above to scan.'}
          </div>
        ) : (
          <div className="space-y-2">
            {models.map((m) => {
              const isLoaded = instances.some(
                (inst) => inst.model_path === m.file_path
              );
              const isOperating = loadingModelPath === m.file_path;
              const loadError = modelLoadErrors[m.file_path];

              return (
                <div
                  key={m.file_path}
                  className={`flex flex-col sm:flex-row sm:items-center justify-between gap-3 rounded-xl border p-3.5 transition text-xs ${
                    isLoaded
                      ? 'border-emerald-500/40 bg-emerald-500/5'
                      : 'border-[var(--color-border-muted)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
                  }`}
                >
                  <div className="min-w-0 flex-1 space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold text-[var(--color-editor-fg)] truncate">
                        {m.model_name || m.file_name}
                      </span>
                      {isLoaded ? (
                        <span className="inline-flex items-center gap-1 rounded bg-emerald-500/20 px-1.5 py-0.5 text-[9px] font-semibold text-emerald-300 border border-emerald-500/30">
                          <CheckCircle2 size={11} />
                          Load succeeded · ready to try in chat
                        </span>
                      ) : isOperating ? (
                        <span className="text-[10px] text-[var(--color-fg-muted)]">Checking and loading weights…</span>
                      ) : loadError ? (
                        <span className="inline-flex items-center gap-1 text-[10px] text-rose-400">
                          <XCircle size={11} /> Load failed
                        </span>
                      ) : (
                        renderCompatBadge(m.compatibility)
                      )}
                    </div>
                    {loadError && !isLoaded && (
                      <div role="alert" className="text-[11px] text-rose-400 break-words">
                        {loadError}
                      </div>
                    )}

                    <div className="flex flex-wrap items-center gap-2 font-mono text-[10px] text-[var(--color-fg-subtle)]">
                      <span>Size: {formatBytes(m.file_size_bytes)}</span>
                      <span>·</span>
                      <span>Arch: {m.architecture || 'gguf'}</span>
                      {m.quantization && (
                        <>
                          <span>·</span>
                          <span className="rounded bg-[var(--color-surface-2)] px-1 py-0.2 text-[var(--color-fg-muted)]">
                            {m.quantization}
                          </span>
                        </>
                      )}
                      <span>·</span>
                      <span className="truncate max-w-[220px]" title={m.file_path}>
                        {m.file_path}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    {isLoaded ? (
                      <button
                        type="button"
                        onClick={() => handleUnloadModel(m)}
                        disabled={isOperating}
                        className="flex items-center gap-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 px-2.5 py-1.5 text-xs font-medium text-rose-300 hover:bg-rose-500/20 transition disabled:opacity-50"
                      >
                        <PowerOff size={12} />
                        {isOperating ? 'Unloading…' : 'Unload'}
                      </button>
                    ) : (
                      <button
                        type="button"
                        onClick={() => handleLoadModel(m)}
                        disabled={isOperating}
                        className="flex items-center gap-1.5 rounded-lg border border-teal-500/30 bg-teal-500/10 px-3 py-1.5 text-xs font-medium text-teal-300 hover:bg-teal-500/20 transition disabled:opacity-50"
                      >
                        <Play size={12} />
                        {isOperating ? 'Loading…' : loadError ? 'Retry load' : 'Load'}
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {onNavigateToProviders && (
        <button
          type="button"
          onClick={onNavigateToProviders}
          className="text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
        >
          Configure an optional model server in Providers
        </button>
      )}
    </div>
  );
};
