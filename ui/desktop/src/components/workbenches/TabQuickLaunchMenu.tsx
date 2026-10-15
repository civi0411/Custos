import React, { useState, useEffect, useMemo, useRef } from 'react';
import {
  Search,
  ChevronRight,
  Plus,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { HarnessDescriptor } from '@/types/domain';
import type { OrcaTabId, ResourceDefinition } from '@/components/workbenches/OrcaTabbedContainer';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';

export interface TabQuickLaunchMenuProps {
  isOpen: boolean;
  onClose: () => void;
  pickerResources: ResourceDefinition[];
  onOpenResource: (resourceId: OrcaTabId) => void;
  onLaunchAgent: (harness: HarnessDescriptor) => void;
  activeWorkspaceTitle?: string;
  mode?: import('@/components/shell/AppHeader').ActiveWorkbench;
}

export const TabQuickLaunchMenu: React.FC<TabQuickLaunchMenuProps> = ({
  isOpen,
  onClose,
  pickerResources,
  onOpenResource,
  onLaunchAgent,
  activeWorkspaceTitle,
  mode,
}) => {
  const isEngineering = mode === 'engineering';
  const [harnesses, setHarnesses] = useState<HarnessDescriptor[]>([]);
  const [loading, setLoading] = useState(false);
  const [harnessError, setHarnessError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!isOpen || !isEngineering) return;
    let mounted = true;
    setLoading(true);
    setHarnessError(null);

    daemonClient
      .listHarnesses()
      .then((list) => {
        if (!mounted) return;
        setHarnesses(list);
      })
      .catch((err) => {
        console.warn('Failed to probe coding agent harnesses:', err);
        if (mounted) setHarnessError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });

    return () => {
      mounted = false;
    };
  }, [isOpen, isEngineering]);

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 50);
    } else {
      setSearchQuery('');
    }
  }, [isOpen]);

  // Keyboard shortcut: Esc to close
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const getAgentIcon = (id: string) => getAgentBrandIcon(id, 14, 'shrink-0');

  const filteredHarnesses = useMemo(() => {
    if (!searchQuery.trim()) return harnesses;
    const query = searchQuery.toLowerCase();
    return harnesses.filter(
      (h) =>
        h.name.toLowerCase().includes(query) ||
        h.id.toLowerCase().includes(query) ||
        h.description.toLowerCase().includes(query) ||
        h.binary_path.toLowerCase().includes(query)
    );
  }, [harnesses, searchQuery]);

  const installedHarnesses = useMemo(
    () => filteredHarnesses.filter((h) => h.is_available),
    [filteredHarnesses]
  );
  const uninstalledHarnesses = useMemo(
    () => filteredHarnesses.filter((h) => !h.is_available),
    [filteredHarnesses]
  );

  const filteredResources = useMemo(() => {
    const list = isEngineering
      ? pickerResources.filter((r) => r.id !== 'agents')
      : pickerResources.filter((r) => r.id !== 'agents');
    if (!searchQuery.trim()) return list;
    const query = searchQuery.toLowerCase();
    return list.filter(
      (r) =>
        r.title.toLowerCase().includes(query) ||
        r.id.toLowerCase().includes(query) ||
        r.description.toLowerCase().includes(query)
    );
  }, [pickerResources, searchQuery, isEngineering]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-end p-4 pt-12 bg-black/30 backdrop-blur-xs"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-label="New Tab & Agent Quick Launch"
        className="w-[360px] max-h-[85vh] flex flex-col rounded-xl border border-border-default bg-surface-1 shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-100"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header & Search */}
        <div className="p-3 border-b border-border-default bg-surface-2/40 shrink-0">
          <div className="flex items-center justify-between mb-2">
            <span className="text-[11px] font-semibold tracking-wider uppercase text-fg-muted">
              Launch Agent / New Tab
            </span>
            {activeWorkspaceTitle && (
              <span className="text-[10px] text-fg-muted truncate max-w-[150px]">
                {activeWorkspaceTitle}
              </span>
            )}
          </div>
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-fg-muted" />
            <input
              ref={searchInputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search coding agents or resources..."
              className="w-full h-8 pl-8 pr-3 text-xs rounded-lg border border-border-default bg-surface-1 text-fg-editor placeholder:text-fg-muted/60 focus:outline-hidden focus:border-focus focus:ring-1 focus:ring-focus"
            />
          </div>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto p-2 space-y-4 no-scrollbar">
          {/* Section 1: Detected Coding Agents (Only in Engineering Workbench) */}
          {isEngineering && (
            <div>
              <div className="flex items-center justify-between px-2 py-1 text-[11px] font-semibold text-fg-muted">
                <span>Coding Agents (Parallel Tasks)</span>
                <span className="text-[10px] text-emerald-400 font-mono">
                  {installedHarnesses.length} ready
                </span>
              </div>

              <div className="space-y-0.5 mt-1">
                {installedHarnesses.map((harness) => (
                  <button
                    key={harness.id}
                    type="button"
                    onClick={() => onLaunchAgent(harness)}
                    className="group flex w-full items-center justify-between gap-2.5 rounded-lg px-2.5 py-2 text-left hover:bg-surface-2 transition border border-transparent hover:border-border-default/60"
                    title={`${harness.description} (CLI: ${harness.binary_path})`}
                  >
                    <div className="flex items-center gap-2.5 min-w-0 flex-1">
                      <div className="p-1 rounded-md bg-surface-2/80 group-hover:bg-surface-1 transition shrink-0">
                        {getAgentIcon(harness.id)}
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-1.5">
                          <span className="text-xs font-medium text-fg-editor truncate">
                            {harness.name}
                          </span>
                          <span className="inline-flex items-center gap-1 text-[10px] text-emerald-400 font-medium shrink-0">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                            Ready
                          </span>
                        </div>
                        <p className="text-[10px] text-fg-muted truncate">
                          {harness.description}
                        </p>
                      </div>
                    </div>

                    <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition">
                      <span className="text-[10px] text-fg-muted group-hover:text-fg-editor font-medium">
                        Launch Tab
                      </span>
                      <Plus className="w-3 h-3 text-fg-muted group-hover:text-fg-editor" />
                    </div>
                  </button>
                ))}

                {harnessError && <p role="alert" className="px-2 py-2 text-xs text-rose-400">Could not detect coding agents: {harnessError}</p>}
                {installedHarnesses.length === 0 && !loading && !harnessError && (
                  <div className="px-3 py-2 text-xs text-fg-muted italic">
                    No installed coding agents found on PATH.
                  </div>
                )}
              </div>

              {/* Uninstalled / Not Detected Agents */}
              {uninstalledHarnesses.length > 0 && (
                <details className="mt-2 group">
                  <summary className="flex items-center justify-between px-2 py-1 text-[10px] text-fg-muted cursor-pointer hover:text-fg-editor select-none">
                    <span>Other Agents ({uninstalledHarnesses.length} uninstalled)</span>
                    <ChevronRight className="w-3 h-3 transition group-open:rotate-90" />
                  </summary>
                  <div className="space-y-0.5 mt-1 pt-1 border-t border-border-default/40">
                    {uninstalledHarnesses.map((harness) => (
                      <div
                        key={harness.id}
                        className="flex w-full items-center justify-between gap-2.5 rounded-lg px-2.5 py-1.5 opacity-60 hover:opacity-100 transition hover:bg-surface-2/50"
                        title={`Command '${harness.binary_path}' not found in PATH`}
                      >
                        <div className="flex items-center gap-2.5 min-w-0 flex-1">
                          <div className="p-1 rounded-md bg-surface-2 shrink-0">
                            {getAgentIcon(harness.id)}
                          </div>
                          <div className="min-w-0 flex-1">
                            <span className="text-xs text-fg-editor truncate block">
                              {harness.name}
                            </span>
                            <span className="text-[10px] text-fg-muted">
                              {harness.binary_path} CLI not found
                            </span>
                          </div>
                        </div>
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-surface-2 text-fg-muted shrink-0">
                          not installed
                        </span>
                      </div>
                    ))}
                  </div>
                </details>
              )}
            </div>
          )}

          {/* Section 2: Workspace Resources */}
          <div>
            <div className="px-2 py-1 text-[11px] font-semibold text-fg-muted">
              Workspace Surfaces & Views
            </div>
            <div className="space-y-0.5 mt-1">
              {filteredResources.map((resource) => {
                const Icon = resource.icon;
                return (
                  <button
                    key={resource.id}
                    type="button"
                    onClick={() => onOpenResource(resource.id)}
                    className="group flex w-full items-center justify-between gap-2.5 rounded-lg px-2.5 py-2 text-left hover:bg-surface-2 transition border border-transparent hover:border-border-default/60"
                    title={resource.reason || resource.description}
                  >
                    <div className="flex items-center gap-2.5 min-w-0 flex-1">
                      <div className="p-1 rounded-md bg-surface-2/80 group-hover:bg-surface-1 transition shrink-0">
                        <Icon className="w-3.5 h-3.5 text-fg-muted group-hover:text-fg-editor" />
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-1.5">
                          <span className="text-xs font-medium text-fg-editor truncate">
                            {resource.title}
                          </span>
                          {resource.status !== 'available' && (
                            <span className="text-[9px] px-1 rounded bg-amber-500/10 text-amber-400 shrink-0">
                              {resource.status === 'degraded' ? 'Limited' : 'Unavailable'}
                            </span>
                          )}
                        </div>
                        <p className="text-[10px] text-fg-muted truncate">
                          {resource.description}
                        </p>
                      </div>
                    </div>

                    <ChevronRight className="w-3.5 h-3.5 text-fg-muted opacity-0 group-hover:opacity-100 transition shrink-0" />
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        {/* Footer info */}
        <div className="p-2.5 px-3 border-t border-border-default bg-surface-2/30 flex items-center justify-between text-[10px] text-fg-muted shrink-0">
          <span>Each launched agent runs in its own tab</span>
          <kbd className="px-1.5 py-0.5 rounded bg-surface-1 border border-border-default font-mono">
            ESC
          </kbd>
        </div>
      </div>
    </div>
  );
};
