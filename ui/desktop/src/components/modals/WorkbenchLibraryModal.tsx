import React, { useState } from 'react';
import {
  X,
  CheckCircle2,
  Sparkles,
  Layers,
  Power,
  Loader2,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import {
  BUILTIN_CONTRIBUTIONS,
  canonicalWorkbenchId,
} from '@/components/workbenches/registry';

export const WorkbenchLibraryModal: React.FC = () => {
  const {
    isWorkbenchLibraryOpen,
    setIsWorkbenchLibraryOpen,
    installedWorkbenches,
    refreshInstalledWorkbenches,
  } = useAppContext();

  const [loadingId, setLoadingId] = useState<string | null>(null);

  if (!isWorkbenchLibraryOpen) return null;

  const installedCanonical = new Set(
    (installedWorkbenches || []).map((id) => canonicalWorkbenchId(id))
  );

  const handleToggle = async (canonicalId: string, isCurrentlyInstalled: boolean) => {
    setLoadingId(canonicalId);
    try {
      if (isCurrentlyInstalled) {
        await daemonClient.disablePackage(canonicalId);
      } else {
        await daemonClient.installPackage(canonicalId);
      }
      await refreshInstalledWorkbenches();
    } catch (err) {
      console.error('Failed to toggle package:', err);
    } finally {
      setLoadingId(null);
    }
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
          {BUILTIN_CONTRIBUTIONS.map((contrib) => {
            const Icon = contrib.icon;
            const isInstalled = contrib.builtin || installedCanonical.has(contrib.canonicalId);

            return (
              <div
                key={contrib.canonicalId}
                className={`flex items-start justify-between p-4 rounded-lg border transition ${
                  isInstalled
                    ? 'bg-white/[0.03] border-white/8'
                    : 'bg-black/20 border-white/6 hover:border-white/12'
                }`}
              >
                <div className="flex items-start gap-3.5">
                  <div
                    className={`size-10 rounded-lg flex items-center justify-center shrink-0 border ${
                      isInstalled
                        ? 'bg-white/10 border-white/15 text-white'
                        : 'bg-white/4 border-white/8 text-neutral-400'
                    }`}
                  >
                    <Icon className="size-5" />
                  </div>
                  <div>
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-sm font-medium text-white">
                        {contrib.label}
                      </span>
                      {contrib.builtin ? (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-neutral-800 text-neutral-300 border border-white/10">
                          Built-in
                        </span>
                      ) : isInstalled ? (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/25">
                          Active
                        </span>
                      ) : (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-500/15 text-blue-400 border border-blue-500/25">
                          Available
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-neutral-400 max-w-md leading-relaxed">
                      {contrib.description}
                    </p>
                  </div>
                </div>

                <div className="shrink-0 pt-0.5">
                  {contrib.builtin ? (
                    <div className="flex items-center gap-1.5 text-xs text-neutral-400 font-medium px-2.5 py-1">
                      <CheckCircle2 className="size-4 text-emerald-400" />
                      <span>Built-in</span>
                    </div>
                  ) : (
                    <button
                      type="button"
                      disabled={loadingId === contrib.canonicalId}
                      onClick={() => handleToggle(contrib.canonicalId, isInstalled)}
                      className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition cursor-pointer ${
                        loadingId === contrib.canonicalId
                          ? 'opacity-50 cursor-not-allowed bg-white/5 text-neutral-400'
                          : isInstalled
                          ? 'bg-red-500/10 hover:bg-red-500/20 text-red-300 border border-red-500/20'
                          : 'bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-300 border border-emerald-500/30'
                      }`}
                    >
                      {loadingId === contrib.canonicalId ? (
                        <>
                          <Loader2 className="size-3.5 animate-spin" />
                          <span>Updating...</span>
                        </>
                      ) : isInstalled ? (
                        <>
                          <Power className="size-3.5 text-red-400" />
                          <span>Disable</span>
                        </>
                      ) : (
                        <>
                          <Power className="size-3.5 text-emerald-400" />
                          <span>Activate</span>
                        </>
                      )}
                    </button>
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
