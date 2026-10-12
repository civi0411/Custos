import React, { useState, useMemo, useEffect, useRef } from 'react';
import {
  Search,
  Clock,
  Sparkles,
  Server,
  Layers,
  ArrowRight,
  CornerDownLeft,
  AlertTriangle,
  XCircle,
  CheckCircle2,
} from 'lucide-react';
import { ResourceDefinition } from '../views/OrcaTabbedContainer';
import { daemonClient } from '@/api/daemon_client';
import type { ContextualActionDescriptor } from '@/types/domain';

export type PaletteCategory = 'recent' | 'task' | 'env' | 'library';

interface QuickResourcePaletteProps {
  resources: ResourceDefinition[];
  mode: 'code' | 'research' | 'chat';
  sessionId?: string;
  onOpenResource: (resource: ResourceDefinition) => void;
  recentResourceIds?: string[];
  className?: string;
}

export const QuickResourcePalette: React.FC<QuickResourcePaletteProps> = ({
  resources,
  mode,
  sessionId,
  onOpenResource,
  recentResourceIds = ['files', 'terminal', 'changes'],
  className = '',
}) => {
  const [activeCategory, setActiveCategory] = useState<PaletteCategory>('task');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [selectedActions, setSelectedActions] = useState<ContextualActionDescriptor[]>([]);
  const [loadingActions, setLoadingActions] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Group definitions
  const categories: { id: PaletteCategory; label: string; shortcut: string; icon: React.ElementType }[] = [
    { id: 'recent', label: 'Recent', shortcut: '1', icon: Clock },
    { id: 'task', label: 'Task Scoped', shortcut: '2', icon: Sparkles },
    { id: 'env', label: 'Environment', shortcut: '3', icon: Server },
    { id: 'library', label: 'Library', shortcut: '4', icon: Layers },
  ];

  // Scoped list per category
  const categorizedResources = useMemo(() => {
    switch (activeCategory) {
      case 'recent': {
        const recentList = recentResourceIds
          .map((id) => resources.find((r) => r.id === id))
          .filter(Boolean) as ResourceDefinition[];
        return recentList.length > 0 ? recentList : resources.slice(0, 4);
      }
      case 'task': {
        if (mode === 'code') {
          return resources.filter((r) =>
            ['files', 'changes', 'terminal', 'browser', 'emulator', 'worktrees', 'kanban', 'evidence'].includes(r.id)
          );
        }
        if (mode === 'research') {
          return resources.filter((r) =>
            ['literature', 'experiments', 'claims', 'knowledge', 'synthesis', 'artifacts', 'runs', 'notes'].includes(r.id)
          );
        }
        // Copilot
        return resources.filter((r) =>
          ['notes', 'artifacts', 'browser', 'emulator'].includes(r.id)
        );
      }
      case 'env': {
        return resources.filter((r) =>
          ['terminal', 'worktrees', 'fleet'].includes(r.id)
        );
      }
      case 'library':
      default:
        return resources;
    }
  }, [activeCategory, resources, recentResourceIds, mode]);

  // Filter by search query
  const filteredResources = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return categorizedResources;
    return categorizedResources.filter((res) => {
      const matchTitle = res.title.toLowerCase().includes(q);
      const matchDesc = res.description.toLowerCase().includes(q);
      const matchOps = res.supportedOperations?.some((op) => op.toLowerCase().includes(q));
      return matchTitle || matchDesc || matchOps;
    });
  }, [categorizedResources, searchQuery]);

  // Auto-clamp selectedIndex
  useEffect(() => {
    setSelectedIndex(0);
  }, [searchQuery, activeCategory]);

  const activeResource = filteredResources[selectedIndex] || filteredResources[0];

  // Fetch contextual actions for selected resource from daemon
  useEffect(() => {
    if (!activeResource) {
      setSelectedActions([]);
      return;
    }

    let isMounted = true;
    setLoadingActions(true);
    daemonClient
      .getResourceActions(activeResource.id, sessionId)
      .then((actions) => {
        if (isMounted) {
          setSelectedActions(actions || []);
          setLoadingActions(false);
        }
      })
      .catch(() => {
        if (isMounted) {
          setSelectedActions([]);
          setLoadingActions(false);
        }
      });

    return () => {
      isMounted = false;
    };
  }, [activeResource?.id, sessionId]);

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev + 1) % Math.max(1, filteredResources.length));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev <= 0 ? Math.max(0, filteredResources.length - 1) : prev - 1
      );
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (activeResource) {
        onOpenResource(activeResource);
      }
    } else if (e.altKey || e.metaKey || e.ctrlKey) {
      if (e.key === '1') {
        e.preventDefault();
        setActiveCategory('recent');
      } else if (e.key === '2') {
        e.preventDefault();
        setActiveCategory('task');
      } else if (e.key === '3') {
        e.preventDefault();
        setActiveCategory('env');
      } else if (e.key === '4') {
        e.preventDefault();
        setActiveCategory('library');
      }
    }
  };

  const modeKicker = useMemo(() => {
    if (mode === 'code') return 'Coding Session Scope';
    if (mode === 'research') return 'Research Protocol Scope';
    return 'Copilot Context Scope';
  }, [mode]);

  return (
    <div
      className={`flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-fg-default)] ${className}`}
      onKeyDown={handleKeyDown}
    >
      {/* Top Bar: Search & Category Navigation */}
      <div className="shrink-0 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] px-5 py-4">
        <div className="flex items-center justify-between gap-4 mb-3">
          <div className="flex items-center gap-2">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
              {modeKicker}
            </span>
            <span className="text-[10px] text-[var(--color-fg-muted)]">•</span>
            <span className="text-xs font-medium text-[var(--color-fg-default)]">
              Quick Resource Palette
            </span>
          </div>
          <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-subtle)]">
            <span className="inline-flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono">
              ↑↓
            </span>
            <span>Navigate</span>
            <span className="inline-flex items-center gap-1 rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono">
              ↵
            </span>
            <span>Open</span>
          </div>
        </div>

        {/* Search Input */}
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-[var(--color-fg-subtle)]" />
          <input
            ref={searchInputRef}
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search resources, surfaces, or operations (e.g. terminal, diff, tree)..."
            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] pl-9 pr-4 py-2 text-xs text-[var(--color-fg-default)] placeholder:text-[var(--color-fg-subtle)] focus:border-[var(--color-border-default)] focus:outline-none focus:ring-1 focus:ring-[var(--color-border-default)] transition-colors"
            autoFocus
          />
        </div>

        {/* Category Tabs */}
        <div className="mt-3 flex items-center gap-1">
          {categories.map((cat) => {
            const Icon = cat.icon;
            const isActive = activeCategory === cat.id;
            return (
              <button
                key={cat.id}
                type="button"
                onClick={() => setActiveCategory(cat.id)}
                className={`inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                  isActive
                    ? 'bg-[var(--color-surface-3,var(--color-border-subtle))] text-[var(--color-fg-default)] shadow-xs'
                    : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-fg-default)]'
                }`}
              >
                <Icon className="h-3 w-3" />
                <span>{cat.label}</span>
                <span className="ml-1 text-[9px] text-[var(--color-fg-subtle)]">
                  {cat.shortcut}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Main Split: Resource List (Left) + Detail & Preflight Action Inspector (Right) */}
      <div className="flex flex-1 min-h-0 divide-x divide-[var(--color-border-subtle)]">
        {/* Left: Compact List */}
        <div className="flex-1 overflow-y-auto p-3 space-y-1.5">
          {filteredResources.length === 0 ? (
            <div className="rounded-lg border border-dashed border-[var(--color-border-subtle)] p-8 text-center text-xs text-[var(--color-fg-subtle)]">
              No resources match "{searchQuery}" in {activeCategory} category.
            </div>
          ) : (
            filteredResources.map((resource, index) => {
              const Icon = resource.icon;
              const isSelected = index === selectedIndex;
              const isDegraded = resource.status === 'degraded';
              const isUnavailable = resource.status === 'unavailable';

              return (
                <div
                  key={resource.id}
                  onClick={() => {
                    setSelectedIndex(index);
                  }}
                  onDoubleClick={() => onOpenResource(resource)}
                  className={`group flex items-center justify-between rounded-lg border px-3 py-2.5 transition cursor-pointer ${
                    isSelected
                      ? 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] shadow-xs'
                      : 'border-transparent hover:border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-1)]'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-md border ${
                        isSelected
                          ? 'border-[var(--color-border-default)] bg-[var(--color-surface-1)]'
                          : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]'
                      }`}
                    >
                      <Icon className="h-4 w-4 text-[var(--color-fg-default)]" />
                    </div>

                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-semibold text-[var(--color-fg-default)] truncate">
                          {resource.title}
                        </span>
                        {isDegraded && (
                          <span className="inline-flex items-center gap-0.5 rounded px-1.5 py-0.2 text-[9px] font-medium text-amber-500 bg-amber-500/10">
                            Degraded
                          </span>
                        )}
                        {isUnavailable && (
                          <span className="inline-flex items-center gap-0.5 rounded px-1.5 py-0.2 text-[9px] font-medium text-zinc-500 bg-zinc-500/10">
                            Unavailable
                          </span>
                        )}
                      </div>
                      <p className="text-[11px] text-[var(--color-fg-muted)] truncate max-w-sm">
                        {resource.description}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 shrink-0 ml-3">
                    {/* Primary Open Button */}
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenResource(resource);
                      }}
                      className={`inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition ${
                        isSelected
                          ? 'bg-[var(--color-fg-default)] text-[var(--color-canvas)] hover:opacity-90'
                          : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-fg-default)]'
                      }`}
                    >
                      <span>Open</span>
                      <CornerDownLeft className="h-3 w-3" />
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Right: Selected Resource Detail, Operations & Contextual Actions */}
        <div className="w-80 shrink-0 overflow-y-auto bg-[var(--color-surface-1)] p-4 flex flex-col justify-between">
          {activeResource ? (
            <div className="space-y-4">
              {/* Header */}
              <div>
                <div className="flex items-center gap-2 mb-1.5">
                  <div className="flex h-7 w-7 items-center justify-center rounded-md border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]">
                    {React.createElement(activeResource.icon, {
                      className: 'h-3.5 w-3.5 text-[var(--color-fg-default)]',
                    })}
                  </div>
                  <div>
                    <h3 className="text-xs font-semibold text-[var(--color-fg-default)]">
                      {activeResource.title}
                    </h3>
                    <div className="flex items-center gap-1.5 text-[10px] text-[var(--color-fg-subtle)]">
                      <span>Group: {activeResource.group}</span>
                      <span>•</span>
                      <span className="font-mono">custos://resource/{activeResource.id}</span>
                    </div>
                  </div>
                </div>
                <p className="mt-2 text-xs leading-relaxed text-[var(--color-fg-muted)]">
                  {activeResource.description}
                </p>
              </div>

              {/* Status Section */}
              <div className="rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] p-2.5 text-xs">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
                    Truthful Status
                  </span>
                  <div className="flex items-center gap-1">
                    {activeResource.status === 'available' ? (
                      <CheckCircle2 className="h-3 w-3 text-emerald-500" />
                    ) : activeResource.status === 'degraded' ? (
                      <AlertTriangle className="h-3 w-3 text-amber-500" />
                    ) : (
                      <XCircle className="h-3 w-3 text-zinc-500" />
                    )}
                    <span
                      className={`text-[10px] font-medium capitalize ${
                        activeResource.status === 'available'
                          ? 'text-emerald-500'
                          : activeResource.status === 'degraded'
                          ? 'text-amber-500'
                          : 'text-zinc-500'
                      }`}
                    >
                      {activeResource.status}
                    </span>
                  </div>
                </div>
                {activeResource.reason ? (
                  <p className="text-[11px] text-[var(--color-fg-muted)]">
                    {activeResource.reason}
                  </p>
                ) : (
                  <p className="text-[11px] text-[var(--color-fg-subtle)]">
                    Ready for operator interaction and agent-directed calls.
                  </p>
                )}
              </div>

              {/* Contextual Actions (from v1.resources.get_actions) */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
                    Preflight Actions
                  </span>
                  {loadingActions && (
                    <span className="text-[10px] text-[var(--color-fg-subtle)]">Loading...</span>
                  )}
                </div>

                {selectedActions.length > 0 ? (
                  <div className="space-y-1.5">
                    {selectedActions.map((action) => (
                      <div
                        key={action.id}
                        className="rounded-md border border-[var(--color-border-subtle)] bg-[var(--color-canvas)] p-2 text-xs"
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-medium text-[var(--color-fg-default)]">
                            {action.title}
                          </span>
                          <span
                            className={`rounded px-1.5 py-0.2 text-[9px] font-mono uppercase ${
                              action.effect === 'destructive'
                                ? 'bg-red-500/10 text-red-400'
                                : action.effect === 'execute'
                                ? 'bg-blue-500/10 text-blue-400'
                                : action.effect === 'write'
                                ? 'bg-amber-500/10 text-amber-400'
                                : 'bg-emerald-500/10 text-emerald-400'
                            }`}
                          >
                            {action.effect}
                          </span>
                        </div>
                        <p className="mt-1 text-[10px] text-[var(--color-fg-muted)]">
                          {action.description}
                        </p>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="rounded border border-dashed border-[var(--color-border-subtle)] p-2.5 text-center text-[10px] text-[var(--color-fg-subtle)]">
                    No contextual preflight actions returned by daemon.
                  </div>
                )}
              </div>

              {/* Supported Operations */}
              {activeResource.supportedOperations && activeResource.supportedOperations.length > 0 && (
                <div>
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] mb-1.5">
                    Registered Operations
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {activeResource.supportedOperations.map((op) => (
                      <code
                        key={op}
                        className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-[var(--color-fg-muted)]"
                      >
                        {op}
                      </code>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="text-center text-xs text-[var(--color-fg-subtle)] my-auto">
              Select a resource to inspect details.
            </div>
          )}

          {/* Action Footer */}
          {activeResource && (
            <div className="mt-6 pt-3 border-t border-[var(--color-border-subtle)]">
              <button
                type="button"
                onClick={() => onOpenResource(activeResource)}
                className="w-full inline-flex items-center justify-center gap-2 rounded-lg bg-[var(--color-fg-default)] text-[var(--color-canvas)] py-2 text-xs font-semibold hover:opacity-90 transition shadow-xs"
              >
                <span>Launch {activeResource.title}</span>
                <ArrowRight className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
