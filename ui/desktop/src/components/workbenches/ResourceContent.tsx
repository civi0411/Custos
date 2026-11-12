import React from 'react';
import { Wrench, Plus } from 'lucide-react';
import { WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { ErrorBoundary } from '@/components/common/ErrorBoundary';
import { ConversationPane } from '@/components/chat/ConversationPane';
import type { Session } from '@/types';
import type { ActiveWorkbench } from '@/types/workbench';
import { useAppContext } from '@/context/AppContext';
import {
  getWorkbenchForTabType,
  canonicalWorkbenchId,
  CanonicalWorkbenchId,
  WorkbenchNotInstalledPlaceholder,
} from './registry';
import { findScopedResourceTab, getTabType, urlFor, useResourceCatalog, workbenchResourceIds, type OrcaTab, type OrcaTabId, type ResourceDefinition, type ResourceTabsState } from './workbenchResources';
import type { DomainPaneContext } from './shared/DomainPaneContext';

const LazyCodingRouter = React.lazy(() =>
  import('./coding/router').then((m) => ({
    default: ({ ctx }: { ctx: DomainPaneContext }) => <>{m.renderCodingPane(ctx)}</>,
  }))
);
const LazyResearchRouter = React.lazy(() =>
  import('./research/router').then((m) => ({
    default: ({ ctx }: { ctx: DomainPaneContext }) => <>{m.renderResearchPane(ctx)}</>,
  }))
);
const LazyAssistantRouter = React.lazy(() =>
  import('./assistant/router').then((m) => ({
    default: ({ ctx }: { ctx: DomainPaneContext }) => <>{m.renderAssistantPane(ctx)}</>,
  }))
);
const LazyQuantRouter = React.lazy(() =>
  import('./quant/router').then((m) => ({
    default: ({ ctx }: { ctx: DomainPaneContext }) => <>{m.renderQuantPane(ctx)}</>,
  }))
);

const LAZY_WORKBENCH_ROUTERS: Partial<
  Record<CanonicalWorkbenchId, React.LazyExoticComponent<React.ComponentType<{ ctx: DomainPaneContext }>>>
> = {
  coding: LazyCodingRouter,
  research: LazyResearchRouter,
  assistant: LazyAssistantRouter,
  quant: LazyQuantRouter,
};

const TasksPane = React.lazy(() => import('@/components/tasks/TasksPane').then(module => ({ default: module.TasksPane })));
const BrowserWorkbenchPane = React.lazy(() => import('./shared/BrowserWorkbenchPane').then(module => ({ default: module.BrowserWorkbenchPane })));
const MobileSimulatorWorkbenchPane = React.lazy(() => import('./shared/MobileSimulatorWorkbenchPane').then(module => ({ default: module.MobileSimulatorWorkbenchPane })));


export interface OrcaTabbedContainerProps {
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
  mode?: 'chat' | 'code' | 'research' | 'engineering' | 'copilot' | 'quant';
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
  onNewConversation?: () => void;
  onNewSessionForTask?: (taskId: string) => void;
  onCreateTask?: (params: import('@/types/domain').CreateTaskParams) => Promise<import('@/types/domain').Task>;
  onAdvanceStatus?: (taskId: string, status: import('@/types/domain').TaskStatus) => Promise<void>;
  onCompleteTask?: (taskId: string, summary?: string) => Promise<void>;
  onCancelTask?: (taskId: string, reason?: string) => Promise<void>;
  onRefreshTasks?: () => Promise<import('@/types/domain').Task[]>;
}

const UnavailableResource = ({ resource, onRetry }: { resource: ResourceDefinition; onRetry?: () => void }) => <WorkbenchEmptyState title={resource.title + ' is unavailable'} description={resource.reason || resource.description} action={onRetry ? <button type="button" className="canvas-open-button" onClick={onRetry}>Retry connection</button> : undefined} />;

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
  onNewConversation,
  onCreateTask,
  onAdvanceStatus,
  onCompleteTask,
  onCancelTask,
  onRefreshTasks,
}) => {
  const { installedWorkbenches, setIsWorkbenchLibraryOpen } = useAppContext();
  const { resources, retry } = useResourceCatalog(catalog);
  const [renderAttempt, setRenderAttempt] = React.useState(0);
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
    const needsSession = ['conversation', 'chat', 'lab', 'experiments', 'runs', 'drafts'].includes(tabType);
    const scopeReason = needsWorkspace && !tab.workspaceId
      ? 'Select a workspace, then open this resource from its workbench.'
      : needsWorkspace && _workspaces?.length && !_workspaces.some(workspace => workspace.id === tab.workspaceId)
        ? 'This workspace is no longer registered. Close this tab and select an available workspace.'
        : needsSession && !session?.id ? 'Select or create a conversation to open this resource.' : null;
    if (scopeReason) return <WorkbenchEmptyState title={needsSession && !session?.id ? 'Start a conversation' : 'Choose a workspace'} description={scopeReason} action={needsSession && !session?.id && onNewConversation ? <button type="button" className="canvas-open-button" onClick={onNewConversation}><Plus size={13} />New conversation</button> : undefined} />;


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
      <div className="wb-resource-body flex h-full min-h-0 flex-col bg-[var(--color-canvas)]" data-resource-kind={tabType}>
        {tabResource?.reason && tabResource.status === 'degraded' && (
          <div className="shrink-0 border-b border-amber-500/20 bg-amber-500/5 px-3 py-1.5 text-[10px] text-amber-600 dark:text-amber-300">
            {tabResource.reason}
          </div>
        )}
        <div className="min-h-0 flex-1">{content}</div>
      </div>
    );

    // Workbench installation check: render placeholder if domain pack is not installed
    const responsibleWorkbench = getWorkbenchForTabType(tabType);
    if (responsibleWorkbench) {
      const isInstalled = (installedWorkbenches || []).some(
        (id) => canonicalWorkbenchId(id) === responsibleWorkbench
      );
      if (!isInstalled) {
        return (
          <WorkbenchNotInstalledPlaceholder
            workbenchId={responsibleWorkbench}
            onOpenLibrary={() => setIsWorkbenchLibraryOpen(true)}
          />
        );
      }
    }

    // Core Conversation Peer Mapping
    if (tabType === 'conversation' || tabType === 'chat') {
      const explicitSessionId = tab.sessionId;
      const resolvedExplicit = explicitSessionId
        ? (allSessions?.find((s) => s.id === explicitSessionId) || (session?.id === explicitSessionId ? session : null))
        : null;

      if (explicitSessionId && !resolvedExplicit) {
        return frameTabResource(
          <WorkbenchEmptyState
            title="Conversation Unavailable"
            description={`The conversation session "${explicitSessionId}" could not be found or has been closed.`}
            action={
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="px-3 py-1 text-xs rounded bg-white/10 hover:bg-white/20 text-white transition cursor-pointer"
                  onClick={() => {
                    resourceTabs.setTabs((prev) => {
                      const filtered = prev.filter((t) => t.id !== tab.id);
                      if (resourceTabs.activeTabId === tab.id && filtered.length > 0) {
                        resourceTabs.setActiveTabId(filtered[0].id);
                      }
                      return filtered;
                    });
                  }}
                >
                  Close Tab
                </button>
                {session?.id && (
                  <button
                    type="button"
                    className="px-3 py-1 text-xs rounded bg-workbench-accent/20 hover:bg-workbench-accent/30 text-workbench-accent border border-workbench-accent/30 transition cursor-pointer"
                    onClick={() => {
                      resourceTabs.setTabs((prev) =>
                        prev.map((t) => (t.id === tab.id ? { ...t, sessionId: session.id } : t))
                      );
                    }}
                  >
                    Rebind to Current Conversation
                  </button>
                )}
              </div>
            }
          />
        );
      }

      const tabSession = resolvedExplicit || session;
      const tabBoundTask = tabSession?.taskId ? (tasks?.find((t) => t.id === tabSession.taskId) ?? null) : boundTask;
      return frameTabResource(
        <ConversationPane
          session={tabSession || { id: '', title: 'No conversation selected', messages: [], diffCode: [] }}
          boundTask={tabBoundTask}
          boundWorkspace={_workspaces?.find((w) => w.id === (tab.workspaceId || tabBoundTask?.metadata?.workspace_id || activeWorkspaceId)) || boundWorkspace}
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

    // Domain-Specific Workbench Mappings
    const domainContext: DomainPaneContext = {
      tab,
      tabType,
      tabResource,
      session,
      tasks,
      allSessions,
      workspaces: _workspaces,
      activeWorkspaceId,
      initialAgentWorkspaceId,
      initialFilesWorkspaceId,
      initialTerminalWorkspaceId,
      setInitialAgentWorkspaceId,
      setInitialFilesWorkspaceId,
      setInitialTerminalWorkspaceId,
      isResourceActive,
      resourceTabs,
      frameTabResource,
      onShowToast,
      onAskAgent,
      onOpenTab,
      onSelectWorkspace,
      onHandoffToCoding,
    };

    if (responsibleWorkbench) {
      const LazyRouter = LAZY_WORKBENCH_ROUTERS[responsibleWorkbench];
      if (LazyRouter) {
        return (
          <React.Suspense
            fallback={
              <div
                role="status"
                aria-label={`Loading ${responsibleWorkbench} workbench`}
                className="h-full p-8 bg-canvas space-y-5 motion-safe:animate-pulse"
              >
                <div className="h-7 w-2/3 bg-surface-2 rounded" />
                <div className="h-32 bg-surface-1 rounded" />
              </div>
            }
          >
            <LazyRouter ctx={domainContext} />
          </React.Suspense>
        );
      }
    }


    // Shared Browser & Emulator
    if (tabType === 'browser') {
      return frameTabResource(
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
      return frameTabResource(
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

  return <ErrorBoundary key={(activeTab?.id || 'empty') + ':' + renderAttempt} fallback={<WorkbenchEmptyState title="This pane could not be displayed" description="The other working tabs remain available." action={<button type="button" className="canvas-open-button" onClick={() => setRenderAttempt(value => value + 1)}>Reload pane</button>} />}>
    <React.Suspense fallback={<WorkbenchEmptyState title="Opening resource…" busy />}>{renderTab(activeTab)}</React.Suspense>
  </ErrorBoundary>;
};
