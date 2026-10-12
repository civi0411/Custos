import WorktreeIcon from '~icons/codicon/worktree';
import React from 'react';
import {
  Activity,
  Bot,
  BookOpen,
  FileCode,
  FileDiff,
  FlaskConical,
  FolderTree,
  Globe,
  NotebookTabs,
  ShieldCheck,
  Terminal,
  Wrench,
  Layers,
  Server,
  Smartphone,
  Search,
  Compass,
} from 'lucide-react';
import { BrowserWorkbenchPane } from '@/components/views/BrowserWorkbenchPane';
import { MobileSimulatorWorkbenchPane } from '@/components/views/MobileSimulatorWorkbenchPane';
import { RemoteFleetWorkbenchPane } from '@/components/views/RemoteFleetWorkbenchPane';
import { Session } from '@/types';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor } from '@/types/domain';
import { ExecutionWorkspacesPane } from '@/components/workspaces/engineering/ExecutionWorkspacesPane';
import { TerminalWorkbenchPane } from '@/components/workspaces/engineering/TerminalWorkbenchPane';
import { FilesWorkbenchPane } from '@/components/workspaces/engineering/FilesWorkbenchPane';
import { ChangesWorkbenchPane } from '@/components/workspaces/engineering/ChangesWorkbenchPane';
import { AgentsWorkbenchPane } from '@/components/workspaces/engineering/AgentsWorkbenchPane';
import { EvidenceWorkbenchPane } from '@/components/workspaces/engineering/EvidenceWorkbenchPane';
import { NotesWorkbenchPane } from '@/components/workspaces/NotesWorkbenchPane';
import {
  ClaimsMatrixPane,
  LiteraturePane,
  NotebookWorkspacePane,
  ResearchMethodsPane,
  RunsLedgerPane,
  ArtifactsWorkbenchPane,
  ResearchSynthesisPane,
} from '@/components/research';

export type OrcaTabId =
  | 'tools'
  | 'changes'
  | 'terminal'
  | 'files'
  | 'worktrees'
  | 'kanban'
  | 'evidence'
  | 'dag'
  | 'browser'
  | 'emulator'
  | 'fleet'
  | 'notes'
  | 'artifacts'
  | 'knowledge'
  | 'literature'
  | 'claims'
  | 'experiments'
  | 'runs'
  | 'synthesis';

export interface OrcaTab {
  id: string;
  type?: OrcaTabId;
  title: string;
  url: string;
}

export function getTabType(tab: OrcaTab | { id: string; type?: OrcaTabId }): OrcaTabId {
  if (tab.type) return tab.type;
  const base = tab.id.split('-')[0] as OrcaTabId;
  return base;
}

export interface ResourceTabsState {
  tabs: OrcaTab[];
  activeTabId: string;
  setTabs: React.Dispatch<React.SetStateAction<OrcaTab[]>>;
  setActiveTabId: React.Dispatch<React.SetStateAction<string>>;
}

interface OrcaTabbedContainerProps {
  resourceTabs: ResourceTabsState;
  isPaneOpen: boolean;
  session?: Session | null;
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
  onShowToast?: (msg: string) => void;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  mode?: 'chat' | 'code' | 'research';
  onAskAgent?: (draft: string) => void;
  onHandoffToCoding?: (claims: unknown[]) => void;
  splitPercent?: number;
  onOpenTab?: (id: OrcaTabId) => void;
}

export type ResourceDefinition = {
  id: OrcaTabId;
  title: string;
  description: string;
  icon: React.ElementType;
  group: 'Engineering' | 'Research' | 'Shared';
  status: 'available' | 'degraded' | 'unavailable';
  reason?: string;
  supportedOperations?: string[];
};

