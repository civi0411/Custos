import React from 'react';
import {
  Activity,
  Bot,
  BookOpen,
  FileCode,
  FileDiff,
  FlaskConical,
  FolderTree,
  GitBranch,
  Globe,
  NotebookTabs,
  ShieldCheck,
  Terminal,
  Wrench,
  Layers,
  Server,
  Smartphone,
  Play,
} from 'lucide-react';
import { BrowserWorkbenchPane } from '@/components/views/BrowserWorkbenchPane';
import { MobileSimulatorWorkbenchPane } from '@/components/views/MobileSimulatorWorkbenchPane';
import { RemoteFleetWorkbenchPane } from '@/components/views/RemoteFleetWorkbenchPane';
import { Session } from '@/types';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor, PacksJobCatalogItem } from '@/types/domain';
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
  id: OrcaTabId;
  title: string;
  url: string;
}

export interface ResourceTabsState {
  tabs: OrcaTab[];
  activeTabId: OrcaTabId;
  setTabs: React.Dispatch<React.SetStateAction<OrcaTab[]>>;
  setActiveTabId: React.Dispatch<React.SetStateAction<OrcaTabId>>;
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

const DEFAULT_RESOURCES: ResourceDefinition[] = [
  { id: 'files', title: 'Files', description: 'Repository tree, editors and source anchors.', icon: FolderTree, group: 'Engineering', status: 'available', supportedOperations: ['tree', 'read', 'write'] },
  { id: 'changes', title: 'Changes', description: 'Diff review, annotations and patch decisions.', icon: FileDiff, group: 'Engineering', status: 'available', supportedOperations: ['diff', 'file_diff', 'stage', 'unstage', 'discard'] },
  { id: 'terminal', title: 'Terminal', description: 'Bounded PTY streams scoped to an execution workspace.', icon: Terminal, group: 'Engineering', status: 'available', supportedOperations: ['spawn', 'write', 'resize', 'read', 'terminate', 'list', 'get'] },
  { id: 'worktrees', title: 'Workspaces', description: 'Inspect daemon-owned folder and Git execution workspaces.', icon: GitBranch, group: 'Engineering', status: 'available' },
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
  { id: 'fleet', title: 'Remote Fleet', description: 'Persisted SSH host inventory. Remote probes and execution are degraded.', icon: Server, group: 'Engineering', status: 'degraded', reason: 'SSH transport is not connected; probes and remote execution fail closed.', supportedOperations: ['hosts.list', 'hosts.register', 'jobs.list', 'jobs.create'] },
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
}) => {
  const [capabilities, setCapabilities] = React.useState<CapabilityDescriptor[]>([]);
  const [fetchFailed, setFetchFailed] = React.useState(false);
  const [capabilitiesLoaded, setCapabilitiesLoaded] = React.useState(false);
  const [packsJobs, setPacksJobs] = React.useState<PacksJobCatalogItem[]>([]);
  const [runningJobId, setRunningJobId] = React.useState<string | null>(null);

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

    daemonClient.listPacksJobs()
      .then((jobs) => {
        if (isMounted && Array.isArray(jobs)) {
          setPacksJobs(jobs);
        }
      })
      .catch((err) => {
        console.warn('Failed to load workbench jobs from daemon:', err);
      });

    return () => {
      isMounted = false;
    };
  }, []);

  const handleRunJob = async (job: PacksJobCatalogItem) => {
    if (job.status === 'planned') {
      onShowToast?.(`Job "${job.name}" is planned for a future release.`);
      return;
    }
    const id = job.jobId || job.job_id || '';
    setRunningJobId(id);
    try {
      const res = await daemonClient.runPacksJob({
        jobId: id,
        sessionId: session?.id || session?.sessionId,
      });
      onShowToast?.(`Workbench Job "${job.name}" started (Task ${res.taskId || res.task_id})`);
    } catch (err) {
      onShowToast?.(`Failed to start job: ${String(err)}`);
    } finally {
      setRunningJobId(null);
    }
  };

  const filteredJobs = React.useMemo(() => {
    const targetLens = mode === 'code' ? 'coding' : mode === 'research' ? 'research' : 'assistant';
    return packsJobs.filter((j) => j.lens === targetLens);
  }, [packsJobs, mode]);

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

  const activeId = resourceTabs.activeTabId;
  const activeResource = resources.find((resource) => resource.id === activeId);

  const openResource = (resource: ResourceDefinition) => {
    resourceTabs.setTabs((previous) =>
      previous.some((tab) => tab.id === resource.id)
        ? previous
        : [...previous, { id: resource.id, title: resource.title, url: urlFor(resource.id) }]
    );
    resourceTabs.setActiveTabId(resource.id);
  };

  const visibleResources = resources.filter((resource) => {
    if (mode === 'research') return resource.group !== 'Engineering';
    if (mode === 'code') return resource.group !== 'Research';
    return resource.group === 'Shared' || resource.id === 'evidence' || resource.id === 'literature';
  });

  if (activeId === 'tools') {
    return (
      <div className="h-full overflow-y-auto bg-[var(--color-canvas)] px-6 py-7 text-[var(--color-editor-fg)]">
        <div className="mx-auto max-w-4xl">
          <div className="mb-6">
            <div className="workbench-kicker">{mode} workbench</div>
            <h1 className="mt-1 text-lg font-semibold tracking-[-0.02em]">Resources</h1>
            <p className="mt-1 max-w-xl text-xs leading-5 text-[var(--color-fg-muted)]">
              Open a resource beside the same conversation. Tabs preserve the session; they do not
              create a run, grant authority, or imply that an operation succeeded.
            </p>
          </div>

          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {visibleResources.map((resource) => {
              const Icon = resource.icon;
              return (
                <button
                  key={resource.id}
                  type="button"
                  onClick={() => openResource(resource)}
                  className="group rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 text-left transition hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)]"
                >
                  <div className="flex items-start justify-between">
                    <Icon className="h-4 w-4 text-[var(--color-fg-muted)] group-hover:workbench-accent" />
                    <span className={`text-[9px] font-semibold uppercase tracking-[0.08em] ${resource.status === 'available'
                        ? 'text-emerald-500'
                        : resource.status === 'degraded'
                          ? 'text-amber-500'
                          : 'text-[var(--color-fg-subtle)]'
                      }`}>
                      {resource.status === 'available' ? 'API' : resource.status === 'degraded' ? 'Degraded' : 'Planned'}
                    </span>
                  </div>
                  <div className="mt-5 text-xs font-semibold">{resource.title}</div>
                  <p className="mt-1.5 text-[11px] leading-5 text-[var(--color-fg-muted)]">{resource.description}</p>
                </button>
              );
            })}
          </div>

          <div className="mt-10 border-t border-[var(--color-border-muted)] pt-8">
            <div className="flex items-center justify-between mb-4">
              <div>
                <div className="flex items-center gap-2">
                  <Bot className="h-4 w-4 text-[var(--color-fg-muted)]" />
                  <h2 className="text-sm font-semibold tracking-[-0.01em]">Workbench Jobs</h2>
                  <span className="rounded-full bg-[var(--color-surface-2)] px-2 py-0.5 text-[10px] font-mono text-[var(--color-fg-muted)]">
                    {filteredJobs.length}
                  </span>
                </div>
                <p className="mt-1 text-[11px] text-[var(--color-fg-muted)]">
                  Pre-configured autonomous multi-role jobs tailored for the {mode} workbench.
                </p>
              </div>
            </div>

            {filteredJobs.length === 0 ? (
              <div className="rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-6 text-center text-xs text-[var(--color-fg-muted)]">
                No jobs configured for the {mode} workbench.
              </div>
            ) : (
              <div className="grid gap-3 md:grid-cols-2">
                {filteredJobs.map((job) => {
                  const id = job.jobId || job.job_id || '';
                  const role = job.primaryRole || job.primary_role || 'Agent';
                  const topology = job.recommendedTopology || job.recommended_topology || 'Single';
                  const isRunning = runningJobId === id;
                  const isPlanned = job.status === 'planned';
                  const isDegraded = job.status === 'degraded';
                  const reason = job.degradationReason || job.degradation_reason;

                  return (
                    <div
                      key={id}
                      className="group flex flex-col justify-between rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 transition hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)]"
                    >
                      <div>
                        <div className="flex items-start justify-between gap-2">
                          <div className="font-semibold text-xs text-[var(--color-fg-default)]">
                            {job.name}
                          </div>
                          <span
                            className={`shrink-0 rounded-full px-2 py-0.5 text-[9px] font-semibold uppercase tracking-[0.08em] ${
                              job.status === 'available'
                                ? 'bg-emerald-500/10 text-emerald-500'
                                : isDegraded
                                ? 'bg-amber-500/10 text-amber-500'
                                : 'bg-zinc-500/10 text-[var(--color-fg-subtle)]'
                            }`}
                          >
                            {job.status}
                          </span>
                        </div>

                        <p className="mt-1.5 text-[11px] leading-5 text-[var(--color-fg-muted)]">
                          {job.description}
                        </p>

                        {isDegraded && reason && (
                          <div className="mt-2 rounded-md bg-amber-500/10 border border-amber-500/20 px-2 py-1 text-[10px] text-amber-600 dark:text-amber-400">
                            {reason}
                          </div>
                        )}

                        <div className="mt-3 flex flex-wrap items-center gap-2 text-[10px] text-[var(--color-fg-subtle)] font-mono">
                          <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5">
                            Role: {role}
                          </span>
                          <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5">
                            Topology: {topology}
                          </span>
                        </div>
                      </div>

                      <div className="mt-4 flex items-center justify-between border-t border-[var(--color-border-subtle)] pt-3">
                        <span className="text-[10px] text-[var(--color-fg-subtle)] font-mono truncate max-w-[150px]">
                          {id}
                        </span>
                        <button
                          type="button"
                          disabled={isPlanned || isRunning}
                          onClick={() => handleRunJob(job)}
                          className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-medium transition ${
                            isPlanned
                              ? 'cursor-not-allowed opacity-50 bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                              : isRunning
                              ? 'opacity-70 bg-indigo-500/20 text-indigo-400'
                              : 'bg-indigo-600 hover:bg-indigo-500 text-white shadow-sm'
                          }`}
                        >
                          <Play className="h-3 w-3 fill-current" />
                          {isRunning ? 'Starting...' : isPlanned ? 'Planned' : 'Run Job'}
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          <div className="mt-6 flex items-center gap-2 border-t border-[var(--color-border-muted)] pt-4 text-[10px] text-[var(--color-fg-subtle)]">
            <Wrench className="h-3.5 w-3.5" />
            Active session: {session?.title || 'No session selected'}
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

  if (activeId === 'browser') {
    return frameResource(
      <BrowserWorkbenchPane
        onShowToast={onShowToast}
        supportedOperations={activeResource.supportedOperations ?? []}
        capabilityReason={activeResource.reason}
      />
    );
  }

  if (activeId === 'emulator') {
    return frameResource(<MobileSimulatorWorkbenchPane onShowToast={onShowToast} />);
  }

  if (activeId === 'fleet') {
    return frameResource(
      <RemoteFleetWorkbenchPane
        onShowToast={onShowToast}
        supportedOperations={activeResource.supportedOperations ?? []}
        capabilityReason={activeResource.reason}
      />
    );
  }

  if (activeId === 'literature') {
    return frameResource(<LiteraturePane onShowToast={onShowToast} />);
  }

  if (activeId === 'claims') {
    return frameResource(
      <ClaimsMatrixPane
        onHandoffToCoding={(claims) => onHandoffToCoding?.(claims)}
        onShowToast={onShowToast}
      />
    );
  }

  if (activeId === 'knowledge') {
    return frameResource(<ResearchMethodsPane />);
  }

  if (activeId === 'experiments') {
    return frameResource(<NotebookWorkspacePane sessionId={session?.id} onAskAgent={onAskAgent} onShowToast={onShowToast} />);
  }

  if (activeId === 'worktrees') {
    return frameResource(<ExecutionWorkspacesPane />);
  }

  if (activeId === 'terminal') {
    return frameResource(<TerminalWorkbenchPane />);
  }

  if (activeId === 'files') {
    return frameResource(<FilesWorkbenchPane />);
  }

  if (activeId === 'changes') {
    return frameResource(<ChangesWorkbenchPane />);
  }

  if (activeId === 'kanban') {
    return frameResource(<AgentsWorkbenchPane />);
  }

  if (activeId === 'evidence') {
    return frameResource(<EvidenceWorkbenchPane onShowToast={onShowToast} />);
  }

  if (activeId === 'runs') {
    return frameResource(
      <RunsLedgerPane
        onReproduce={(run) => onAskAgent?.(`Reproduce run ${run.runId}:\n\`${run.command}\``)}
        onShowToast={onShowToast}
      />
    );
  }

  if (activeId === 'synthesis') {
    return frameResource(<ResearchSynthesisPane onShowToast={onShowToast} />);
  }

  if (activeId === 'artifacts') {
    return frameResource(<ArtifactsWorkbenchPane onShowToast={onShowToast} />);
  }

  if (activeId === 'notes') {
    return frameResource(<NotesWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} />);
  }

  return activeResource
    ? <UnavailableResource resource={activeResource} />
    : <UnavailableResource resource={{ id: 'tools', title: 'Unknown resource', description: 'The selected resource is not supported by this client.', icon: Wrench, group: 'Shared', status: 'unavailable', reason: 'The selected resource is not supported by this client.' }} />;
};
