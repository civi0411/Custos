import WorktreeIcon from '~icons/codicon/worktree';
import React from 'react';
import {
  Activity,
  BookOpen,
  Bot,
  FileCode,
  FileText,
  FlaskConical,
  FolderTree,
  Globe,
  ShieldCheck,
  Terminal,
  Wrench,
  Layers,
  Server,
  Smartphone,
  ArrowRight,
  Send,
  Users,
  Calendar,
} from 'lucide-react';

import {
  BrowserWorkbenchPane,
  MobileSimulatorWorkbenchPane,
  RemoteFleetWorkbenchPane,
  NotesWorkbenchPane,
} from './shared';
import { Session } from '@/types';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor } from '@/types/domain';

// Engineering Consolidated Panes
import {
  FilesChangesPane,
  ActivityPane,
  WorktreeManagerPane,
  TerminalWorkbenchPane,
  AgentsWorkbenchPane,
} from './engineering';

// Research Consolidated Panes
import {
  LiteraturePane,
  ResearchMethodsPane,
  ArtifactsWorkbenchPane,
  ResearchSynthesisPane,
  FindingsPane,
  LabPane,
} from './research';

// Copilot Consolidated Panes
import {
  DraftsPane,
  OutboxPane,
  ContactsCalendarPane,
} from './copilot';

export type OrcaTabId =
  | 'tools'
  // Coding
  | 'files'
  | 'activity'
  | 'terminal'
  | 'worktrees'
  | 'agents'
  | 'browser'
  | 'emulator'
  | 'fleet'
  // Research
  | 'literature'
  | 'findings'
  | 'lab'
  | 'knowledge'
  | 'artifacts'
  | 'synthesis'
  // Copilot
  | 'drafts'
  | 'outbox'
  | 'contacts'
  | 'calendar'
  | 'notes'
  // Backward compatibility aliases
  | 'changes'
  | 'kanban'
  | 'evidence'
  | 'dag'
  | 'claims'
  | 'experiments'
  | 'runs';

export interface OrcaTab {
  id: string;
  type?: OrcaTabId;
  title: string;
  url: string;
  agentId?: string;
  workspaceId?: string;
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
  mode?: 'chat' | 'code' | 'research' | 'engineering' | 'copilot';
  onAskAgent?: (draft: string) => void;
  onHandoffToCoding?: (claims: unknown[]) => void;
  splitPercent?: number;
  onOpenTab?: (id: OrcaTabId) => void;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  activeWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  workspaces?: import('@/types/domain').ExecutionWorkspace[];
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
  // Coding Workbench
  {
    id: 'files',
    title: 'Files',
    description: 'Project files & git diffs.',
    icon: FolderTree,
    group: 'Engineering',
    status: 'available',
    supportedOperations: ['tree', 'read', 'write', 'diff', 'stage', 'discard'],
  },
  {
    id: 'activity',
    title: 'Activity',
    description: 'Agent runs, DAG, and tests.',
    icon: Activity,
    group: 'Engineering',
    status: 'available',
    supportedOperations: ['list_harnesses', 'dispatch', 'cancel', 'steer', 'reviews.list'],
  },
  {
    id: 'terminal',
    title: 'Terminal',
    description: 'Command shell.',
    icon: Terminal,
    group: 'Engineering',
    status: 'available',
    supportedOperations: ['spawn', 'write', 'resize', 'read', 'terminate'],
  },
  {
    id: 'worktrees',
    title: 'Worktree Manager',
    description: 'Isolated Git workspaces, comparison, and integration.',
    icon: WorktreeIcon,
    group: 'Engineering',
    status: 'available',
    supportedOperations: ['list', 'dirty', 'steer', 'recover'],
  },
  {
    id: 'browser',
    title: 'Browser Preview',
    description: 'Web application preview.',
    icon: Globe,
    group: 'Shared',
    status: 'available',
  },
  {
    id: 'agents',
    title: 'Coding Agents',
    description: 'Start a native coding harness in a selected execution workspace.',
    icon: Bot,
    group: 'Engineering',
    status: 'available',
  },
  {
    id: 'emulator',
    title: 'Phone Simulator',
    description: 'Mobile viewport simulator with authentic hardware chassis.',
    icon: Smartphone,
    group: 'Shared',
    status: 'available',
  },