export const DEFAULT_RESOURCES: ResourceDefinition[] = [
  { id: 'files', title: 'Files', description: 'Repository tree, editors and source anchors.', icon: FolderTree, group: 'Engineering', status: 'available', supportedOperations: ['tree', 'read', 'write'] },
  { id: 'changes', title: 'Changes', description: 'Diff review, annotations and patch decisions.', icon: FileDiff, group: 'Engineering', status: 'available', supportedOperations: ['diff', 'file_diff', 'stage', 'unstage', 'discard'] },
  { id: 'terminal', title: 'Terminal', description: 'Bounded PTY streams scoped to an execution workspace.', icon: Terminal, group: 'Engineering', status: 'available', supportedOperations: ['spawn', 'write', 'resize', 'read', 'terminate', 'list', 'get'] },
  { id: 'worktrees', title: 'Workspaces', description: 'Inspect daemon-owned folder and Git execution workspaces.', icon: WorktreeIcon, group: 'Engineering', status: 'available' },
  { id: 'kanban', title: 'Agents', description: 'Worker runs, native coding harnesses (Claude Code, Codex, Goose), and agent loop dispatch.', icon: Bot, group: 'Engineering', status: 'available', supportedOperations: ['list_harnesses', 'get_harness', 'run_native', 'dispatch', 'cancel', 'steer'] },
  { id: 'evidence', title: 'Evidence', description: 'Criteria, receipts, verifier records and stale status.', icon: ShieldCheck, group: 'Engineering', status: 'available', supportedOperations: ['reviews.list', 'reviews.record', 'reviews.get', 'reviews.mark_stale'] },
  { id: 'dag', title: 'Workflow', description: 'Execution graph, dependencies, budgets and run state.', icon: Activity, group: 'Engineering', status: 'available' },
  { id: 'literature', title: 'Sources', description: 'Versioned literature, passages and corpus coverage.', icon: BookOpen, group: 'Research', status: 'available' },
  { id: 'claims', title: 'Claims', description: 'Atomic claims and support, contradiction or unknown evidence.', icon: ShieldCheck, group: 'Research', status: 'available' },
  { id: 'knowledge', title: 'Methods', description: 'Reproduction recipes separated from observed execution records.', icon: FlaskConical, group: 'Research', status: 'available' },
  { id: 'experiments', title: 'Notebook', description: 'Authorized kernels, code cells and reproducible compute epochs.', icon: NotebookTabs, group: 'Research', status: 'available', supportedOperations: ['cells.list', 'cells.save', 'execute', 'interrupt', 'reset', 'status'] },
  { id: 'runs', title: 'Runs', description: 'Experiment attempts, environments, artifacts and receipts.', icon: FlaskConical, group: 'Research', status: 'available' },
  { id: 'synthesis', title: 'Synthesis & Handoff', description: 'Synthesize research claims and recipes into Invariant Coding tasks.', icon: Layers, group: 'Research', status: 'available', supportedOperations: ['synthesis.proposals.list', 'synthesis.proposals.get', 'synthesis.proposals.save', 'synthesis.handoff.execute'] },
  { id: 'artifacts', title: 'Artifacts', description: 'Inspect versions, lineage, annotations, provenance DAG and review findings.', icon: FileCode, group: 'Research', status: 'available', supportedOperations: ['list', 'get', 'record_lineage', 'lineage.list', 'lineage.graph', 'annotations.list', 'annotations.save'] },
  { id: 'browser', title: 'Browser', description: 'Persisted browser sessions and tab intents. Physical navigation and snapshots are degraded.', icon: Globe, group: 'Shared', status: 'degraded', reason: 'Browser execution adapter is not connected; navigation and snapshots fail closed.', supportedOperations: ['sessions.list', 'sessions.create', 'tabs.list', 'tabs.create', 'tabs.close'] },
  { id: 'emulator', title: 'Phone Simulator', description: 'Interactive mobile device simulator (iOS, Android, Responsive) for live frontend preview, touch gestures, and DOM inspection.', icon: Smartphone, group: 'Engineering', status: 'available', supportedOperations: ['preview', 'inspect_ax_tree', 'rotate', 'viewport_switch', 'touch_gesture'] },
  { id: 'fleet', title: 'Remote Fleet', description: 'Remote host inventory for coding and research compute. SSH probes and execution are degraded.', icon: Server, group: 'Shared', status: 'degraded', reason: 'SSH transport is not connected; probes and remote execution fail closed.', supportedOperations: ['hosts.list', 'hosts.register', 'jobs.list', 'jobs.create'] },
  { id: 'notes', title: 'Notes', description: 'Task notes, scratchpads, and Markdown knowledge capture with version history.', icon: FileCode, group: 'Shared', status: 'available', supportedOperations: ['list', 'get', 'save', 'history'] },
];

const urlFor = (id: OrcaTabId) => id === 'tools' ? 'custos://resources' : `custos://resource/${id}`;

