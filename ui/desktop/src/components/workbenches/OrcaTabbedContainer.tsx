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
  ArrowRightLeft,
  Columns2,
  X,
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
  splitTabId?: string | null;
  setSplitTabId?: React.Dispatch<React.SetStateAction<string | null>>;
  splitRatio?: number;
  setSplitRatio?: React.Dispatch<React.SetStateAction<number>>;
  toggleSplitTab?: (tabId: string) => void;
  closeSplitTab?: () => void;
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
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
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
  nativeHost = false
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
        status: nativeHost ? 'available' : 'unavailable',
        reason: nativeHost ? undefined : 'Native browser preview requires the Custos desktop host.',
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

  const renderTab = (tab: OrcaTab | undefined) => {
    if (!tab) return null;
    const tabType = getTabType(tab);
    const tabResource = resources.find((resource) => resource.id === tabType);

    if (tabType === 'tools') {
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
                    {resource.status !== 'available' && (
                      <span className="text-[10px] text-[var(--color-fg-muted)]">
                        {resource.status === 'degraded' ? 'Limited' : 'Unavailable'}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      );
    }

    if (tabResource?.status === 'unavailable') {
      return <UnavailableResource resource={tabResource} onRetry={() => setCapabilityAttempt((attempt) => attempt + 1)} />;
    }

    const frameTabResource = (content: React.ReactNode) => (
      <div className="flex h-full min-h-0 flex-col bg-[var(--color-canvas)]">
        {tabResource?.reason && tabResource.status === 'degraded' && (
          <div className="shrink-0 border-b border-amber-500/20 bg-amber-500/5 px-3 py-1.5 text-[10px] text-amber-600 dark:text-amber-300">
            {tabResource.reason}
          </div>
        )}
        <div className="min-h-0 flex-1">{content}</div>
      </div>
    );

    // Coding Mappings
    if (tabType === 'files' || tabType === 'changes') {
      return frameTabResource(
        <FilesChangesPane
          sessionId={session?.id}
          initialWorkspaceId={tab.workspaceId || activeWorkspaceId || initialFilesWorkspaceId}
          activeWorkspaceId={activeWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          onShowToast={onShowToast}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'activity' || tabType === 'kanban' || tabType === 'evidence' || tabType === 'dag') {
      return frameTabResource(
        <ActivityPane
          sessionId={session?.id}
          onShowToast={onShowToast}
          onAskAgent={onAskAgent}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'worktrees') {
      return frameTabResource(
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

    if (tabType === 'agents') {
      return frameTabResource(
        <AgentsWorkbenchPane
          key={tab.id}
          tabId={tab.id}
          initialHarnessId={tab.agentId}
          sessionId={session?.id}
          initialWorkspaceId={tab.workspaceId || activeWorkspaceId || initialAgentWorkspaceId}
          activeWorkspaceId={activeWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          onShowToast={onShowToast}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'terminal') {
      return frameTabResource(
        <TerminalWorkbenchPane
          initialWorkspaceId={tab.workspaceId || activeWorkspaceId || initialTerminalWorkspaceId}
          activeWorkspaceId={activeWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
        />
      );
    }

    if (tabType === 'fleet') {
      return frameTabResource(
        <RemoteFleetWorkbenchPane
          onShowToast={onShowToast}
          supportedOperations={tabResource?.supportedOperations ?? []}
          capabilityReason={tabResource?.reason}
        />
      );
    }

    // Research Mappings
    if (tabType === 'literature') {
      return frameTabResource(
        <LiteraturePane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
      );
    }

    if (tabType === 'findings' || tabType === 'claims') {
      return frameTabResource(
        <FindingsPane
          sessionId={session?.id}
          onHandoffToCoding={(claims) => onHandoffToCoding?.(claims)}
          onShowToast={onShowToast}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'lab' || tabType === 'experiments' || tabType === 'runs') {
      return frameTabResource(
        <LabPane
          sessionId={session?.id}
          onAskAgent={onAskAgent}
          onShowToast={onShowToast}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'knowledge') {
      return frameTabResource(<ResearchMethodsPane />);
    }

    if (tabType === 'synthesis') {
      return frameTabResource(<ResearchSynthesisPane onShowToast={onShowToast} />);
    }

    if (tabType === 'artifacts') {
      return frameTabResource(
        <ArtifactsWorkbenchPane onShowToast={onShowToast} onAskAgent={onAskAgent} />
      );
    }

    // Copilot Mappings
    if (tabType === 'drafts') {
      return frameTabResource(
        <DraftsPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
      );
    }

    if (tabType === 'outbox') {
      return frameTabResource(
        <OutboxPane onShowToast={onShowToast} />
      );
    }

    if (tabType === 'contacts') {
      return frameTabResource(
        <ContactsCalendarPane type="contacts" onShowToast={onShowToast} onOpenTab={onOpenTab} />
      );
    }

    if (tabType === 'calendar') {
      return frameTabResource(
        <ContactsCalendarPane type="calendar" onShowToast={onShowToast} onOpenTab={onOpenTab} />
      );
    }

    if (tabType === 'notes') {
      return frameTabResource(
        <NotesWorkbenchPane sessionId={session?.id} onShowToast={onShowToast} onAskAgent={onAskAgent} />
      );
    }

    // Shared Browser & Emulator
    if (tabType === 'browser') {
      return (
        <React.Suspense
          fallback={
            <div role="status" aria-label="Loading browser" className="h-full p-8 bg-canvas space-y-5 motion-safe:animate-pulse">
              <div className="h-7 w-2/3 bg-surface-2 rounded" />
              <div className="h-32 bg-surface-1 rounded" />
            </div>
          }
        >
          <BrowserWorkbenchPane
            tabId={tab.id}
            active={true}
            url={tab.url.startsWith('custos:') ? 'about:blank' : tab.url}
            onUrlChange={(url) =>
              resourceTabs.setTabs((previous) =>
                previous.map((item) => (item.id === tab.id && item.url !== url ? { ...item, url } : item))
              )
            }
            onAskAgent={onAskAgent}
          />
        </React.Suspense>
      );
    }

    if (tabType === 'emulator') {
      return (
        <React.Suspense
          fallback={
            <div role="status" aria-label="Loading emulator" className="h-full p-8 bg-canvas space-y-5 motion-safe:animate-pulse">
              <div className="h-7 w-2/3 bg-surface-2 rounded" />
              <div className="h-32 bg-surface-1 rounded" />
            </div>
          }
        >
          <MobileSimulatorWorkbenchPane
            tabId={tab.id}
            active={true}
            url={tab.url.startsWith('custos:') ? 'about:blank' : tab.url}
            onUrlChange={(url) =>
              resourceTabs.setTabs((previous) =>
                previous.map((item) => (item.id === tab.id && item.url !== url ? { ...item, url } : item))
              )
            }
            onAskAgent={onAskAgent}
          />
        </React.Suspense>
      );
    }

    return tabResource
      ? <UnavailableResource resource={tabResource} />
      : <UnavailableResource resource={{ id: 'tools', title: 'Unknown resource', description: 'The selected resource is not supported by this client.', icon: Wrench, group: 'Shared', status: 'unavailable', reason: 'The selected resource is not supported by this client.' }} />;
  };

  const isSplitActive = Boolean(
    resourceTabs.splitTabId &&
    resourceTabs.splitTabId !== resourceTabs.activeTabId &&
    resourceTabs.tabs.some((t) => t.id === resourceTabs.splitTabId)
  );

  const splitTab = isSplitActive
    ? resourceTabs.tabs.find((t) => t.id === resourceTabs.splitTabId)
    : undefined;

  const splitRatio = resourceTabs.splitRatio ?? 50;
  const setSplitRatio = resourceTabs.setSplitRatio;
  const splitContainerRef = React.useRef<HTMLDivElement>(null);
  const isDraggingSplit = React.useRef(false);

  React.useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDraggingSplit.current || !splitContainerRef.current || !setSplitRatio) return;
      const rect = splitContainerRef.current.getBoundingClientRect();
      const newRatio = ((e.clientX - rect.left) / rect.width) * 100;
      if (newRatio >= 20 && newRatio <= 80) {
        setSplitRatio(Math.round(newRatio));
      }
    };
    const handleMouseUp = () => {
      isDraggingSplit.current = false;
    };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [setSplitRatio]);

  if (isSplitActive && splitTab) {
    return (
      <div ref={splitContainerRef} className="flex h-full w-full min-h-0 min-w-0 flex-row overflow-hidden relative">
        {/* Left Pane (Primary Active Tab) */}
        <div
          className="h-full min-h-0 min-w-0 overflow-hidden flex flex-col"
          style={{ width: `${splitRatio}%` }}
        >
          {renderTab(activeTab)}
        </div>

        {/* Draggable Inner Splitter */}
        <div
          className="w-1.5 h-full bg-white/5 hover:bg-workbench-accent cursor-col-resize shrink-0 z-30 transition-colors flex items-center justify-center group select-none relative"
          onMouseDown={() => {
            isDraggingSplit.current = true;
          }}
          onDoubleClick={() => setSplitRatio?.(50)}
          title="Kéo để chỉnh tỷ lệ song song (Nhấn đúp về 50/50)"
        >
          <div className="w-0.5 h-8 rounded-full bg-white/20 group-hover:bg-white transition-colors" />
        </div>

        {/* Right Pane (Secondary Split Tab) */}
        <div
          className="h-full min-h-0 min-w-0 overflow-hidden flex flex-col border-l border-white/10"
          style={{ width: `${100 - splitRatio}%` }}
        >
          {/* Sub-Header for Parallel Split Pane */}
          <div className="h-7 px-2.5 bg-[#18181b] border-b border-white/10 flex items-center justify-between shrink-0 select-none">
            <div className="flex items-center gap-1.5 min-w-0">
              <span className="px-1 py-0.2 rounded text-[8.5px] font-mono bg-purple-500/20 text-purple-300 font-semibold border border-purple-500/30">
                2
              </span>
              <span className="text-[11px] font-medium text-white/90 truncate">
                {splitTab.title}
              </span>
            </div>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => {
                  const currentActive = resourceTabs.activeTabId;
                  resourceTabs.setActiveTabId(splitTab.id);
                  resourceTabs.setSplitTabId?.(currentActive);
                }}
                className="p-1 rounded text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
                title="Đổi vị trí 2 tab (Swap panes)"
              >
                <ArrowRightLeft className="size-3" />
              </button>
              <button
                type="button"
                onClick={() => resourceTabs.closeSplitTab?.()}
                className="p-1 rounded text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
                title="Đóng chế độ song song (Close split view)"
              >
                <X className="size-3" />
              </button>
            </div>
          </div>

          <div className="flex-1 min-h-0 overflow-hidden">
            {renderTab(splitTab)}
          </div>
        </div>
      </div>
    );
  }

  // Single tab view
  return renderTab(activeTab);
};

// Retain only browser tabs that have actually been visited. Hidden native
// webviews preserve page state; removing a tab or panel runs their cleanup.
export const OrcaTabbedContainer = React.memo(function OrcaTabbedContainer(props: OrcaTabbedContainerProps) {
  const { resourceTabs } = props;
  const isSplitActive = Boolean(
    resourceTabs.splitTabId &&
    resourceTabs.splitTabId !== resourceTabs.activeTabId &&
    resourceTabs.tabs.some((t) => t.id === resourceTabs.splitTabId)
  );
  const activeTab = resourceTabs.tabs.find((tab) => tab.id === resourceTabs.activeTabId);
  const browserActive = !isSplitActive && !!activeTab && ['browser', 'emulator'].includes(getTabType(activeTab));
  const [visited, setVisited] = React.useState<Set<string>>(() => new Set());
  const [isDragOverSplit, setIsDragOverSplit] = React.useState(false);
  const containerRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    setVisited((previous) => {
      const next = new Set([...previous].filter((id) => resourceTabs.tabs.some((tab) => tab.id === id)));
      if (browserActive && activeTab) next.add(activeTab.id);
      if (next.size === previous.size && [...next].every((id) => previous.has(id))) return previous;
      return next;
    });
  }, [resourceTabs.tabs, browserActive, activeTab]);

  const handleDragOver = (e: React.DragEvent) => {
    const hasTab = e.dataTransfer.types.includes('application/custos-tab-id') || e.dataTransfer.types.includes('text/plain');
    if (!hasTab) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    if (!containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    const relX = e.clientX - rect.left;
    if (relX > rect.width * 0.3) {
      setIsDragOverSplit(true);
    } else {
      setIsDragOverSplit(false);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    if (!containerRef.current || !containerRef.current.contains(e.relatedTarget as Node)) {
      setIsDragOverSplit(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    setIsDragOverSplit(false);
    const droppedTabId = e.dataTransfer.getData('application/custos-tab-id') || e.dataTransfer.getData('text/plain');
    if (!droppedTabId) return;
    if (droppedTabId !== resourceTabs.activeTabId) {
      resourceTabs.setSplitTabId?.(droppedTabId);
      props.onShowToast?.(`Đã mở song song tab "${resourceTabs.tabs.find((t) => t.id === droppedTabId)?.title || droppedTabId}"`);
    } else if (resourceTabs.tabs.length > 1) {
      const otherTab = resourceTabs.tabs.find((t) => t.id !== droppedTabId);
      if (otherTab) {
        resourceTabs.setSplitTabId?.(droppedTabId);
        resourceTabs.setActiveTabId(otherTab.id);
        props.onShowToast?.(`Đã mở song song tab "${droppedTabId}"`);
      }
    }
  };

  return (
    <div
      ref={containerRef}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className="relative h-full min-h-0"
    >
      {/* Drop Zone Indicator for Split View */}
      {isDragOverSplit && (
        <div className="absolute inset-y-1 right-1 w-[48%] z-50 pointer-events-none flex flex-col items-center justify-center bg-blue-600/15 backdrop-blur-xs border-2 border-dashed border-blue-400 rounded-xl m-1 transition-all duration-200">
          <div className="flex flex-col items-center gap-2 p-4 rounded-xl bg-[#1c1c20]/95 border border-white/20 shadow-2xl text-center">
            <Columns2 className="size-8 text-blue-400 animate-pulse" />
            <span className="text-xs font-semibold text-white">Thả vào đây để xem song song</span>
            <span className="text-[10px] text-white/60">Chia đôi màn hình với tab đang mở (Split View)</span>
          </div>
        </div>
      )}

      <div className="h-full" style={{ display: browserActive ? 'none' : 'block' }}>
        <ResourceContent {...props} />
      </div>
      {!isSplitActive && resourceTabs.tabs
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
