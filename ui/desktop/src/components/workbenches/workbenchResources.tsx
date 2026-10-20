import React from 'react';
import WorktreeIcon from '~icons/codicon/worktree';
import { Activity, BookOpen, Bot, FileCode, FileDiff, FileText, FlaskConical, FolderTree, Globe, ShieldCheck, Terminal, Layers, Server, Smartphone, ArrowRight, Send, Users, Calendar, MessageSquare, CheckSquare } from 'lucide-react';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor } from '@/types/domain';
import type { ResourceRef } from '@/types/canvas';

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
  | 'runs'
  | 'training';

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
  return [...DEFAULT_RESOURCES.map(resource => resource.id), 'tools', 'chat', 'task', 'changes', 'kanban', 'evidence', 'dag', 'claims', 'experiments', 'runs', 'training'].includes(kind)
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
    title: 'Code Editor',
    description: 'Code editor and file viewer for workspace files.',
    icon: FileCode,
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
    title: 'Notebook',
    description: 'Notebook cells and observed Python output.',
    icon: FlaskConical,
    group: 'Research',
    status: 'available',
    supportedOperations: ['cells.list', 'cells.save', 'execute', 'interrupt', 'reset'],
  },
  { id: 'runs', title: 'Runs', description: 'Experiment logs, inputs, environments and outputs.', icon: Activity, group: 'Research', status: 'available' },
  { id: 'training', title: 'Training', description: 'Recorded training attempts, compute targets and checkpoint recovery.', icon: Server, group: 'Research', status: 'available' },
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
    : mode === 'research' ? ['literature', 'findings', 'lab', 'runs', 'training', 'knowledge', 'artifacts', 'synthesis', 'terminal', 'files', 'fleet']
      : ['drafts', 'outbox', 'contacts', 'calendar']) as OrcaTabId[], ...shared];
}

/** The daemon is authoritative for executable resources; native previews are host-scoped. */
export function resolveResources(
  capabilities: CapabilityDescriptor[],
  loaded: boolean,
  failed: boolean,
  _nativeHost = false
): ResourceDefinition[] {
  const aliases: Partial<Record<OrcaTabId, string>> = {
    activity: 'kanban',
    findings: 'claims',
    lab: 'experiments',
    training: 'knowledge',
  };

  return DEFAULT_RESOURCES.map((def) => {
    if (def.id === 'browser' || def.id === 'emulator') {
      return {
        ...def,
        status: 'available',
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
    return {
      ...def,
      status: matched.status.type,
      reason: 'reason' in matched.status ? matched.status.reason : undefined,
      supportedOperations: matched.supportedOperations ?? matched.supported_operations ?? [],
    };
  });
}

export const urlFor = (id: OrcaTabId) =>
  id === 'tools' ? 'custos://resources' : `custos://resource/${id}`;

export const getTabIcon = (type: OrcaTabId, active: boolean, tab?: OrcaTab) => {
  const cls = `size-3.5 shrink-0 ${active ? 'text-white' : 'text-white/40'}`;
  switch (type) {
    case 'conversation':
    case 'chat': return <MessageSquare className={cls} />;
    case 'tasks':
    case 'task': return <CheckSquare className={cls} />;
    case 'files': {
      if (tab?.url?.startsWith('custos://diff?')) return <FileDiff className={cls} />;
      if (tab?.url?.startsWith('custos://file?')) return <FileCode className={cls} />;
      return <FolderTree className={cls} />;
    }
    case 'activity': return <Activity className={cls} />;
    case 'terminal': return <Terminal className={cls} />;
    case 'worktrees': return <WorktreeIcon className={cls} />;
    case 'agents': {
      if (tab?.agentId) {
        return getAgentBrandIcon(tab.agentId, 14, cls);
      }
      return <Bot className={cls} />;
    }
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

export function useResourceCatalog(provided?: { resources: ResourceDefinition[]; retry: () => void }) {
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