const UnavailableResource: React.FC<{ resource: ResourceDefinition }> = ({ resource }) => {
  const Icon = resource.icon;
  const isDegraded = resource.status === 'degraded';

  return (
    <div className="flex h-full items-center justify-center bg-[var(--color-canvas)] p-8">
      <div className="max-w-md text-center">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)]">
          <Icon className="h-5 w-5 text-[var(--color-fg-muted)]" />
        </div>
        <div className="mb-2 inline-flex items-center gap-1.5 rounded-full border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2.5 py-0.5 text-[10px] font-medium uppercase tracking-wider">
          <span className={`h-1.5 w-1.5 rounded-full ${isDegraded ? 'bg-amber-400' : 'bg-zinc-500'}`} />
          <span className={isDegraded ? 'text-amber-400' : 'text-[var(--color-fg-muted)]'}>
            {isDegraded ? 'Degraded Capability' : 'Planned Capability'}
          </span>
        </div>
        <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">{resource.title} is currently unavailable</h2>
        <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">{resource.description}</p>

        {resource.reason && (
          <div className="mt-4 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-3 text-left">
            <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">Daemon Rationale</div>
            <div className="mt-1 text-xs leading-relaxed text-[var(--color-fg-default)]">{resource.reason}</div>
          </div>
        )}

        {resource.supportedOperations && resource.supportedOperations.length > 0 && (
          <div className="mt-3 flex flex-wrap justify-center gap-1 text-[10px] text-[var(--color-fg-subtle)]">
            <span>Operations:</span>
            {resource.supportedOperations.map((op) => (
              <code key={op} className="rounded bg-[var(--color-surface-2)] px-1 py-0.5 text-[10px]">{op}</code>
            ))}
          </div>
        )}

        <p className="mt-4 text-[11px] leading-5 text-[var(--color-fg-subtle)]">
          Custos ADE Kernel operates fail-closed. Surfaces without an active daemon API and verifiable event stream
          will never display fabricated worktrees, synthetic terminal sessions, or simulated completions.
        </p>
      </div>
    </div>
  );
};

