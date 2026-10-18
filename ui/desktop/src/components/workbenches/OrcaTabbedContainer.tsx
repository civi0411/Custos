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
  Columns2,
  X,
  Send,
  Users,
  Calendar,
  Plus,
  MessageSquare,
  CheckSquare,
  Maximize2,
  Minimize2,
  MoreHorizontal,
} from 'lucide-react';
import { canvasGroups, canvasReducer, createCanvas, reconcileCanvas, restoreCanvas, type CanvasAction, type CanvasNode, type CanvasState, type CanvasGroup } from '@/types/canvas';
import { ConversationPane } from '@/components/chat/ConversationPane';
import { TasksPane } from '@/components/tasks/TasksPane';
import type { ResourceRef } from '@/types/canvas';
import type { ActiveWorkbench } from '@/components/shell/AppHeader';
import { useScreenContext } from '@/context/ScreenContext';
import { ActionIcon } from '@/components/common';

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
  | 'unsupported'
  | 'tools'
  // Core Conversation & Peer Chat
  | 'conversation'
  | 'chat'
  // Tasks & Timeline
  | 'tasks'
  | 'task'
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
  sessionId?: string;
  taskId?: string;
  runId?: string;
  sourceId?: string;
  artifactId?: string;
  revision?: string;
  ref?: ResourceRef;
}

export function getTabType(tab: OrcaTab | { id: string; type?: OrcaTabId }): OrcaTabId {
  const kind = tab.type || tab.id;
  return [...DEFAULT_RESOURCES.map(resource => resource.id), 'tools', 'chat', 'task', 'changes', 'kanban', 'evidence', 'dag', 'claims', 'experiments', 'runs'].includes(kind)
    ? kind as OrcaTabId : 'unsupported';
}

/** A resource URL may prepare a draft, never execute a handoff. */
export function synthesisClaimFromUrl(value: string): string | undefined {
  try {
    const url = new URL(value);
    if (url.protocol !== 'custos:' || url.hostname !== 'synthesis') return undefined;
    const id = url.searchParams.get('claim');
    return id && id.length <= 128 && !/[\s\x00-\x1f]/.test(id) ? id : undefined;
  } catch { return undefined; }
}

/** Resource identity is scoped; tab IDs are presentation identities only. */
export function findScopedResourceTab(tabs: OrcaTab[], type: OrcaTabId, scope: Pick<OrcaTab, 'workspaceId' | 'sessionId' | 'taskId' | 'agentId'>): OrcaTab | undefined {
  return tabs.find(tab => getTabType(tab) === type
    && tab.workspaceId === scope.workspaceId && tab.sessionId === scope.sessionId
    && tab.taskId === scope.taskId && tab.agentId === scope.agentId);
}

export interface ResourceTabsState {
  scopeKey?: string;
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
  isResourceActive?: boolean;
  catalog?: { resources: ResourceDefinition[]; retry: () => void };
  session?: Session | null;
  onSendMessage?: (text: string, sessionId?: string) => Promise<boolean> | void;
  boundTask?: import('@/types/domain').Task | null;
  boundWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  taskAttentionCount?: number;
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
  onShowToast?: (msg: string) => void;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  mode?: 'chat' | 'code' | 'research' | 'engineering' | 'copilot';
  onAskAgent?: (draft: string, sessionId?: string) => void;
  onHandoffToCoding?: (claims: unknown[], result: import('@/types/domain').HandoffToCodingResult) => void;
  splitPercent?: number;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string, targetPane?: 'primary' | 'split', targetWorkspaceId?: string) => void;
  onOpenResourcePicker?: (targetPane?: 'primary' | 'split', anchor?: { top: number; left: number }) => void;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  activeWorkspace?: import('@/types/domain').ExecutionWorkspace | null;
  workspaces?: import('@/types/domain').ExecutionWorkspace[];
  tasks?: import('@/types/domain').Task[];
  allSessions?: Session[];
  selectedTaskId?: string | null;
  onSelectTask?: (taskId: string | null) => void;
  onOpenSession?: (sessionId: string) => void;
  onNewSessionForTask?: (taskId: string) => void;
  onCreateTask?: (params: import('@/types/domain').CreateTaskParams) => Promise<import('@/types/domain').Task>;
  onAdvanceStatus?: (taskId: string, status: import('@/types/domain').TaskStatus) => Promise<void>;
  onCompleteTask?: (taskId: string, summary?: string) => Promise<void>;
  onCancelTask?: (taskId: string, reason?: string) => Promise<void>;
  onRefreshTasks?: () => Promise<import('@/types/domain').Task[]>;
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
  // Core Conversation
  {
    id: 'conversation',
    title: 'Chat / Agent Session',
    description: 'Peer conversation, task steering and thought stream.',
    icon: MessageSquare,
    group: 'Shared',
    status: 'available',
  },
  {
    id: 'tasks',
    title: 'Tasks & Timeline',
    description: 'Long-term task orchestration, criteria and outcomes.',
    icon: CheckSquare,
    group: 'Shared',
    status: 'available',
  },
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