  // Research Workbench
  {
    id: 'literature',
    title: 'Sources',
    description: 'Papers & literature citations.',
    icon: BookOpen,
    group: 'Research',
    status: 'available',
  },
  {
    id: 'findings',
    title: 'Findings',
    description: 'Claims & inline evidence.',
    icon: ShieldCheck,
    group: 'Research',
    status: 'available',
  },
  {
    id: 'lab',
    title: 'Lab',
    description: 'Notebook cells & run ledger.',
    icon: FlaskConical,
    group: 'Research',
    status: 'available',
    supportedOperations: ['execute', 'interrupt', 'reset', 'runs.list'],
  },
  {
    id: 'knowledge',
    title: 'Methods',
    description: 'Reproduction protocols.',
    icon: Layers,
    group: 'Research',
    status: 'available',
  },
  {
    id: 'artifacts',
    title: 'Artifacts',
    description: 'Generated plots & figures.',
    icon: FileCode,
    group: 'Research',
    status: 'available',
  },
  {
    id: 'synthesis',
    title: 'Handoff',
    description: 'Research-to-coding synthesis.',
    icon: ArrowRight,
    group: 'Research',
    status: 'available',
    supportedOperations: ['synthesis.handoff.execute'],
  },

  // Copilot Workbench
  {
    id: 'drafts',
    title: 'Drafts',
    description: 'Communication & document drafts.',
    icon: FileText,
    group: 'Shared',
    status: 'available',
    supportedOperations: ['drafts.list', 'drafts.save', 'drafts.delete'],
  },
  {
    id: 'outbox',
    title: 'Outbox',
    description: 'Recorded external actions and receipts.',
    icon: Send,
    group: 'Shared',
    status: 'available',
    supportedOperations: ['outbox.list', 'outbox.approve', 'outbox.dispatch'],
  },
  {
    id: 'contacts',
    title: 'Contacts',
    description: 'Verified contact roster and communication endpoints.',
    icon: Users,
    group: 'Shared',
    status: 'available',
  },
  {
    id: 'calendar',
    title: 'Calendar',
    description: 'Scheduled milestones, deadlines, and task calendar.',
    icon: Calendar,
    group: 'Shared',
    status: 'available',
  },
  {
    id: 'notes',
    title: 'Notes',
    description: 'Session scratchpad & notes.',
    icon: FileCode,
    group: 'Shared',
    status: 'available',
  },
  {
    id: 'fleet',
    title: 'Remote Fleet',
    description: 'Distributed compute hosts.',
    icon: Server,
    group: 'Shared',
    status: 'available',
    supportedOperations: ['hosts.list', 'hosts.register', 'hosts.ping', 'fleet.exec'],
  },
];

/** The daemon is authoritative for executable resources; native previews are host-scoped. */
export function resolveResources(
  capabilities: CapabilityDescriptor[],
  loaded: boolean,
  failed: boolean,
  _nativeHost?: boolean
): ResourceDefinition[] {
  const aliases: Partial<Record<OrcaTabId, string>> = {
    activity: 'kanban',
    findings: 'claims',
    lab: 'runs',
  };

  return DEFAULT_RESOURCES.map((def) => {
    if (def.id === 'browser' || def.id === 'emulator') {
      return {
        ...def,
        status: 'available',
        supportedOperations: [],
      };
    }
    if (def.id === 'contacts' || def.id === 'calendar') {
      return { ...def, supportedOperations: [] };
    }
    if (!loaded || failed) {
      return {
        ...def,
        status: 'unavailable',
        reason: failed ? 'Cannot reach the Custos daemon.' : 'Checking daemon capabilities.',
        supportedOperations: [],
      };
    }
    const matched = capabilities.find(
      (capability) => (capability.resourceId ?? capability.resource_id) === (aliases[def.id] ?? def.id)
    );
    if (!matched) {
      return {
        ...def,
        status: 'unavailable',
        reason: 'The daemon does not advertise this resource.',
        supportedOperations: [],
      };
    }
    if (def.id === 'lab' && matched.status.type !== 'unavailable') {
      const notebook = capabilities.find((capability) => (capability.resourceId ?? capability.resource_id) === 'experiments');
      if (!notebook || notebook.status.type !== 'available') {
        return {
          ...def,
          status: 'degraded',
          reason: 'Run records are readable, but notebook execution is not available from this daemon.',
          supportedOperations: ['runs.list'],
        };
      }
      return {
        ...def,
        status: matched.status.type,
        supportedOperations: ['runs.list', ...(notebook.supportedOperations ?? notebook.supported_operations ?? [])],
      };
    }
    return {
      ...def,
      status: matched.status.type,
      reason: 'reason' in matched.status ? matched.status.reason : undefined,
      supportedOperations: matched.supportedOperations ?? matched.supported_operations ?? [],
    };
  });
}

