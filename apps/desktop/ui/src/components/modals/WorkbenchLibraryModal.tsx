import React, { useState } from 'react';
import {
  X,
  CheckCircle2,
  Sparkles,
  Layers,
  Power,
  Loader2,
  Download,
  AlertTriangle,
  ArrowUpRight,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import {
  BUILTIN_CONTRIBUTIONS,
  canonicalWorkbenchId,
} from '@/components/workbenches/registry';
import type { ActiveWorkbench } from '@/types/workbench';

const WORKBENCH_PREREQS: Record<string, string> = {
  assistant: 'Core Runtime (Built-in)',
  coding: 'Git Workspace',
  research: 'Python Kernel (Lazy)',
  quant: 'Market Data / SQLite',
};

export const WorkbenchLibraryModal: React.FC = () => {
  const {
    isWorkbenchLibraryOpen,
    setIsWorkbenchLibraryOpen,
    installedWorkbenches,
    workbenchStates,
    refreshInstalledWorkbenches,
    setActiveWorkbench,
    activeWorkbench,
    showToast,
  } = useAppContext();

  const [loadingId, setLoadingId] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  if (!isWorkbenchLibraryOpen) return null;

  const installedCanonical = new Set(
    (installedWorkbenches || []).map((id) => canonicalWorkbenchId(id))
  );

  const handleAction = async (
    canonicalId: string,
    state: 'builtin' | 'active' | 'disabled' | 'not_installed'
  ) => {
    setLoadingId(canonicalId);
    setActionError(null);
    try {
      if (state === 'active') {
        await daemonClient.disablePackage(canonicalId);
        const modeAlias: ActiveWorkbench =
          canonicalId === 'coding' ? 'engineering' : (canonicalId as ActiveWorkbench);
        if (activeWorkbench === modeAlias || activeWorkbench === (canonicalId as ActiveWorkbench)) {
          setActiveWorkbench('copilot');
        }
        showToast?.(`Disabled ${canonicalId} workbench. Existing tasks & data remain preserved.`);
      } else if (state === 'disabled') {
        await daemonClient.enablePackage(canonicalId);
        showToast?.(`Enabled ${canonicalId} workbench.`);
      } else {
        await daemonClient.installPackage(canonicalId);
        await daemonClient.enablePackage(canonicalId);
        showToast?.(`Installed and enabled ${canonicalId} workbench.`);
      }
      await refreshInstalledWorkbenches();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setActionError(`Failed to update '${canonicalId}': ${msg}`);
    } finally {
      setLoadingId(null);
    }
  };

  const handleOpenWorkbench = (canonicalId: string) => {
    const targetMode: ActiveWorkbench =
      canonicalId === 'coding'
        ? 'engineering'
        : canonicalId === 'assistant'
        ? 'copilot'
        : (canonicalId as ActiveWorkbench);
    setActiveWorkbench(targetMode);
    setIsWorkbenchLibraryOpen(false);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 animate-in fade-in duration-150"
      onClick={() => setIsWorkbenchLibraryOpen(false)}
    >
      <div
        className="relative w-full max-w-2xl bg-[#141416] border border-white/12 rounded-xl shadow-2xl overflow-hidden flex flex-col text-neutral-200"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="workbench-library-title"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-white/8 bg-[#18181b]">
          <div className="flex items-center gap-2.5">
            <div className="size-8 rounded-lg bg-white/6 border border-white/10 flex items-center justify-center text-white">
              <Layers className="size-4" />
            </div>
            <div>
              <h2 id="workbench-library-title" className="text-sm font-semibold text-white tracking-tight">
                Workbench Library
              </h2>
              <p className="text-xs text-neutral-400">
                Extend Custos with specialized domain tools, verifiers, and UI panes
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={() => setIsWorkbenchLibraryOpen(false)}
            className="p-1 rounded-md text-neutral-400 hover:text-white hover:bg-white/6 transition cursor-pointer"
            aria-label="Close dialog"
          >
            <X className="size-4" />
          </button>
        </div>

        {/* Content list */}
        <div className="p-6 space-y-3.5 max-h-[70vh] overflow-y-auto">
          {actionError && (
            <div className="flex items-start gap-2 rounded-lg border border-red-500/30 bg-red-500/10 px-3.5 py-2.5 text-xs text-red-200">
              <AlertTriangle className="size-4 shrink-0 text-red-400 mt-0.5" />
              <span className="flex-1 leading-relaxed">{actionError}</span>
              <button
                type="button"
                onClick={() => setActionError(null)}
                className="text-red-300/70 hover:text-white cursor-pointer"
                aria-label="Dismiss error"
              >
                <X className="size-3.5" />
              </button>
            </div>
          )}

          {BUILTIN_CONTRIBUTIONS.map((contrib) => {
            const Icon = contrib.icon;
            const pkgState = contrib.builtin
              ? 'builtin'
              : workbenchStates?.[contrib.canonicalId] ||
                (installedCanonical.has(contrib.canonicalId) ? 'active' : 'not_installed');
            const isActive = pkgState === 'builtin' || pkgState === 'active';
            const isDisabled = pkgState === 'disabled';

            return (
              <div
                key={contrib.canonicalId}
                className={`flex items-start justify-between gap-4 p-4 rounded-lg border transition ${
                  isActive
                    ? 'bg-white/[0.03] border-white/10'
                    : 'bg-black/20 border-white/6 hover:border-white/12'
                }`}
              >
                <div className="flex items-start gap-3.5 min-w-0">
                  <div
                    className={`size-10 rounded-lg flex items-center justify-center shrink-0 border ${
                      isActive
                        ? 'bg-white/10 border-white/15 text-white'
                        : 'bg-white/4 border-white/8 text-neutral-400'
                    }`}
                  >
                    <Icon className="size-5" />
                  </div>
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2 mb-1">
                      <span className="text-sm font-medium text-white">
                        {contrib.label}
                      </span>
                      {pkgState === 'builtin' ? (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-neutral-800 text-neutral-300 border border-white/10">
                          Built-in Core
                        </span>
                      ) : pkgState === 'active' ? (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/25">
                          Active
                        </span>
                      ) : isDisabled ? (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/25">
                          Installed (Disabled)
                        </span>
                      ) : (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-500/15 text-blue-400 border border-blue-500/25">
                          On-Demand
                        </span>
                      )}
                      {WORKBENCH_PREREQS[contrib.canonicalId] && (
                        <span className="px-1.5 py-0.5 rounded text-[10px] bg-white/[0.04] text-neutral-400 border border-white/6">
                          {WORKBENCH_PREREQS[contrib.canonicalId]}
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-neutral-400 max-w-md leading-relaxed">
                      {contrib.description}
                    </p>
                  </div>
                </div>

                <div className="shrink-0 flex items-center gap-2 pt-0.5">
                  {pkgState === 'builtin' ? (
                    <div className="flex items-center gap-1.5 text-xs text-neutral-400 font-medium px-2.5 py-1">
                      <CheckCircle2 className="size-4 text-emerald-400" />
                      <span>Always On</span>
                    </div>
                  ) : (
                    <>
                      {pkgState === 'active' && (
                        <button
                          type="button"
                          onClick={() => handleOpenWorkbench(contrib.canonicalId)}
                          className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-md text-xs font-medium bg-white/10 hover:bg-white/16 text-white border border-white/15 transition cursor-pointer"
                        >
                          <span>Open</span>
                          <ArrowUpRight className="size-3.5" />
                        </button>
                      )}
                      <button
                        type="button"
                        disabled={loadingId === contrib.canonicalId}
                        onClick={() => handleAction(contrib.canonicalId, pkgState)}
                        className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition cursor-pointer ${
                          loadingId === contrib.canonicalId
                            ? 'opacity-50 cursor-not-allowed bg-white/5 text-neutral-400'
                            : pkgState === 'active'
                            ? 'bg-red-500/10 hover:bg-red-500/20 text-red-300 border border-red-500/20'
                            : pkgState === 'disabled'
                            ? 'bg-amber-500/15 hover:bg-amber-500/25 text-amber-200 border border-amber-500/30'
                            : 'bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-300 border border-emerald-500/30'
                        }`}
                      >
                        {loadingId === contrib.canonicalId ? (
                          <>
                            <Loader2 className="size-3.5 animate-spin" />
                            <span>Applying...</span>
                          </>
                        ) : pkgState === 'active' ? (
                          <>
                            <Power className="size-3.5 text-red-400" />
                            <span>Disable</span>
                          </>
                        ) : pkgState === 'disabled' ? (
                          <>
                            <Power className="size-3.5 text-amber-300" />
                            <span>Enable</span>
                          </>
                        ) : (
                          <>
                            <Download className="size-3.5 text-emerald-400" />
                            <span>Install &amp; Enable</span>
                          </>
                        )}
                      </button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-6 py-3 border-t border-white/8 bg-[#18181b]/60 text-xs text-neutral-400">
          <div className="flex items-center gap-1.5">
            <Sparkles className="size-3.5 text-amber-400" />
            <span>Installed packages share canonical tasks and authority grants</span>
          </div>
          <button
            type="button"
            onClick={() => setIsWorkbenchLibraryOpen(false)}
            className="px-3 py-1 rounded bg-white/8 hover:bg-white/12 text-white text-xs font-medium transition cursor-pointer"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