export function workbenchResourceIds(mode: string, includeShared = true): OrcaTabId[] {
  const shared: OrcaTabId[] = includeShared ? ['conversation', 'tasks', 'notes', 'browser', 'emulator'] : [];
  return [...(mode === 'code' || mode === 'engineering'
    ? ['files', 'activity', 'terminal', 'worktrees', 'agents']
    : mode === 'research' ? ['literature', 'findings', 'lab', 'knowledge', 'artifacts', 'synthesis']
      : ['drafts', 'outbox', 'contacts', 'calendar']) as OrcaTabId[], ...shared];
}

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
    if (def.id === 'contacts' || def.id === 'calendar' || def.id === 'conversation' || def.id === 'notes') {
      return { ...def, status: 'available', supportedOperations: [] };
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

const getTabIcon = (type: OrcaTabId, active: boolean) => {
  const cls = `size-3.5 shrink-0 ${active ? 'text-white' : 'text-white/40'}`;
  switch (type) {
    case 'conversation':
    case 'chat': return <MessageSquare className={cls} />;
    case 'tasks':
    case 'task': return <CheckSquare className={cls} />;
    case 'files': return <FolderTree className={cls} />;
    case 'activity': return <Activity className={cls} />;
    case 'terminal': return <Terminal className={cls} />;
    case 'worktrees': return <WorktreeIcon className={cls} />;
    case 'agents': return <Bot className={cls} />;
    case 'browser': return <Globe className={cls} />;
    case 'emulator': return <Smartphone className={cls} />;
    case 'fleet': return <Server className={cls} />;
    case 'literature': return <BookOpen className={cls} />;
    case 'findings': return <ShieldCheck className={cls} />;
    case 'lab': return <FlaskConical className={cls} />;
    case 'synthesis': return <Layers className={cls} />;
    case 'artifacts': return <FileCode className={cls} />;
    case 'drafts': return <FileText className={cls} />;
    case 'outbox': return <Send className={cls} />;
    case 'contacts': return <Users className={cls} />;
    case 'calendar': return <Calendar className={cls} />;
    default: return <Globe className={cls} />;
  }
};

function useResourceCatalog(provided?: { resources: ResourceDefinition[]; retry: () => void }) {
  const [capabilities, setCapabilities] = React.useState<CapabilityDescriptor[]>([]);
  const [fetchFailed, setFetchFailed] = React.useState(false);
  const [capabilitiesLoaded, setCapabilitiesLoaded] = React.useState(false);
  const [capabilityAttempt, setCapabilityAttempt] = React.useState(0);

  React.useEffect(() => {
    if (provided) return;
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
  }, [capabilityAttempt, provided]);

  const resources = React.useMemo(
    () => resolveResources(capabilities, capabilitiesLoaded, fetchFailed, '__TAURI_INTERNALS__' in window),
    [capabilities, capabilitiesLoaded, fetchFailed]
  );

  return provided || { resources, retry: () => setCapabilityAttempt(attempt => attempt + 1) };
}

export const ResourceContent: React.FC<OrcaTabbedContainerProps> = ({
  resourceTabs,
  catalog,
  isResourceActive = true,
  session,
  onSendMessage,
  boundTask,
  boundWorkspace,
  taskAttentionCount,
  onShowToast,
  mode = 'code',
  onAskAgent,
  onHandoffToCoding,
  onOpenTab,
  activeWorkspaceId,
  onSelectWorkspace,
  activeWorkspace: _activeWorkspace,
  workspaces: _workspaces,
  tasks,
  allSessions,
  selectedTaskId,
  onSelectTask,
  onOpenSession,
  onNewSessionForTask,
  onCreateTask,
  onAdvanceStatus,
  onCompleteTask,
  onCancelTask,
  onRefreshTasks,
}) => {
  const { resources, retry } = useResourceCatalog(catalog);
  const [initialAgentWorkspaceId, setInitialAgentWorkspaceId] = React.useState<string | undefined>();
  const [initialFilesWorkspaceId, setInitialFilesWorkspaceId] = React.useState<string | undefined>();
  const [initialTerminalWorkspaceId, setInitialTerminalWorkspaceId] = React.useState<string | undefined>();

  const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);

  const openResource = (resource: ResourceDefinition) => {
    if (onOpenTab) { onOpenTab(resource.id, urlFor(resource.id), resource.title); return; }
    const existing = findScopedResourceTab(resourceTabs.tabs, resource.id, {
      workspaceId: activeWorkspaceId, sessionId: session?.id, taskId: selectedTaskId || session?.taskId,
    });
    if (existing) { resourceTabs.setActiveTabId(existing.id); return; }
    const newTabId = `${resource.id}-${crypto.randomUUID()}`;
    const newTab: OrcaTab = {
      id: newTabId,
      type: resource.id,
      title: resource.title,
      url: urlFor(resource.id),
      workspaceId: activeWorkspaceId,
      sessionId: session?.id,
      taskId: selectedTaskId || session?.taskId,
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

  const visibleResources = React.useMemo(() => resources.filter(resource => workbenchResourceIds(mode).includes(resource.id)), [resources, mode]);

  const renderTab = (tab: OrcaTab | undefined) => {
    if (!tab) return null;
    const tabType = getTabType(tab);
    const tabResource = resources.find((resource) => resource.id === tabType);
    const needsWorkspace = ['files', 'changes', 'terminal', 'agents'].includes(tabType);
    const needsSession = ['conversation', 'chat', 'literature', 'findings', 'claims', 'lab', 'experiments', 'runs', 'drafts'].includes(tabType);
    const scopeReason = needsWorkspace && !tab.workspaceId
      ? 'Select a workspace, then open this resource from its workbench.'
      : needsWorkspace && _workspaces?.length && !_workspaces.some(workspace => workspace.id === tab.workspaceId)
        ? 'This workspace is no longer registered. Close this tab and select an available workspace.'
        : needsSession && !session?.id ? 'Select or create a conversation to open this resource.' : null;
    if (scopeReason) return <div className="h-full flex flex-col items-center justify-center p-8 gap-3 text-center text-fg-muted"><Layers size={26} /><h3 className="text-sm text-fg-default">Choose a scope</h3><p className="text-xs max-w-xs leading-relaxed">{scopeReason}</p></div>;


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
      return <UnavailableResource resource={tabResource} onRetry={retry} />;
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

    // Core Conversation Peer Mapping
    if (tabType === 'conversation' || tabType === 'chat') {
      const tabSession = (tab.sessionId && allSessions?.find((s) => s.id === tab.sessionId)) ||
        (tab.sessionId && session?.id === tab.sessionId ? session : null) ||
        session;
      const tabBoundTask = tabSession?.taskId ? (tasks?.find((t) => t.id === tabSession.taskId) ?? null) : boundTask;
      return frameTabResource(
        <ConversationPane
          session={tabSession || { id: '', title: 'No conversation selected', messages: [], diffCode: [] }}
          boundTask={tabBoundTask}
          boundWorkspace={boundWorkspace || _workspaces?.find((w) => w.id === (tabBoundTask?.metadata?.workspace_id || activeWorkspaceId))}
          taskAttentionCount={taskAttentionCount}
          mode={(mode as ActiveWorkbench) || 'engineering'}
          onSendMessage={(t) => (onSendMessage ? onSendMessage(t, tabSession?.id) : onAskAgent?.(t, tabSession?.id))}
          onOpenTab={onOpenTab}
          onSelectTask={onSelectTask}
          onClearTask={onSelectTask ? () => onSelectTask(null) : undefined}
          onShowToast={onShowToast}
          isCompact={false}
        />
      );
    }

    // Tasks & Timeline Mapping
    if (tabType === 'tasks' || tabType === 'task') {
      return frameTabResource(
        <TasksPane
          tasks={tasks || []}
          sessions={allSessions || []}
          workspaces={_workspaces || []}
          selectedTaskId={tab.taskId || selectedTaskId || null}
          onSelectTask={(id) => onSelectTask?.(id)}
          onOpenSession={(sId) => onOpenSession?.(sId)}
          onNewSessionForTask={(tId) => onNewSessionForTask?.(tId)}
          onCreateTask={onCreateTask || (async () => ({} as any))}
          onAdvanceStatus={onAdvanceStatus || (async () => { })}
          onCompleteTask={onCompleteTask || (async () => { })}
          onCancelTask={onCancelTask || (async () => { })}
          onRefreshTasks={onRefreshTasks || (async () => [])}
        />
      );
    }

    // Coding Mappings
    if (tabType === 'files' || tabType === 'changes') {
      const isDiffExplicit = Boolean(tab.url && tab.url.startsWith('custos://diff?path='));
      const isFileExplicit = Boolean(tab.url && tab.url.startsWith('custos://file?path='));
      const filePathFromUrl = tab.url && (tab.url.startsWith('custos://file?path=') || tab.url.startsWith('custos://diff?path='))
        ? decodeURIComponent(tab.url.replace(/^custos:\/\/(file|diff)\?path=/, ''))
        : undefined;
      const effectiveWorkspaceId = tab.workspaceId || initialFilesWorkspaceId || activeWorkspaceId;
      return frameTabResource(
        <FilesChangesPane
          key={tab.id}
          sessionId={session?.id}
          initialWorkspaceId={effectiveWorkspaceId}
          initialFilePath={filePathFromUrl}
          initialViewMode={isDiffExplicit ? 'diff' : isFileExplicit ? 'content' : undefined}
          activeWorkspaceId={effectiveWorkspaceId}
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
            const ws = _workspaces?.find((w: import('@/types/domain').ExecutionWorkspace) => w.id === workspaceId);
            const wsLabel = ws?.kind?.type === 'git' ? ws.kind.branch : ws?.name;
            const resTitle = resource === 'terminal' ? 'Terminal' : resource === 'files' ? 'Files' : 'Agents';
            const customTitle = wsLabel ? `${resTitle} (${wsLabel})` : undefined;
            onOpenTab?.(resource, undefined, customTitle, 'primary', workspaceId);
          }}
        />
      );
    }

    if (tabType === 'agents') {
      const effectiveWorkspaceId = tab.workspaceId || initialAgentWorkspaceId || activeWorkspaceId;
      return frameTabResource(
        <AgentsWorkbenchPane
          key={tab.id}
          tabId={tab.id}
          initialHarnessId={tab.agentId}
          sessionId={session?.id}
          initialWorkspaceId={effectiveWorkspaceId}
          activeWorkspaceId={effectiveWorkspaceId}
          onSelectWorkspace={onSelectWorkspace}
          onShowToast={onShowToast}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'terminal') {
      const effectiveWorkspaceId = tab.workspaceId || initialTerminalWorkspaceId || activeWorkspaceId;
      return frameTabResource(
        <TerminalWorkbenchPane
          key={tab.id}
          initialWorkspaceId={effectiveWorkspaceId}
          activeWorkspaceId={effectiveWorkspaceId}
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
          onShowToast={onShowToast}
          onOpenTab={onOpenTab}
        />
      );
    }

    if (tabType === 'lab' || tabType === 'experiments' || tabType === 'runs') {
      return frameTabResource(
        <LabPane
          workspace={_workspaces?.find(workspace => workspace.id === tab.workspaceId) || null}
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
      return frameTabResource(<ResearchSynthesisPane
        initialWorkspaceId={tab.workspaceId}
        initialClaimId={synthesisClaimFromUrl(tab.url)}
        onShowToast={onShowToast}
        onNavigateToTask={onHandoffToCoding ? (taskId, receipt) => onHandoffToCoding([], { ...receipt, task_ids: [taskId] }) : undefined}
      />);
    }

    if (tabType === 'artifacts') {
      return frameTabResource(
        <ArtifactsWorkbenchPane workspaceId={tab.workspaceId} selectedArtifactPath={tab.artifactId} onShowToast={onShowToast} onAskAgent={onAskAgent} />
      );
    }

    // Copilot Mappings
    if (tabType === 'drafts') {
      return frameTabResource(
        <DraftsPane taskId={tab.taskId || session?.taskId} sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
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
            active={isResourceActive}
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
            active={isResourceActive}
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

  return renderTab(activeTab);
};

export const OrcaTabbedContainer = React.memo(function OrcaTabbedContainer(props: OrcaTabbedContainerProps) {
  const { resourceTabs } = props;
  const catalog = useResourceCatalog();
  const scope = resourceTabs.scopeKey || 'default';
  const storageKey = `custos.canvas.v1:${scope}`;
  const tabIds = resourceTabs.tabs.map(tab => tab.id);
  const tabSignature = JSON.stringify(tabIds);
  const load = React.useCallback(() => {
    try { return restoreCanvas(JSON.parse(localStorage.getItem(storageKey) || 'null'), tabIds, resourceTabs.activeTabId); }
    catch { return createCanvas(tabIds, resourceTabs.activeTabId); }
  }, [storageKey, tabSignature]);
  const [layouts, setLayouts] = React.useState<Record<string, CanvasState>>({});
  const restored = React.useMemo(load, [load]);
  const state = reconcileCanvas(layouts[scope] || restored, tabIds);
  const stateRef = React.useRef(state);
  stateRef.current = state;
  const dispatch = React.useCallback((action: CanvasAction) => {
    const next = canvasReducer(stateRef.current, action);
    setLayouts(previous => ({ ...previous, [scope]: next }));
    try { localStorage.setItem(storageKey, JSON.stringify(next)); } catch {}
  }, [scope, storageKey]);
  const resize = React.useRef<{ id: string; axis: 'horizontal' | 'vertical'; rect: DOMRect } | null>(null);
  const [dragTarget, setDragTarget] = React.useState<string | null>(null);
  const rootRef = React.useRef<HTMLDivElement>(null);
  const [visited, setVisited] = React.useState<Set<string>>(new Set());
  React.useEffect(() => {
    const visibleIds = canvasGroups(state.root).map(group => group.activeTabId).filter(Boolean);
    setVisited(previous => new Set([...previous, ...visibleIds].filter(id => tabIds.includes(id))));
  }, [scope, tabSignature, JSON.stringify(state.root)]);
  React.useEffect(() => {
    const move = (event: PointerEvent) => {
      const current = resize.current;
      if (!current) return;
      const ratio = current.axis === 'horizontal'
        ? (event.clientX - current.rect.left) / current.rect.width * 100
        : (event.clientY - current.rect.top) / current.rect.height * 100;
      dispatch({ type: 'resize', splitId: current.id, ratio });
    };
    const stop = () => { resize.current = null; document.body.classList.remove('canvas-resizing'); };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', stop);
    window.addEventListener('pointercancel', stop);
    return () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', stop); window.removeEventListener('pointercancel', stop); stop(); };
  }, [dispatch]);
  const focus = (group: CanvasGroup, tabId = group.activeTabId) => {
    dispatch({ type: 'focus', groupId: group.id, tabId });
    resourceTabs.setActiveTabId(tabId);
    const targetTab = resourceTabs.tabs.find((t) => t.id === tabId);
    if (targetTab?.sessionId && props.onOpenSession) {
      props.onOpenSession(targetTab.sessionId);
    }
    if (targetTab?.taskId && props.onSelectTask) {
      props.onSelectTask(targetTab.taskId);
    }
  };
  const split = (group: CanvasGroup, axis: 'horizontal' | 'vertical', tabId?: string) => {
    let targetTabId = tabId;
    if (!targetTabId && group.tabIds.length > 1) {
      targetTabId = group.tabIds.find(id => id !== group.activeTabId) || group.tabIds[1];
    }
    const newGroupId = crypto.randomUUID();
    dispatch({ type: 'split', groupId: group.id, newGroupId, axis, tabId: targetTabId });
    if (targetTabId) {
      resourceTabs.setActiveTabId(targetTabId);
    } else {
      // Group only had 1 tab: open a natural companion tab into the newly created split pane
      const companionType: OrcaTabId = isEngineeringMode ? 'terminal' : props.mode === 'research' ? 'lab' : 'tasks';
      const companionTitle = companionType === 'terminal' ? 'Terminal' : companionType === 'lab' ? 'Lab & Notebook' : 'Tasks & Approvals';
      const companionTabId = `${companionType}-${crypto.randomUUID()}`;
      const companionTab: OrcaTab = {
        id: companionTabId,
        type: companionType,
        title: companionTitle,
        url: urlFor(companionType),
        workspaceId: props.activeWorkspaceId,
        sessionId: props.session?.id,
      };
      resourceTabs.setTabs(prev => [...prev, companionTab]);
      resourceTabs.setActiveTabId(companionTabId);
    }
  };


  type Rect = { x: number; y: number; w: number; h: number };
  const leaves: { group: CanvasGroup; rect: Rect }[] = [];
  const separators: { node: Extract<CanvasNode, { kind: 'split' }>; rect: Rect }[] = [];
  const walk = (node: CanvasNode, rect: Rect) => {
    if (node.kind === 'group') { leaves.push({ group: node, rect }); return; }
    separators.push({ node, rect });
    const ratio = node.ratio / 100;
    walk(node.first, node.axis === 'horizontal' ? { ...rect, w: rect.w * ratio } : { ...rect, h: rect.h * ratio });
    walk(node.second, node.axis === 'horizontal' ? { ...rect, x: rect.x + rect.w * ratio, w: rect.w * (1 - ratio) } : { ...rect, y: rect.y + rect.h * ratio, h: rect.h * (1 - ratio) });
  };
  const groups = canvasGroups(state.root);
  walk(state.root, { x: 0, y: 0, w: 100, h: 100 });
  const singleGroup = state.zoomedGroupId;
  if (singleGroup) {
    const displayed = groups.find(group => group.id === singleGroup) || groups[0];
    leaves.length = 0; separators.length = 0;
    walk(displayed, { x: 0, y: 0, w: 100, h: 100 });
  }
  const position = (rect: Rect) => ({ left: `${rect.x}%`, top: `${rect.y}%`, width: `${rect.w}%`, height: `${rect.h}%` });
  const isEngineeringMode = props.mode === 'code' || props.mode === 'engineering';
  const deskKinds: OrcaTabId[] = props.mode === 'research' ? ['lab', 'literature', 'findings', 'artifacts']
    : isEngineeringMode ? ['terminal', 'files'] : ['tasks', 'drafts'];
  const deskUnavailable = deskKinds.map(kind => catalog.resources.find(resource => resource.id === kind)).find(resource => !resource || resource.status === 'unavailable');
  const deskCapabilitiesReady = deskKinds.every(kind => catalog.resources.some(resource => resource.id === kind && resource.status !== 'unavailable'));
  const deskScopeReady = !!props.session?.id && !!props.activeWorkspaceId && !!props.workspaces?.some(workspace => workspace.id === props.activeWorkspaceId && workspace.status !== 'archived');
  const openDesk = () => {
    if (!deskScopeReady || !deskCapabilitiesReady) return;
    const nextTabs = [...resourceTabs.tabs];
    const selected = deskKinds.map(kind => {
      const scopeRef = { workspaceId: props.activeWorkspaceId, sessionId: props.session?.id, taskId: props.selectedTaskId || props.session?.taskId };
      const existing = findScopedResourceTab(nextTabs, kind, scopeRef);
      if (existing) return existing.id;
      const resource = DEFAULT_RESOURCES.find(item => item.id === kind);
      const tab: OrcaTab = { ...scopeRef, id: `${kind}-${crypto.randomUUID()}`, type: kind, title: resource?.title || kind, url: urlFor(kind) };
      nextTabs.push(tab); return tab.id;
    });
    resourceTabs.setTabs(nextTabs);
    setLayouts(previous => ({ ...previous, [scope]: canvasReducer(reconcileCanvas(previous[scope] || restored, nextTabs.map(tab => tab.id)), { type: 'arrange', tabIds: selected, layout: 'review' }) }));
    resourceTabs.setActiveTabId(selected[0]);
  };
  const close = (tabId: string) => {
    const owner = groups.find(group => group.tabIds.includes(tabId));
    // If this pane only has this 1 tab and there are multiple split panes, close the split pane!
    if (owner && owner.tabIds.length <= 1 && groups.length > 1) {
      dispatch({ type: 'removeGroup', groupId: owner.id });
    } else {
      dispatch({ type: 'close', tabId });
    }
    resourceTabs.setTabs(previous => previous.filter(tab => tab.id !== tabId));
    if (resourceTabs.activeTabId === tabId) {
      const remaining = owner?.tabIds.filter(id => id !== tabId) || [];
      const fallbackTab = remaining[0] || tabIds.find(id => id !== tabId) || '';
      resourceTabs.setActiveTabId(fallbackTab);
    }
  };

  React.useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && stateRef.current.zoomedGroupId) {
        event.preventDefault();
        dispatch({ type: 'zoom', groupId: stateRef.current.zoomedGroupId });
        return;
      }
      if (!(event.metaKey || event.ctrlKey)) return;
      const currentGroups = canvasGroups(stateRef.current.root);
      const focused = currentGroups.find(group => group.id === stateRef.current.focusedGroupId) || currentGroups[0];
      if (event.key === '\\') {
        event.preventDefault();
        split(focused, event.shiftKey ? 'vertical' : 'horizontal');
      } else if (event.key.toLowerCase() === 't' && !event.shiftKey) {
        event.preventDefault();
        if (focused) focus(focused);
        const btn = rootRef.current?.querySelector<HTMLButtonElement>(`[data-group-id="${focused.id}"] .canvas-tab-new`) || rootRef.current?.querySelector<HTMLButtonElement>('.canvas-tab-new');
        const rect = btn?.getBoundingClientRect();
        props.onOpenResourcePicker?.('primary', rect ? {
          top: rect.bottom + 4,
          left: Math.max(12, Math.min(rect.left, window.innerWidth - 316)),
        } : undefined);
      } else if (event.key.toLowerCase() === 'w' && !event.shiftKey) {
        event.preventDefault();
        if (focused) {
          if (focused.activeTabId) {
            close(focused.activeTabId);
          } else if (currentGroups.length > 1) {
            dispatch({ type: 'removeGroup', groupId: focused.id });
          }
        }
      } else if (event.altKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)) {
        event.preventDefault();
        const index = currentGroups.findIndex(group => group.id === focused.id);
        const next = currentGroups[(index + (event.key === 'ArrowRight' ? 1 : currentGroups.length - 1)) % currentGroups.length];
        focus(next);
        rootRef.current?.querySelector<HTMLElement>(`[data-group-id="${next.id}"] [role="tab"]`)?.focus();
      }
    };
    const root = rootRef.current;
    root?.addEventListener('keydown', onKey);
    return () => root?.removeEventListener('keydown', onKey);
  }, [dispatch, resourceTabs.setActiveTabId, props.onOpenResourcePicker]);

  const {
    activeScreenId,
    activeScreen,
    updateActiveScreenLayout,
    registerCanvasActions,
  } = useScreenContext();

  const prevLayoutRef = React.useRef<string | null>(null);
  // Sync canvas layout when screen changes
  React.useEffect(() => {
    if (activeScreen?.canvasState) {
      const serialized = JSON.stringify(activeScreen.canvasState);
      if (serialized !== prevLayoutRef.current) {
        prevLayoutRef.current = serialized;
        setLayouts((previous) => ({
          ...previous,
          [scope]: reconcileCanvas(activeScreen.canvasState, tabIds),
        }));
      }
    }
  }, [activeScreenId, scope, tabSignature, activeScreen?.canvasState]);

  // Sync active canvas state back into screen model
  React.useEffect(() => {
    const serialized = JSON.stringify(state);
    if (serialized !== prevLayoutRef.current) {
      prevLayoutRef.current = serialized;
      updateActiveScreenLayout(state);
    }
  }, [activeScreenId, state, updateActiveScreenLayout]);

  // Expose focused pane operations to AppTopBar ScreenSwitcherBar
  React.useEffect(() => {
    registerCanvasActions({
      splitRight: () => {
        const focusedGroup = groups.find((g) => g.id === state.focusedGroupId) || groups[0];
        if (focusedGroup) split(focusedGroup, 'horizontal');
      },
      splitDown: () => {
        const focusedGroup = groups.find((g) => g.id === state.focusedGroupId) || groups[0];
        if (focusedGroup) split(focusedGroup, 'vertical');
      },
      toggleZoom: () => {
        if (state.focusedGroupId) {
          dispatch({ type: 'zoom', groupId: state.focusedGroupId });
        }
      },
      isZoomed: Boolean(state.zoomedGroupId),
    });
    return () => registerCanvasActions(null);
  }, [registerCanvasActions, dispatch, scope, tabSignature, JSON.stringify(state.root), state.focusedGroupId, state.zoomedGroupId]);

  return (
    <div className="h-full min-h-0 flex flex-col bg-canvas">
      <div ref={rootRef} className="workspace-canvas flex-1 min-h-0 relative overflow-hidden bg-canvas" aria-label="Resource workspace">
        {leaves.map(({ group, rect }) => (
          <section key={group.id} data-group-id={group.id} className={`canvas-group absolute flex flex-col overflow-hidden ${state.focusedGroupId === group.id ? 'is-focused' : ''}`}
            style={position(rect)} aria-label={`Resource group ${groups.indexOf(group) + 1}`}
            onPointerDown={() => focus(group)}
            onDragOver={event => { if (event.dataTransfer.types.includes('application/custos-tab-id')) { event.preventDefault(); setDragTarget(group.id); } }}
            onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setDragTarget(null); }}
            onDrop={event => {
              const id = event.dataTransfer.getData('application/custos-tab-id');
              setDragTarget(null); if (!tabIds.includes(id)) return;
              event.preventDefault();
              const bounds = event.currentTarget.getBoundingClientRect();
              if (event.clientY > bounds.bottom - bounds.height * .22) split(group, 'vertical', id);
              else if (event.clientX > bounds.right - bounds.width * .22) split(group, 'horizontal', id);
              else dispatch({ type: 'move', tabId: id, groupId: group.id });
              resourceTabs.setActiveTabId(id);
            }}>
            <header className="canvas-tabbar flex shrink-0 items-center gap-1 px-1">
              <div role="tablist" aria-label="Resources" className="flex flex-1 min-w-0 items-center overflow-x-auto no-scrollbar py-0.5">
                {group.tabIds.map(id => {
                  const tab = resourceTabs.tabs.find(item => item.id === id);
                  if (!tab) return null;
                  const isChatTab = tab.type === 'conversation' || tab.type === 'chat';
                  const tabSession = isChatTab
                    ? (tab.sessionId ? props.allSessions?.find((s) => s.id === tab.sessionId) : (tab.id === 'conversation' ? props.session : null))
                    : null;
                  const tabTitle = tabSession?.title || tab.title;
                  const tabWs = tab.workspaceId ? props.workspaces?.find((w: import('@/types/domain').ExecutionWorkspace) => w.id === tab.workspaceId) : null;
                  const wsBadge = tabWs?.kind?.type === 'git' ? tabWs.kind.branch : (tabWs?.lineage?.target_branch || tabWs?.name);
                  const fullTabTitle = wsBadge ? `[${wsBadge}] ${tabTitle}` : tabTitle;
                  const isActive = group.activeTabId === id;
                  return <div key={id} className={`canvas-tab group flex items-center shrink-0 ${isActive ? 'is-active' : ''}`} draggable
                    onDragStart={event => { event.dataTransfer.setData('application/custos-tab-id', id); event.dataTransfer.effectAllowed = 'move'; }}
                    onDragEnd={() => setDragTarget(null)}
                    onDragOver={event => { if (event.dataTransfer.types.includes('application/custos-tab-id')) event.preventDefault(); }}
                    onDrop={event => { const moved = event.dataTransfer.getData('application/custos-tab-id'); if (!tabIds.includes(moved)) return; event.stopPropagation(); event.preventDefault(); setDragTarget(null); dispatch({ type: 'move', tabId: moved, groupId: group.id, beforeTabId: id }); focus(group, moved); }}>
                    <button type="button" role="tab" id={`tab-${id}`} aria-selected={isActive} aria-controls={`resource-${id}`} tabIndex={isActive ? 0 : -1}
                      className="flex items-center gap-1.5 px-2.5 h-full min-w-0 select-none text-left" title={fullTabTitle} onClick={() => focus(group, id)}
                      onKeyDown={event => {
                        if (event.altKey || event.metaKey || event.ctrlKey) return;
                        if (!['ArrowRight', 'ArrowLeft', 'Home', 'End', 'Delete'].includes(event.key)) return;
                        event.preventDefault(); event.stopPropagation();
                        const index = group.tabIds.indexOf(id);
                        if (event.key === 'Delete') {
                          const next = group.tabIds[index + 1] || group.tabIds[index - 1];
                          close(id);
                          if (next) { focus(group, next); requestAnimationFrame(() => document.getElementById(`tab-${next}`)?.focus()); }
                          else requestAnimationFrame(() => rootRef.current?.querySelector<HTMLButtonElement>('[data-group-id] button[aria-label="Open resource in group"]')?.focus());
                          return;
                        }
                        const next = group.tabIds[event.key === 'Home' ? 0 : event.key === 'End' ? group.tabIds.length - 1
                          : (index + (event.key === 'ArrowRight' ? 1 : group.tabIds.length - 1)) % group.tabIds.length];
                        focus(group, next); document.getElementById(`tab-${next}`)?.focus();
                      }}>
                      {getTabIcon(getTabType(tab), isActive)}
                      <span className={`truncate max-w-40 text-[11px] ${isActive ? 'font-semibold text-white' : 'font-normal text-white/60'}`}>{tabTitle}</span>
                      {wsBadge && (
                        <span className={`font-mono text-[9px] px-1 py-0.2 rounded shrink-0 ${isActive ? 'bg-white/10 text-white/80 border border-white/15' : 'bg-white/5 text-white/40 border border-white/5'}`}>
                          {wsBadge}
                        </span>
                      )}
                    </button>
                    <button
                      type="button"
                      className="canvas-tab-close mr-1"
                      aria-label={`Close ${tabTitle}`}
                      onClick={event => { event.stopPropagation(); close(id); }}
                    >
                      <X size={11} />
                    </button>
                  </div>;
                })}

                {/* Browser-style '+' button directly next to the displayed tabs */}
                <button
                  type="button"
                  className="canvas-tab-new ml-1 cursor-pointer"
                  title="New Tab (⌘T)"
                  aria-label="Open resource in group"
                  onClick={(e) => {
                    e.stopPropagation();
                    focus(group);
                    const rect = e.currentTarget.getBoundingClientRect();
                    props.onOpenResourcePicker?.('primary', {
                      top: rect.bottom + 4,
                      left: Math.max(12, Math.min(rect.left, window.innerWidth - 316)),
                    });
                  }}
                >
                  <Plus size={14} />
                </button>
              </div>

              {/* Pane window controls */}
              <div className="flex items-center gap-0.5 shrink-0 pl-1 border-l border-white/5">
                <ActionIcon
                  icon={Columns2}
                  tooltip="Chia ô sang phải (⌘\)"
                  size="xs"
                  onClick={() => split(group, 'horizontal')}
                  ariaLabel="Split group right"
                />
                <ActionIcon
                  icon={state.zoomedGroupId ? Minimize2 : Maximize2}
                  tooltip={state.zoomedGroupId ? "Thu nhỏ về kích thước cũ (⇧⌘⏎)" : "Phóng to khung này (⇧⌘⏎)"}
                  size="xs"
                  active={Boolean(state.zoomedGroupId)}
                  onClick={() => dispatch({ type: 'zoom', groupId: group.id })}
                  ariaLabel="Zoom or restore group"
                />
                {groups.length > 1 && (
                  <ActionIcon
                    icon={X}
                    tooltip="Đóng ô chia này (Close pane)"
                    size="xs"
                    onClick={() => dispatch({ type: 'removeGroup', groupId: group.id })}
                    ariaLabel="Close pane"
                  />
                )}
                <details
                  className="canvas-layout-menu"
                  onKeyDown={event => {
                    if (event.key === 'Escape') {
                      event.preventDefault();
                      event.currentTarget.removeAttribute('open');
                      event.currentTarget.querySelector<HTMLElement>('summary')?.focus();
                    }
                  }}
                  onBlur={event => {
                    if (!event.currentTarget.contains(event.relatedTarget as Node)) {
                      event.currentTarget.removeAttribute('open');
                    }
                  }}
                >
                  <summary className="canvas-icon" aria-label="More layout actions" title="More layout actions">
                    <MoreHorizontal size={14} />
                  </summary>
                  <div className="canvas-layout-popover">
                    <p className="text-[10px] uppercase tracking-wider text-fg-muted px-2 py-1">Layout</p>
                    <button
                      type="button"
                      onClick={(event) => {
                        split(group, 'vertical');
                        event.currentTarget.closest('details')?.removeAttribute('open');
                      }}
                    >
                      Chia ô xuống dưới (Split Down)
                    </button>
                    {props.onToggleExpand && (
                      <button
                        type="button"
                        onClick={(event) => {
                          props.onToggleExpand?.();
                          event.currentTarget.closest('details')?.removeAttribute('open');
                        }}
                      >
                        {props.isExpanded ? 'Hiện hội thoại' : 'Mở rộng không gian'}
                      </button>
                    )}
                    {groups.length > 1 && (
                      <>
                        <div className="h-px bg-border-default my-1" />
                        {group.activeTabId && groups.filter(item => item.id !== group.id).map(item => (
                          <button
                            key={item.id}
                            type="button"
                            onClick={(event) => {
                              dispatch({ type: 'move', tabId: group.activeTabId, groupId: item.id });
                              resourceTabs.setActiveTabId(group.activeTabId);
                              event.currentTarget.closest('details')?.removeAttribute('open');
                            }}
                          >
                            Chuyển tab sang ô bên cạnh
                          </button>
                        ))}
                        <button
                          type="button"
                          onClick={(event) => {
                            dispatch({ type: 'merge' });
                            event.currentTarget.closest('details')?.removeAttribute('open');
                          }}
                        >
                          Gộp tất cả về 1 khung (Merge)
                        </button>
                        <button
                          type="button"
                          className="text-red-400 hover:text-red-300"
                          onClick={(event) => {
                            dispatch({ type: 'removeGroup', groupId: group.id });
                            event.currentTarget.closest('details')?.removeAttribute('open');
                          }}
                        >
                          Đóng ô chia này (Close Pane)
                        </button>
                      </>
                    )}
                    {!isEngineeringMode && (
                      <>
                        <div className="h-px bg-border-default my-1" />
                        <button
                          type="button"
                          disabled={!deskScopeReady || !deskCapabilitiesReady}
                          title={!deskScopeReady ? 'Select a conversation and workspace first' : !deskCapabilitiesReady ? deskUnavailable?.reason || 'Some desk resources are unavailable' : 'Arrange existing resources without starting execution'}
                          onClick={event => {
                            openDesk();
                            event.currentTarget.closest('details')?.removeAttribute('open');
                          }}
                        >
                          Arrange full {props.mode === 'research' ? 'Research' : 'Copilot'} desk
                        </button>
                      </>
                    )}
                  </div>
                </details>
              </div>
            </header>
            {!group.activeTabId && (
              isEngineeringMode ? (
                <div className="canvas-empty flex-1 flex flex-col items-center justify-center text-center p-8 gap-3 select-none">
                  <div className="size-12 rounded-2xl bg-white/[0.04] border border-white/8 flex items-center justify-center text-white/50 mb-1">
                    <Terminal size={22} className="text-white/60" />
                  </div>
                  <div>
                    <h3 className="text-sm font-semibold text-white/90">Engineering Workspace</h3>
                    <p className="text-xs text-white/40 mt-1 max-w-sm">
                      Build and inspect code with AI agents, integrated terminal, and live Git diffs.
                    </p>
                  </div>
                  <div className="flex flex-wrap items-center justify-center gap-2 mt-2">
                    <button
                      type="button"
                      className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition text-white/80 font-medium cursor-pointer"
                      onClick={() => { focus(group); props.onOpenTab?.('terminal'); }}
                    >
                      <Terminal size={14} className="text-white/60" />
                      New Terminal
                    </button>
                    <button
                      type="button"
                      className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition text-white/80 font-medium cursor-pointer"
                      onClick={() => { focus(group); props.onOpenTab?.('files'); }}
                    >
                      <FolderTree size={14} className="text-white/60" />
                      Files & Diff
                    </button>
                    {groups.length > 1 && (
                      <button
                        type="button"
                        className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-red-500/10 hover:bg-red-500/20 border border-red-500/20 transition text-red-300 font-medium cursor-pointer"
                        onClick={() => dispatch({ type: 'removeGroup', groupId: group.id })}
                      >
                        <X size={14} className="text-red-400" />
                        Đóng ô chia này
                      </button>
                    )}
                  </div>
                  <p className="text-[11px] text-white/30 mt-2 font-mono">
                    Press ⌥1, ⌥2, ⌥3 to switch workbenches · ⌥I for Inspector
                  </p>
                </div>
              ) : props.mode === 'research' ? (
                <div className="canvas-empty flex-1 flex flex-col items-center justify-center text-center p-8 gap-3 select-none">
                  <div className="size-12 rounded-2xl bg-white/[0.04] border border-white/8 flex items-center justify-center text-white/50 mb-1">
                    <FlaskConical size={22} className="text-white/60" />
                  </div>
                  <div>
                    <h3 className="text-sm font-semibold text-white/90">Research Workspace</h3>
                    <p className="text-xs text-white/40 mt-1 max-w-sm">
                      Investigate scientific literature, analyze data in notebooks, and synthesize verified findings.
                    </p>
                  </div>
                  <div className="flex flex-wrap items-center justify-center gap-2 mt-2">
                    <button
                      type="button"
                      className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition text-white/80 font-medium cursor-pointer"
                      onClick={() => { focus(group); props.onOpenTab?.('literature'); }}
                    >
                      <BookOpen size={14} className="text-white/60" />
                      Literature Sources
                    </button>
                    <button
                      type="button"
                      className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition text-white/80 font-medium cursor-pointer"
                      onClick={() => { focus(group); props.onOpenTab?.('lab'); }}
                    >
                      <FlaskConical size={14} className="text-white/60" />
                      Lab & Notebook
                    </button>
                    {groups.length > 1 && (
                      <button
                        type="button"
                        className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-red-500/10 hover:bg-red-500/20 border border-red-500/20 transition text-red-300 font-medium cursor-pointer"
                        onClick={() => dispatch({ type: 'removeGroup', groupId: group.id })}
                      >
                        <X size={14} className="text-red-400" />
                        Đóng ô chia này
                      </button>
                    )}
                  </div>
                  <p className="text-[11px] text-white/30 mt-2 font-mono">
                    Press ⌥2 for Research · ⌥I for Provenance & Evidence Inspector
                  </p>
                </div>
              ) : (
                <div className="canvas-empty flex-1 flex flex-col items-center justify-center text-center p-8 gap-3 select-none">
                  <div className="size-12 rounded-2xl bg-white/[0.04] border border-white/8 flex items-center justify-center text-white/50 mb-1">
                    <Bot size={22} className="text-white/60" />
                  </div>
                  <div>
                    <h3 className="text-sm font-semibold text-white/90">Copilot Workspace</h3>
                    <p className="text-xs text-white/40 mt-1 max-w-sm">
                      Orchestrate agent tasks, review pending actions with human-in-the-loop safeguards, and draft deliverables.
                    </p>
                  </div>
                  <div className="flex flex-wrap items-center justify-center gap-2 mt-2">
                    <button
                      type="button"
                      className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition text-white/80 font-medium cursor-pointer"
                      onClick={() => { focus(group); props.onOpenTab?.('tasks'); }}
                    >
                      <CheckSquare size={14} className="text-white/60" />
                      Tasks & Approvals
                    </button>
                    <button
                      type="button"
                      className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/10 transition text-white/80 font-medium cursor-pointer"
                      onClick={() => { focus(group); props.onOpenTab?.('drafts'); }}
                    >
                      <FileText size={14} className="text-white/60" />
                      Drafts & Reports
                    </button>
                    {groups.length > 1 && (
                      <button
                        type="button"
                        className="px-3.5 py-2 text-xs flex items-center gap-2 rounded-lg bg-red-500/10 hover:bg-red-500/20 border border-red-500/20 transition text-red-300 font-medium cursor-pointer"
                        onClick={() => dispatch({ type: 'removeGroup', groupId: group.id })}
                      >
                        <X size={14} className="text-red-400" />
                        Đóng ô chia này
                      </button>
                    )}
                  </div>
                  <p className="text-[11px] text-white/30 mt-2 font-mono">
                    Press ⌥3 for Copilot · ⌥I for Policy & Approvals Inspector
                  </p>
                </div>
              )
            )}
            {dragTarget === group.id && <div className="absolute inset-x-1 bottom-1 top-9 z-40 pointer-events-none rounded border-2 border-dashed border-accent bg-accent/10 flex items-center justify-center text-xs text-fg-default">Drop to move · right edge to split · bottom edge to stack</div>}
          </section>
        ))}
        {resourceTabs.tabs.filter(tab => visited.has(tab.id) || leaves.some(({ group }) => group.activeTabId === tab.id)).map(tab => {
          const leaf = leaves.find(({ group }) => group.activeTabId === tab.id);
          return <div key={`${scope}:${tab.id}`} id={`resource-${tab.id}`} role="tabpanel" aria-labelledby={`tab-${tab.id}`}
            className="workbench-resource absolute overflow-hidden" hidden={!leaf}
            style={leaf ? { left: `${leaf.rect.x}%`, top: `calc(${leaf.rect.y}% + 34px)`, width: `${leaf.rect.w}%`, height: `calc(${leaf.rect.h}% - 34px)` } : { display: 'none' }}
            onPointerDown={() => { if (leaf) focus(leaf.group); }}
            onDragOver={event => { if (leaf && event.dataTransfer.types.includes('application/custos-tab-id')) { event.preventDefault(); setDragTarget(leaf.group.id); } }}
            onDrop={event => {
              if (!leaf) return;
              const id = event.dataTransfer.getData('application/custos-tab-id');
              if (!tabIds.includes(id)) return;
              event.preventDefault(); setDragTarget(null);
              const bounds = event.currentTarget.getBoundingClientRect();
              if (event.clientY > bounds.bottom - bounds.height * .22) split(leaf.group, 'vertical', id);
              else if (event.clientX > bounds.right - bounds.width * .22) split(leaf.group, 'horizontal', id);
              else dispatch({ type: 'move', tabId: id, groupId: leaf.group.id });
              resourceTabs.setActiveTabId(id);
            }}>
            <ResourceContent {...props} catalog={catalog} session={tab.sessionId ? props.allSessions?.find(session => session.id === tab.sessionId) || (props.session?.id === tab.sessionId ? props.session : null) : props.session}
              onAskAgent={props.onAskAgent ? draft => props.onAskAgent?.(draft, tab.sessionId || props.session?.id) : undefined}
              onSendMessage={props.onSendMessage ? (text, sId) => props.onSendMessage?.(text, sId || tab.sessionId || props.session?.id) : undefined}
              activeWorkspaceId={tab.workspaceId} isResourceActive={!!leaf && props.isPaneOpen}
              resourceTabs={{ ...resourceTabs, activeTabId: tab.id, splitTabId: null }} />
          </div>;
        })}
        {separators.map(({ node, rect }) => <div key={node.id} role="separator" tabIndex={0} aria-label="Resize resource groups"
          aria-orientation={node.axis === 'horizontal' ? 'vertical' : 'horizontal'} aria-valuenow={Math.round(node.ratio)} aria-valuemin={20} aria-valuemax={80}
          className={`canvas-separator absolute z-30 ${node.axis === 'horizontal' ? 'is-horizontal' : 'is-vertical'}`}
          style={node.axis === 'horizontal' ? { left: `calc(${rect.x + rect.w * node.ratio / 100}% - 2px)`, top: `${rect.y}%`, width: 5, height: `${rect.h}%` } : { top: `calc(${rect.y + rect.h * node.ratio / 100}% - 2px)`, left: `${rect.x}%`, height: 5, width: `${rect.w}%` }}
          onDoubleClick={() => dispatch({ type: 'resize', splitId: node.id, ratio: 50 })}
          onKeyDown={event => { if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home'].includes(event.key)) return; event.preventDefault(); dispatch({ type: 'resize', splitId: node.id, ratio: event.key === 'Home' ? 50 : node.ratio + (['ArrowLeft', 'ArrowUp'].includes(event.key) ? -5 : 5) }); }}
          onPointerDown={event => {
            event.preventDefault(); const bounds = rootRef.current?.getBoundingClientRect(); if (!bounds) return;
            resize.current = { id: node.id, axis: node.axis, rect: new DOMRect(bounds.left + bounds.width * rect.x / 100, bounds.top + bounds.height * rect.y / 100, bounds.width * rect.w / 100, bounds.height * rect.h / 100) };
            document.body.classList.add('canvas-resizing');
          }} />)}
      </div>
    </div>
  );
});