export const OrcaTabbedContainer: React.FC<OrcaTabbedContainerProps> = ({
  resourceTabs,
  session,
  onShowToast,
  mode = 'code',
  onAskAgent,
  onHandoffToCoding,
  onOpenTab,
}) => {
  const [capabilities, setCapabilities] = React.useState<CapabilityDescriptor[]>([]);
  const [fetchFailed, setFetchFailed] = React.useState(false);
  const [capabilitiesLoaded, setCapabilitiesLoaded] = React.useState(false);

  React.useEffect(() => {
    let isMounted = true;
    daemonClient.listCapabilities()
      .then((caps) => {
        if (isMounted && Array.isArray(caps)) {
          setCapabilities(caps);
          setFetchFailed(false);
          setCapabilitiesLoaded(true);
        }
      })
      .catch((err) => {
        console.warn('Failed to load capability registry from daemon:', err);
        if (isMounted) {
          setFetchFailed(true);
          setCapabilitiesLoaded(true);
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  const resources = React.useMemo(() => {
    if (!capabilitiesLoaded || fetchFailed) {
      return DEFAULT_RESOURCES.map((def) => ({
        ...def,
        status: 'unavailable' as const,
        reason: capabilitiesLoaded
          ? 'Daemon capability registry offline or fetch failed; operating fail-closed.'
          : 'Loading daemon capability registry; operations remain fail-closed.',
        supportedOperations: [],
      }));
    }
    return DEFAULT_RESOURCES.map((def) => {
      const matched = capabilities.find(
        (c) => (c.resourceId ?? c.resource_id) === def.id
      );
      if (!matched) {
        return {
          ...def,
          status: 'unavailable' as const,
          reason: 'No daemon capability descriptor is registered for this resource.',
          supportedOperations: [],
        };
      }

      const status = matched.status.type;
      const reason = 'reason' in matched.status ? matched.status.reason : undefined;

      return {
        ...def,
        title: matched.title || def.title,
        description: matched.description || def.description,
        status,
        reason,
        supportedOperations: matched.supportedOperations ?? matched.supported_operations ?? [],
      };
    });
  }, [capabilities, capabilitiesLoaded, fetchFailed]);

  const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);
  const activeType: OrcaTabId = activeTab ? getTabType(activeTab) : (resourceTabs.activeTabId.split('-')[0] as OrcaTabId);
  const activeResource = resources.find((resource) => resource.id === activeType);

  const [searchQuery, setSearchQuery] = React.useState('');
  const [selectedGroup, setSelectedGroup] = React.useState<string>('All');

  React.useEffect(() => {
    setSelectedGroup('All');
  }, [mode]);

  const openResource = (resource: ResourceDefinition) => {
    const newTabId = `${resource.id}-${Date.now()}`;
    const newTab: OrcaTab = {
      id: newTabId,
      type: resource.id,
      title: resource.title,
      url: urlFor(resource.id),
    };
    resourceTabs.setTabs((previous) => {
      const activeIndex = previous.findIndex((t) => t.id === resourceTabs.activeTabId);
      if (activeIndex !== -1 && getTabType(previous[activeIndex]) === 'tools') {
        const next = [...previous];
        next[activeIndex] = newTab;
        return next;
      }
      return [...previous, newTab];
    });
    resourceTabs.setActiveTabId(newTabId);
  };

  // Scope visible resources strictly to current workbench mode
  const visibleResources = React.useMemo(() => {
    if (mode === 'code') {
      return resources.filter(
        (r) =>
          r.group === 'Engineering' ||
          r.id === 'fleet' ||
          r.id === 'browser' ||
          r.id === 'notes'
      );
    }
    if (mode === 'research') {
      return resources.filter(
        (r) =>
          r.group === 'Research' ||
          r.id === 'fleet' ||
          r.id === 'browser' ||
          r.id === 'notes'
      );
    }
    // Copilot: Assistant tools and shared utilities
    return resources.filter(
      (r) =>
        r.id === 'notes' ||
        r.id === 'browser' ||
        r.id === 'fleet' ||
        r.id === 'evidence' ||
        r.id === 'artifacts'
    );
  }, [resources, mode]);

  const categoryPills = React.useMemo(() => {
    if (mode === 'code') return ['All', 'Engineering', 'Shared'] as const;
    if (mode === 'research') return ['All', 'Research', 'Shared'] as const;
    return ['All', 'Shared'] as const;
  }, [mode]);

  const headerMeta = React.useMemo(() => {
    if (mode === 'code') {
      return {
        kicker: 'Coding Workbench',
        title: 'Engineering Tools & Resources',
        description: 'Repository inspection, execution surfaces, and developer tools for this session.',
      };
    }
    if (mode === 'research') {
      return {
        kicker: 'Research Workbench',
        title: 'Research Tools & Resources',
        description: 'Literature, hypotheses, notebook kernels, and synthesis tools for this session.',
      };
    }
    return {
      kicker: 'Copilot Workbench',
      title: 'Copilot Tools & Resources',
      description: 'Assistant tools, scratchpads, and inspection surfaces for this session.',
    };
  }, [mode]);

  const filteredResources = visibleResources.filter((resource) => {
    const isShared = resource.group === 'Shared' || resource.id === 'fleet' || resource.id === 'browser' || resource.id === 'notes' || resource.id === 'evidence' || resource.id === 'artifacts';
    const matchesGroup =
      selectedGroup === 'All' ||
      (selectedGroup === 'Engineering' && resource.group === 'Engineering') ||
      (selectedGroup === 'Research' && resource.group === 'Research') ||
      (selectedGroup === 'Shared' && isShared);
    const q = searchQuery.trim().toLowerCase();
    const matchesQuery =
      !q ||
      resource.title.toLowerCase().includes(q) ||
      resource.description.toLowerCase().includes(q) ||
      (resource.supportedOperations && resource.supportedOperations.some((op) => op.toLowerCase().includes(q)));
    return matchesGroup && matchesQuery;
  });

  if (activeType === 'tools') {
    return (
      <div className="h-full overflow-y-auto bg-[var(--color-canvas)] px-6 py-8 text-[var(--color-editor-fg)]">
        <div className="mx-auto max-w-4xl">
          {/* Hero Header */}
          <div className="mb-6">
            <div className="flex items-center gap-2 mb-2">
              <span className="workbench-kicker">{headerMeta.kicker}</span>
              <span className="text-[10px] text-fg-subtle">•</span>
              <span className="text-[11px] text-fg-subtle flex items-center gap-1">
                <Compass className="w-3.5 h-3.5 text-accent" /> New Tab
              </span>
            </div>
            <h1 className="text-xl font-semibold tracking-[-0.02em] text-fg-editor">
              {headerMeta.title}
            </h1>
            <p className="mt-1 text-xs leading-5 text-[var(--color-fg-muted)]">
              {headerMeta.description}
            </p>

            {/* Quick Search & Filters */}
            <div className="mt-4 flex flex-col sm:flex-row items-stretch sm:items-center gap-3">
              <div className="relative flex-1">
                <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-fg-subtle pointer-events-none" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Filter resources by name, description, or capability..."
                  className="w-full bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl pl-10 pr-4 py-2 text-xs text-fg-editor placeholder:text-fg-subtle focus:outline-none focus:border-border-default focus:ring-1 focus:ring-border-default/20 transition"
                  autoFocus
                />
              </div>

              {/* Category Pills */}
              <div className="flex items-center gap-1 bg-[var(--color-surface-1)] p-1 rounded-xl border border-[var(--color-border-muted)] shrink-0 text-xs">
                {categoryPills.map((group) => (
                  <button
                    key={group}
                    type="button"
                    onClick={() => setSelectedGroup(group)}
                    className={`px-2.5 py-1 rounded-lg text-[11px] font-medium transition ${
                      selectedGroup === group
                        ? 'bg-surface-2 text-fg-editor shadow-xs'
                        : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2/50'
                    }`}
                  >
                    {group}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Cards Grid */}
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {filteredResources.map((resource) => {
              const Icon = resource.icon;
              return (
                <button
                  key={resource.id}
                  type="button"
                  onClick={() => openResource(resource)}
                  className="group rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 text-left transition hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)] hover:shadow-sm"
                >
                  <div className="flex items-start justify-between">
                    <div className="p-2 rounded-lg bg-[var(--color-canvas)] border border-[var(--color-border-muted)] group-hover:border-[var(--color-border-default)] transition">
                      <Icon className="h-4 w-4 text-[var(--color-fg-muted)] group-hover:workbench-accent transition-colors" />
                    </div>
                    <span
                      className={`text-[9px] font-semibold uppercase tracking-[0.08em] px-1.5 py-0.5 rounded ${
                        resource.status === 'available'
                          ? 'text-emerald-500 bg-emerald-500/10'
                          : resource.status === 'degraded'
                          ? 'text-amber-500 bg-amber-500/10'
                          : 'text-[var(--color-fg-subtle)] bg-surface-2'
                      }`}
                    >
                      {resource.status === 'available' ? 'API' : resource.status === 'degraded' ? 'Degraded' : 'Planned'}
                    </span>
                  </div>
                  <div className="mt-4 text-xs font-semibold text-fg-editor group-hover:workbench-accent transition-colors">
                    {resource.title}
                  </div>
                  <p className="mt-1 text-[11px] leading-5 text-[var(--color-fg-muted)] line-clamp-2">
                    {resource.description}
                  </p>

                  {resource.supportedOperations && resource.supportedOperations.length > 0 && (
                    <div className="mt-3 flex flex-wrap gap-1 pt-2 border-t border-[var(--color-border-muted)]/60">
                      {resource.supportedOperations.slice(0, 3).map((op) => (
                        <span key={op} className="text-[9.5px] font-mono px-1 py-0.2 rounded bg-[var(--color-canvas)] text-fg-subtle border border-[var(--color-border-muted)]">
                          {op}
                        </span>
                      ))}
                      {resource.supportedOperations.length > 3 && (
                        <span className="text-[9.5px] font-mono px-1 text-fg-subtle">
                          +{resource.supportedOperations.length - 3}
                        </span>
                      )}
                    </div>
                  )}
                </button>
              );
            })}
          </div>

          {filteredResources.length === 0 && (
            <div className="p-8 text-center text-xs text-fg-subtle border border-dashed border-[var(--color-border-muted)] rounded-xl mt-4">
              No matching resources found for "{searchQuery}".
            </div>
          )}

          <div className="mt-8 flex items-center justify-between border-t border-[var(--color-border-muted)] pt-4 text-[10px] text-[var(--color-fg-subtle)]">
            <div className="flex items-center gap-2">
              <Wrench className="h-3.5 w-3.5" />
              <span>Active session: {session?.title || 'No session selected'}</span>
            </div>
            <span>Click any card to open in this tab</span>
          </div>
        </div>
      </div>
    );
  }

  if (!activeResource || activeResource.status === 'unavailable') {
    return activeResource
      ? <UnavailableResource resource={activeResource} />
      : <UnavailableResource resource={{ id: 'tools', title: 'Unknown resource', description: 'The selected resource is not supported by this client.', icon: Wrench, group: 'Shared', status: 'unavailable', reason: 'The selected resource is not supported by this client.' }} />;
  }

  const frameResource = (content: React.ReactNode) => (
    <div className="flex h-full min-h-0 flex-col bg-[var(--color-canvas)]">
      <div className="flex shrink-0 items-center gap-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-3 py-1.5 text-[10px] text-[var(--color-fg-muted)]">
        <span className="font-semibold text-[var(--color-fg-default)]">{activeResource.title}</span>
        <span className={`rounded-full px-1.5 py-0.5 font-semibold uppercase tracking-[0.06em] ${activeResource.status === 'available'
            ? 'bg-emerald-500/10 text-emerald-500'
            : 'bg-amber-500/10 text-amber-500'
          }`}>
          {activeResource.status}
        </span>
        <span className="truncate">Session: {session?.id || 'unbound'}</span>
        <span className="ml-auto truncate font-mono">
          {(activeResource.supportedOperations ?? []).length > 0
            ? (activeResource.supportedOperations ?? []).join(' · ')
            : 'No operations advertised'}
        </span>
      </div>
      {activeResource.reason && activeResource.status === 'degraded' && (
        <div className="shrink-0 border-b border-amber-500/20 bg-amber-500/5 px-3 py-1 text-[10px] text-amber-600 dark:text-amber-300">
          {activeResource.reason}
        </div>
      )}
      <div className="min-h-0 flex-1">{content}</div>
    </div>
  );

  if (activeType === 'browser') {
    return frameResource(
      <BrowserWorkbenchPane
        onShowToast={onShowToast}
        supportedOperations={activeResource.supportedOperations ?? []}
        capabilityReason={activeResource.reason}
      />
    );
  }

  if (activeType === 'emulator') {
    return frameResource(<MobileSimulatorWorkbenchPane onShowToast={onShowToast} />);
  }

  if (activeType === 'fleet') {
    return frameResource(
      <RemoteFleetWorkbenchPane
        onShowToast={onShowToast}
        supportedOperations={activeResource.supportedOperations ?? []}
        capabilityReason={activeResource.reason}
      />
    );
  }

  if (activeType === 'literature') {
    return frameResource(<LiteraturePane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />);
  }

  if (activeType === 'claims') {
    return frameResource(
      <ClaimsMatrixPane
        onHandoffToCoding={(claims) => onHandoffToCoding?.(claims)}
        onShowToast={onShowToast}
      />
    );
  }

  if (activeType === 'knowledge') {
    return frameResource(<ResearchMethodsPane />);
  }

  if (activeType === 'experiments') {
    return frameResource(<NotebookWorkspacePane sessionId={session?.id} onAskAgent={onAskAgent} onShowToast={onShowToast} onOpenTab={onOpenTab} />);
  }

  if (activeType === 'worktrees') {
    return frameResource(<ExecutionWorkspacesPane />);
  }

  if (activeType === 'terminal') {
    return frameResource(<TerminalWorkbenchPane />);
  }

  if (activeType === 'files') {
    return frameResource(<FilesWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />);
  }

  if (activeType === 'changes') {
    return frameResource(<ChangesWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />);
  }

  if (activeType === 'kanban') {
    return frameResource(<AgentsWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />);
  }

  if (activeType === 'evidence') {
    return frameResource(<EvidenceWorkbenchPane onShowToast={onShowToast} />);
  }

  if (activeType === 'runs') {
    return frameResource(
      <RunsLedgerPane
        sessionId={session?.id}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (activeType === 'synthesis') {
    return frameResource(<ResearchSynthesisPane onShowToast={onShowToast} />);
  }

  if (activeType === 'artifacts') {
    return frameResource(<ArtifactsWorkbenchPane onShowToast={onShowToast} />);
  }

  if (activeType === 'notes') {
    return frameResource(<NotesWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} />);
  }

  return activeResource
    ? <UnavailableResource resource={activeResource} />
    : <UnavailableResource resource={{ id: 'tools', title: 'Unknown resource', description: 'The selected resource is not supported by this client.', icon: Wrench, group: 'Shared', status: 'unavailable', reason: 'The selected resource is not supported by this client.' }} />;
};