const urlFor = (id: OrcaTabId) =>
  id === 'tools' ? 'custos://resources' : `custos://resource/${id}`;

const UnavailableResource: React.FC<{ resource: ResourceDefinition; onRetry?: () => void }> = ({ resource, onRetry }) => {
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
            {isDegraded ? 'Limited' : 'Unavailable'}
          </span>
        </div>
        <h2 className="text-sm font-semibold text-[var(--color-editor-fg)]">{resource.title} is currently unavailable</h2>
        <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">{resource.description}</p>

        {resource.reason && (
          <div className="mt-4 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-3 text-left">
            <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">Reason</div>
            <div className="mt-1 text-xs leading-relaxed text-[var(--color-fg-default)]">{resource.reason}</div>
          </div>
        )}
        {onRetry && (
          <button type="button" onClick={onRetry} className="mt-4 rounded-md border border-[var(--color-border-default)] px-3 py-1.5 text-xs hover:bg-[var(--color-surface-2)]">
            Retry connection
          </button>
        )}

      </div>
    </div>
  );
};

const ResourceContent: React.FC<OrcaTabbedContainerProps> = ({
  resourceTabs,
  session,
  onShowToast,
  mode = 'code',
  onAskAgent,
  onHandoffToCoding,
  onOpenTab,
  activeWorkspaceId,
  onSelectWorkspace,
  activeWorkspace: _activeWorkspace,
  workspaces: _workspaces,
}) => {
  const [capabilities, setCapabilities] = React.useState<CapabilityDescriptor[]>([]);
  const [fetchFailed, setFetchFailed] = React.useState(false);
  const [capabilitiesLoaded, setCapabilitiesLoaded] = React.useState(false);
  const [capabilityAttempt, setCapabilityAttempt] = React.useState(0);
  const [initialAgentWorkspaceId, setInitialAgentWorkspaceId] = React.useState<string | undefined>();
  const [initialFilesWorkspaceId, setInitialFilesWorkspaceId] = React.useState<string | undefined>();
  const [initialTerminalWorkspaceId, setInitialTerminalWorkspaceId] = React.useState<string | undefined>();

  React.useEffect(() => {
    let isMounted = true;
    let timeoutId: ReturnType<typeof setTimeout> | undefined;
    setCapabilitiesLoaded(false);
    setFetchFailed(false);
    Promise.race([
      daemonClient.listCapabilities(),
      new Promise<CapabilityDescriptor[]>((_, reject) => {
        timeoutId = setTimeout(() => reject(new Error('Capability check timed out')), 10000);
      }),
    ])
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
      }).finally(() => { if (timeoutId) clearTimeout(timeoutId); });

    return () => {
      isMounted = false;
    };
  }, [capabilityAttempt]);

  const resources = React.useMemo(
    () => resolveResources(capabilities, capabilitiesLoaded, fetchFailed, '__TAURI_INTERNALS__' in window),
    [capabilities, capabilitiesLoaded, fetchFailed]
  );

  const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);
  const activeType: OrcaTabId = activeTab ? getTabType(activeTab) : (resourceTabs.activeTabId.split('-')[0] as OrcaTabId);
  const activeResource = resources.find((resource) => resource.id === activeType);

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

  // Scope visible resources to current workbench mode + shared surfaces (browser, emulator)
  const visibleResources = React.useMemo(() => {
    if (mode === 'code' || mode === 'engineering') {
      return resources.filter((r) =>
        ['files', 'activity', 'terminal', 'worktrees', 'agents', 'browser', 'emulator'].includes(r.id)
      );
    }
    if (mode === 'research') {
      return resources.filter((r) =>
        ['literature', 'findings', 'lab', 'knowledge', 'artifacts', 'synthesis', 'browser', 'emulator'].includes(r.id)
      );
    }
    // Copilot: drafts, outbox, contacts, calendar, notes + shared browser, emulator
    return resources.filter((r) =>
      ['drafts', 'outbox', 'contacts', 'calendar', 'notes', 'browser', 'emulator'].includes(r.id)
    );
  }, [resources, mode]);

  // New Tab Surface Picker
  if (activeType === 'tools') {
    return (
      <div className="h-full overflow-y-auto bg-[var(--color-canvas)] p-4 text-[var(--color-editor-fg)]">
        <div className="mx-auto max-w-md">
          <h2 className="mb-3 text-xs font-semibold">Resources</h2>
          <div className="space-y-1">
            {visibleResources.map((resource) => {
              const Icon = resource.icon;
              return (
                <button
                  key={resource.id}
                  type="button"
                  onClick={() => openResource(resource)}
                  className="flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left hover:bg-[var(--color-surface-2)]"
                  title={resource.reason || resource.description}
                >
                  <Icon className="h-4 w-4 shrink-0 text-[var(--color-fg-muted)]" />
                  <span className="min-w-0 flex-1 truncate text-xs font-medium">{resource.title}</span>
                  {resource.status !== 'available' && <span className="text-[10px] text-[var(--color-fg-muted)]">{resource.status === 'degraded' ? 'Limited' : 'Unavailable'}</span>}
                </button>
              );
            })}
          </div>
        </div>
      </div>
    );
  }

  if (activeResource?.status === 'unavailable') {
    return <UnavailableResource resource={activeResource} onRetry={() => setCapabilityAttempt((attempt) => attempt + 1)} />;
  }

  const frameResource = (content: React.ReactNode) => (
    <div className="flex h-full min-h-0 flex-col bg-[var(--color-canvas)]">
      {activeResource?.reason && activeResource.status === 'degraded' && (
        <div className="shrink-0 border-b border-amber-500/20 bg-amber-500/5 px-3 py-1.5 text-[10px] text-amber-600 dark:text-amber-300">
          {activeResource.reason}
        </div>
      )}
      <div className="min-h-0 flex-1">{content}</div>
    </div>
  );

  // Coding Mappings
  if (activeType === 'files' || activeType === 'changes') {
    return frameResource(
      <FilesChangesPane
        sessionId={session?.id}
        initialWorkspaceId={activeWorkspaceId || initialFilesWorkspaceId}
        activeWorkspaceId={activeWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (activeType === 'activity' || activeType === 'kanban' || activeType === 'evidence' || activeType === 'dag') {
    return frameResource(
      <ActivityPane
        sessionId={session?.id}
        onShowToast={onShowToast}
        onAskAgent={onAskAgent}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (activeType === 'worktrees') {
    return frameResource(
      <WorktreeManagerPane
        activeWorkspaceId={activeWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        onShowToast={onShowToast}
        onOpenWorkspaceResource={(resource, workspaceId) => {
          onSelectWorkspace?.(workspaceId);
          if (resource === 'agents') setInitialAgentWorkspaceId(workspaceId);
          if (resource === 'files') setInitialFilesWorkspaceId(workspaceId);
          if (resource === 'terminal') setInitialTerminalWorkspaceId(workspaceId);
          onOpenTab?.(resource);
        }}
      />
    );
  }

  if (activeType === 'agents') {
    return frameResource(
      <AgentsWorkbenchPane
        key={activeTab?.id}
        tabId={activeTab?.id}
        initialHarnessId={activeTab?.agentId}
        sessionId={session?.id}
        initialWorkspaceId={activeTab?.workspaceId || activeWorkspaceId || initialAgentWorkspaceId}
        activeWorkspaceId={activeWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (activeType === 'terminal') {
    return frameResource(
      <TerminalWorkbenchPane
        initialWorkspaceId={activeWorkspaceId || initialTerminalWorkspaceId}
        activeWorkspaceId={activeWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
      />
    );
  }

  if (activeType === 'fleet') {
    return frameResource(
      <RemoteFleetWorkbenchPane
        onShowToast={onShowToast}
        supportedOperations={activeResource?.supportedOperations ?? []}
        capabilityReason={activeResource?.reason}
      />
    );
  }

  // Research Mappings
  if (activeType === 'literature') {
    return frameResource(
      <LiteraturePane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (activeType === 'findings' || activeType === 'claims') {
    return frameResource(
      <FindingsPane
        sessionId={session?.id}
        onHandoffToCoding={(claims) => onHandoffToCoding?.(claims)}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (activeType === 'lab' || activeType === 'experiments' || activeType === 'runs') {
    return frameResource(
      <LabPane
        sessionId={session?.id}
        onAskAgent={onAskAgent}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (activeType === 'knowledge') {
    return frameResource(<ResearchMethodsPane />);
  }

  if (activeType === 'synthesis') {
    return frameResource(<ResearchSynthesisPane onShowToast={onShowToast} />);
  }

  if (activeType === 'artifacts') {
    return frameResource(
      <ArtifactsWorkbenchPane onShowToast={onShowToast} onAskAgent={onAskAgent} />
    );
  }

  // Copilot Mappings
  if (activeType === 'drafts') {
    return frameResource(
      <DraftsPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (activeType === 'outbox') {
    return frameResource(
      <OutboxPane onShowToast={onShowToast} />
    );
  }

  if (activeType === 'contacts') {
    return frameResource(
      <ContactsCalendarPane type="contacts" onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (activeType === 'calendar') {
    return frameResource(
      <ContactsCalendarPane type="calendar" onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (activeType === 'notes') {
    return frameResource(
      <NotesWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} onAskAgent={onAskAgent} />
    );
  }

  return activeResource
    ? <UnavailableResource resource={activeResource} />
    : <UnavailableResource resource={{ id: 'tools', title: 'Unknown resource', description: 'The selected resource is not supported by this client.', icon: Wrench, group: 'Shared', status: 'unavailable', reason: 'The selected resource is not supported by this client.' }} />;
};

// Retain only browser tabs that have actually been visited. Hidden native
// webviews preserve page state; removing a tab or panel runs their cleanup.
export const OrcaTabbedContainer = React.memo(function OrcaTabbedContainer(props: OrcaTabbedContainerProps) {
  const { resourceTabs } = props;
  const activeTab = resourceTabs.tabs.find((tab) => tab.id === resourceTabs.activeTabId);
  const browserActive = !!activeTab && ['browser', 'emulator'].includes(getTabType(activeTab));
  const [visited, setVisited] = React.useState<Set<string>>(() => new Set());

  React.useEffect(() => {
    setVisited((previous) => {
      const next = new Set([...previous].filter((id) => resourceTabs.tabs.some((tab) => tab.id === id)));
      if (browserActive && activeTab) next.add(activeTab.id);
      if (next.size === previous.size && [...next].every((id) => previous.has(id))) return previous;
      return next;
    });
  }, [resourceTabs.tabs, browserActive, activeTab]);

  return (
    <div className="relative h-full min-h-0">
      <div className="h-full" style={{ display: browserActive ? 'none' : 'block' }}>
        <ResourceContent {...props} />
      </div>
      {resourceTabs.tabs
        .filter((tab) => ['browser', 'emulator'].includes(getTabType(tab)) && (visited.has(tab.id) || tab.id === resourceTabs.activeTabId))
        .map((tab) => (
          <div key={tab.id} className="h-full" style={{ display: tab.id === resourceTabs.activeTabId ? 'block' : 'none' }}>
            <React.Suspense
              fallback={
                <div role="status" aria-label="Loading browser" className="h-full p-8 bg-canvas space-y-5 motion-safe:animate-pulse">
                  <div className="h-7 w-2/3 bg-surface-2 rounded" />
                  <div className="h-32 bg-surface-1 rounded" />
                </div>
              }
            >
              {getTabType(tab) === 'emulator' ? (
                <MobileSimulatorWorkbenchPane
                  tabId={tab.id}
                  active={tab.id === resourceTabs.activeTabId}
                  url={tab.url.startsWith('custos:') ? 'about:blank' : tab.url}
                  onUrlChange={(url) =>
                    resourceTabs.setTabs((previous) =>
                      previous.map((item) => (item.id === tab.id && item.url !== url ? { ...item, url } : item))
                    )
                  }
                  onAskAgent={props.onAskAgent}
                />
              ) : (
                <BrowserWorkbenchPane
                  tabId={tab.id}
                  active={tab.id === resourceTabs.activeTabId}
                  url={tab.url.startsWith('custos:') ? 'about:blank' : tab.url}
                  onUrlChange={(url) =>
                    resourceTabs.setTabs((previous) =>
                      previous.map((item) => (item.id === tab.id && item.url !== url ? { ...item, url } : item))
                    )
                  }
                  onAskAgent={props.onAskAgent}
                />
              )}
            </React.Suspense>
          </div>
        ))}
    </div>
  );
});
