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
  anchor?: { top: number; left: number } | null;
}

export const TabQuickLaunchMenu: React.FC<TabQuickLaunchMenuProps> = ({
  isOpen,
  onClose,
  pickerResources,
  onOpenResource,
  onLaunchAgent,
  activeWorkspaceTitle,
  mode,
  anchor,
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

  // Section partitioning per workbench mode
  const { section1Title, section2Title, section1Resources, section2Resources } = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    const matchesSearch = (r?: ResourceDefinition): r is ResourceDefinition =>
      Boolean(
        r &&
          (!query ||
            r.title.toLowerCase().includes(query) ||
            r.id.toLowerCase().includes(query) ||
            r.description.toLowerCase().includes(query))
      );

    if (mode === 'research') {
      const sec1Ids: OrcaTabId[] = ['literature', 'lab', 'findings', 'artifacts'];
      const sec2Ids: OrcaTabId[] = ['conversation'];

      const sec1 = sec1Ids
        .map((id) => pickerResources.find((r) => r.id === id))
        .filter(matchesSearch);

      const sec2 = sec2Ids
        .map((id) => pickerResources.find((r) => r.id === id))
        .filter(matchesSearch);

      return {
        section1Title: 'Research Surfaces',
        section2Title: 'Scientific Dialogue',
        section1Resources: sec1,
        section2Resources: sec2,
      };
    }

    if (mode === 'copilot') {
      const sec1Ids: OrcaTabId[] = ['tasks', 'drafts'];
      const sec2Ids: OrcaTabId[] = ['conversation'];

      const sec1 = sec1Ids
        .map((id) => pickerResources.find((r) => r.id === id))
        .filter(matchesSearch);

      const sec2 = sec2Ids
        .map((id) => pickerResources.find((r) => r.id === id))
        .filter(matchesSearch);

      return {
        section1Title: 'Executive Workflows',
        section2Title: 'Assistant Dialogue',
        section1Resources: sec1,
        section2Resources: sec2,
      };
    }

    // Default: Engineering
    const engIds: OrcaTabId[] = ['terminal', 'files', 'browser', 'emulator', 'worktrees', 'conversation'];
    const engResources = engIds
      .map((id) => pickerResources.find((r) => r.id === id))
      .filter(matchesSearch);

    return {
      section1Title: 'Coding Agents',
      section2Title: 'Engineering Surfaces',
      section1Resources: [], // Harnesses are rendered in Section 1
      section2Resources: engResources,
    };
  }, [pickerResources, searchQuery, mode]);

  const menuTitle = useMemo(() => {
    if (mode === 'research') return 'Research Surfaces';
    if (mode === 'copilot') return 'Copilot Surfaces';
    return 'Engineering Surfaces';
  }, [mode]);

  const searchPlaceholder = useMemo(() => {
    if (mode === 'research') return 'Search lab, sources, findings, methods…';
    if (mode === 'copilot') return 'Search tasks, drafts, calendar, outbox…';
    return 'Search agents, files, terminal, browser, emulator…';
  }, [mode]);

  const footerText = 'Quick launch';

  const renderResourceButton = (resource: ResourceDefinition) => {
    const Icon = resource.icon;
    return (
      <button
        key={resource.id}
        type="button"
        onClick={() => onOpenResource(resource.id)}
        className="group flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-surface-2 transition border border-transparent hover:border-border-default/50 cursor-pointer"
        title={resource.reason || resource.description}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <div className="p-1 rounded-md bg-surface-2/80 group-hover:bg-surface-1 transition shrink-0">
            <Icon className="w-3.5 h-3.5 text-fg-muted group-hover:text-fg-editor" />
          </div>
          <span className="text-xs font-medium text-fg-editor truncate flex-1">
            {resource.title}
          </span>
          {resource.status !== 'available' && (
            <span className="text-[9px] px-1 rounded bg-amber-500/10 text-amber-400 shrink-0">
              {resource.status === 'degraded' ? 'Limited' : 'Unavailable'}
            </span>
          )}
        </div>
        <Plus className="w-3 h-3 text-fg-muted opacity-0 group-hover:opacity-100 transition shrink-0" />
      </button>
    );
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 bg-black/25 backdrop-blur-[2px]"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-label={menuTitle}
        style={anchor ? {
          position: 'fixed',
          top: `${anchor.top}px`,
          left: `${Math.max(12, Math.min(anchor.left, window.innerWidth - 308))}px`,
        } : undefined}
        className={`w-[296px] max-h-[75vh] flex flex-col rounded-xl border border-white/12 bg-surface-1/95 backdrop-blur-xl shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-100 ${!anchor ? 'fixed right-4 top-12' : ''}`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header & Search */}
        <div className="p-2.5 pb-2 border-b border-border-default/60 bg-surface-2/40 shrink-0">
          <div className="flex items-center justify-between mb-1.5 px-0.5">
            <span className="text-[10px] font-semibold tracking-wider uppercase text-fg-muted">
              {menuTitle}
            </span>
            {activeWorkspaceTitle && (
              <span className="text-[10px] text-fg-muted/70 truncate max-w-[120px]">
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
              placeholder={searchPlaceholder}
              className="w-full h-7 pl-8 pr-3 text-xs rounded-lg border border-border-default/70 bg-surface-1 text-fg-editor placeholder:text-fg-muted/50 focus:outline-hidden focus:border-focus"
            />
          </div>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto p-1.5 space-y-3 no-scrollbar">
          {/* Section 1: Coding Agents (in Engineering) OR Domain Primary Runtimes (in Research / Copilot) */}
          {isEngineering ? (
            <div>
              <div className="flex items-center justify-between px-2 py-0.5 text-[10px] font-semibold text-fg-muted">
                <span>{section1Title}</span>
                <span className="text-[10px] text-white/80 font-mono">
                  {installedHarnesses.length} ready
                </span>
              </div>

              <div className="space-y-0.5 mt-1">
                {installedHarnesses.map((harness) => (
                  <button
                    key={harness.id}
                    type="button"
                    onClick={() => onLaunchAgent(harness)}
                    className="group flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-surface-2 transition border border-transparent hover:border-border-default/50 cursor-pointer"
                    title={harness.description}
                  >
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <div className="p-1 rounded-md bg-surface-2/80 group-hover:bg-surface-1 transition shrink-0">
                        {getAgentIcon(harness.id)}
                      </div>
                      <span className="text-xs font-medium text-fg-editor truncate flex-1">
                        {harness.name}
                      </span>
                      <span className="inline-flex items-center gap-1 text-[9px] text-white/70 font-medium shrink-0">
                        <span className="w-1.5 h-1.5 rounded-full bg-white/80 animate-pulse" />
                        Ready
                      </span>
                    </div>
                    <Plus className="w-3 h-3 text-fg-muted opacity-0 group-hover:opacity-100 transition shrink-0" />
                  </button>
                ))}

                {harnessError && <p role="alert" className="px-2 py-1 text-[11px] text-rose-400">Could not detect coding agents: {harnessError}</p>}
                {installedHarnesses.length === 0 && !loading && !harnessError && (
                  <div className="px-3 py-1.5 text-xs text-fg-muted italic">
                    No installed coding agents found.
                  </div>
                )}
              </div>

              {/* Uninstalled / Not Detected Agents */}
              {uninstalledHarnesses.length > 0 && (
                <details className="mt-1 group">
                  <summary className="flex items-center justify-between px-2 py-0.5 text-[10px] text-fg-muted cursor-pointer hover:text-fg-editor select-none">
                    <span>Other ({uninstalledHarnesses.length})</span>
                    <ChevronRight className="w-3 h-3 transition group-open:rotate-90" />
                  </summary>
                  <div className="space-y-0.5 mt-1 pt-1 border-t border-border-default/30">
                    {uninstalledHarnesses.map((harness) => (
                      <div
                        key={harness.id}
                        className="flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1 opacity-50 text-xs"
                        title={`Command '${harness.binary_path}' not found`}
                      >
                        <div className="flex items-center gap-2 min-w-0 flex-1">
                          {getAgentIcon(harness.id)}
                          <span className="text-xs text-fg-editor truncate">{harness.name}</span>
                        </div>
                        <span className="text-[9px] font-mono text-fg-muted">not installed</span>
                      </div>
                    ))}
                  </div>
                </details>
              )}
            </div>
          ) : (
            section1Resources.length > 0 && (
              <div>
                <div className="px-2 py-0.5 text-[10px] font-semibold text-fg-muted">
                  {section1Title}
                </div>
                <div className="space-y-0.5 mt-1">
                  {section1Resources.map(renderResourceButton)}
                </div>
              </div>
            )
          )}

          {/* Section 2: Secondary Runtimes & Workspace Tools */}
          {section2Resources.length > 0 && (
            <div>
              <div className="px-2 py-0.5 text-[10px] font-semibold text-fg-muted">
                {section2Title}
              </div>
              <div className="space-y-0.5 mt-1">
                {section2Resources.map(renderResourceButton)}
              </div>
            </div>
          )}

          {/* Fallback if search yields zero results */}
          {searchQuery.trim() &&
            (!isEngineering || installedHarnesses.length === 0) &&
            section1Resources.length === 0 &&
            section2Resources.length === 0 && (
              <div className="px-3 py-6 text-center text-xs text-fg-muted italic">
                No matching surfaces found for "{searchQuery}".
              </div>
            )}
        </div>

        {/* Footer info */}
        <div className="p-2 px-3 border-t border-border-default/60 bg-surface-2/20 flex items-center justify-between text-[10px] text-fg-muted shrink-0">
          <span className="truncate pr-2 text-fg-muted/70">{footerText}</span>
          <kbd className="px-1.5 py-0.5 rounded bg-surface-1 border border-border-default font-mono text-[9px] shrink-0">
            ESC
          </kbd>
        </div>
      </div>
    </div>
  );
};
