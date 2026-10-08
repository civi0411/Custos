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
} from 'lucide-react';
import { Session } from '@/types';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor } from '@/types/domain';
import { ExecutionWorkspacesPane } from '@/components/workspaces/engineering/ExecutionWorkspacesPane';
import {
  ClaimsMatrixPane,
  DeepInspectorPane,
  LiteraturePane,
  NotebookWorkspacePane,
  ResearchMethodsPane,
  RunsLedgerPane,
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
  | 'notes'
  | 'artifacts'
  | 'knowledge'
  | 'literature'
  | 'claims'
  | 'experiments'
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
  { id: 'files', title: 'Files', description: 'Repository tree, editors and source anchors.', icon: FolderTree, group: 'Engineering', status: 'unavailable', reason: 'Safe Git workspace file/editor/diff API scheduled in Roadmap Step 4.' },
  { id: 'changes', title: 'Changes', description: 'Diff review, annotations and patch decisions.', icon: FileDiff, group: 'Engineering', status: 'unavailable', reason: 'Interactive diff review & patch approval API scheduled in Roadmap Step 4.' },
  { id: 'terminal', title: 'Terminal', description: 'Bounded PTY streams scoped to an execution workspace.', icon: Terminal, group: 'Engineering', status: 'unavailable', reason: 'Bounded PTY streams scoped to an execution workspace scheduled in Roadmap Step 3.' },
  { id: 'worktrees', title: 'Workspaces', description: 'Inspect daemon-owned folder and Git execution workspaces.', icon: GitBranch, group: 'Engineering', status: 'available' },
  { id: 'kanban', title: 'Agents', description: 'Worker runs, attention state and delegated task topology.', icon: Bot, group: 'Engineering', status: 'unavailable', reason: 'Worker runs, attention state and delegated task topology scheduled in Roadmap Step 10.' },
  { id: 'evidence', title: 'Evidence', description: 'Criteria, receipts, verifier records and stale status.', icon: ShieldCheck, group: 'Engineering', status: 'unavailable', reason: 'Criteria verifier records and receipts scheduled in Roadmap Step 8.' },
  { id: 'dag', title: 'Workflow', description: 'Execution graph, dependencies, budgets and run state.', icon: Activity, group: 'Engineering', status: 'available' },
  { id: 'literature', title: 'Sources', description: 'Versioned literature, passages and corpus coverage.', icon: BookOpen, group: 'Research', status: 'available' },
  { id: 'claims', title: 'Claims', description: 'Atomic claims and support, contradiction or unknown evidence.', icon: ShieldCheck, group: 'Research', status: 'available' },
  { id: 'knowledge', title: 'Methods', description: 'Reproduction recipes separated from observed execution records.', icon: FlaskConical, group: 'Research', status: 'available' },
  { id: 'experiments', title: 'Notebook', description: 'Authorized kernels, code cells and reproducible compute.', icon: NotebookTabs, group: 'Research', status: 'unavailable', reason: 'Authorized kernels, code cells and reproducible compute scheduled in Roadmap Step 7.' },
  { id: 'synthesis', title: 'Runs', description: 'Experiment attempts, environments, artifacts and receipts.', icon: FlaskConical, group: 'Research', status: 'available' },
  { id: 'artifacts', title: 'Artifacts', description: 'Inspect versions, lineage, annotations and review findings.', icon: FileCode, group: 'Research', status: 'available' },
  { id: 'browser', title: 'Browser', description: 'Scoped browsing and page capture.', icon: Globe, group: 'Shared', status: 'unavailable', reason: 'Scoped browsing and page capture scheduled in Roadmap Step 10.' },
  { id: 'notes', title: 'Notes', description: 'Task notes and Markdown artifacts.', icon: FileCode, group: 'Shared', status: 'unavailable', reason: 'Task notes and Markdown artifacts scheduled in Roadmap Step 6.' },
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

  React.useEffect(() => {
    let isMounted = true;
    daemonClient.listCapabilities()
      .then((caps) => {
        if (isMounted && Array.isArray(caps)) {
          setCapabilities(caps);
        }
      })
      .catch((err) => {
        console.warn('Failed to load capability registry from daemon:', err);
      });
    return () => {
      isMounted = false;
    };
  }, []);

  const resources = React.useMemo(() => {
    if (!capabilities.length) return DEFAULT_RESOURCES;
    return DEFAULT_RESOURCES.map((def) => {
      const matched = capabilities.find(
        (c) => c.resourceId === def.id || c.id === def.id
      );
      if (!matched) return def;

      const status = matched.status.type;
      const reason = 'reason' in matched.status ? matched.status.reason : undefined;

      return {
        ...def,
        title: matched.title || def.title,
        description: matched.description || def.description,
        status,
        reason,
        supportedOperations: matched.supportedOperations,
      };
    });
  }, [capabilities]);

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
                    <span className={`text-[9px] font-semibold uppercase tracking-[0.08em] ${
                      resource.status === 'available'
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

          <div className="mt-6 flex items-center gap-2 border-t border-[var(--color-border-muted)] pt-4 text-[10px] text-[var(--color-fg-subtle)]">
            <Wrench className="h-3.5 w-3.5" />
            Active session: {session?.title || 'No session selected'}
          </div>
        </div>
      </div>
    );
  }

  if (activeId === 'literature') {
    return <LiteraturePane onShowToast={onShowToast} />;
  }

  if (activeId === 'claims') {
    return (
      <ClaimsMatrixPane
        onHandoffToCoding={(claims) => onHandoffToCoding?.(claims)}
        onShowToast={onShowToast}
      />
    );
  }

  if (activeId === 'knowledge') {
    return <ResearchMethodsPane />;
  }

  if (activeId === 'experiments') {
    return <NotebookWorkspacePane onAskAgent={onAskAgent} onShowToast={onShowToast} />;
  }

  if (activeId === 'worktrees') {
    return <ExecutionWorkspacesPane />;
  }

  if (activeId === 'synthesis') {
    return (
      <RunsLedgerPane
        onReproduce={(run) => onAskAgent?.(`Reproduce run ${run.runId}:\n\`${run.command}\``)}
        onShowToast={onShowToast}
      />
    );
  }

  if (activeId === 'artifacts') {
    return <DeepInspectorPane onShowToast={onShowToast} />;
  }

  return activeResource
    ? <UnavailableResource resource={activeResource} />
    : <UnavailableResource resource={{ id: 'tools', title: 'Unknown resource', description: 'The selected resource is not supported by this client.', icon: Wrench, group: 'Shared', status: 'unavailable', reason: 'The selected resource is not supported by this client.' }} />;
};
