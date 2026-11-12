import React from 'react';
import type { DomainPaneContext } from '../shared/DomainPaneContext';

const FilesChangesPane = React.lazy(() =>
  import('./FilesChangesPane').then((module) => ({ default: module.FilesChangesPane }))
);
const WorktreeManagerPane = React.lazy(() =>
  import('./WorktreeManagerPane').then((module) => ({ default: module.WorktreeManagerPane }))
);
const TerminalWorkbenchPane = React.lazy(() =>
  import('./TerminalWorkbenchPane').then((module) => ({ default: module.TerminalWorkbenchPane }))
);
const AgentsWorkbenchPane = React.lazy(() =>
  import('./AgentsWorkbenchPane').then((module) => ({ default: module.AgentsWorkbenchPane }))
);
const WorkflowDagWorkbenchPane = React.lazy(() =>
  import('./WorkflowDagWorkbenchPane').then((module) => ({ default: module.WorkflowDagWorkbenchPane }))
);
const EvidenceWorkbenchPane = React.lazy(() =>
  import('./EvidenceWorkbenchPane').then((module) => ({ default: module.EvidenceWorkbenchPane }))
);
const ActivityPane = React.lazy(() =>
  import('./ActivityPane').then((module) => ({ default: module.ActivityPane }))
);
const RemoteFleetWorkbenchPane = React.lazy(() =>
  import('../shared/RemoteFleetWorkbenchPane').then((module) => ({ default: module.RemoteFleetWorkbenchPane }))
);

export function renderCodingPane(ctx: DomainPaneContext): React.ReactNode | null {
  const {
    tab,
    tabType,
    tabResource,
    session,
    workspaces,
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
  } = ctx;

  if (tabType === 'files' || tabType === 'changes') {
    const isDiffExplicit = Boolean(tab.url && tab.url.startsWith('custos://diff?path='));
    const isFileExplicit = Boolean(tab.url && tab.url.startsWith('custos://file?path='));
    const filePathFromUrl =
      tab.url && (tab.url.startsWith('custos://file?path=') || tab.url.startsWith('custos://diff?path='))
        ? decodeURIComponent(tab.url.replace(/^custos:\/\/(file|diff)\?path=/, ''))
        : undefined;
    const effectiveWorkspaceId = tab.workspaceId || activeWorkspaceId || initialFilesWorkspaceId;
    return frameTabResource(
      <FilesChangesPane
        key={`${tab.id}-${effectiveWorkspaceId}`}
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

  if (tabType === 'activity' || tabType === 'kanban') {
    return frameTabResource(
      <ActivityPane
        sessionId={session?.id}
        initialRunId={tab.runId}
        canCancel={tabResource?.supportedOperations?.includes('runs.cancel') ?? false}
        onShowToast={onShowToast}
        onAskAgent={onAskAgent}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (tabType === 'dag') {
    return frameTabResource(
      <WorkflowDagWorkbenchPane
        sessionId={session?.id}
        initialRunId={tab.runId}
        canCancel={tabResource?.supportedOperations?.includes('runs.cancel') ?? false}
        onShowToast={onShowToast}
        onAskAgent={onAskAgent}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (tabType === 'evidence') {
    return frameTabResource(<EvidenceWorkbenchPane onShowToast={onShowToast} />);
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
          const ws = workspaces?.find((w: import('@/types/domain').ExecutionWorkspace) => w.id === workspaceId);
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
        runId={tab.runId}
        isActive={isResourceActive}
        onRunStarted={(sessionId, run) =>
          resourceTabs.setTabs((previous) =>
            previous.map((item) =>
              item.id === tab.id
                ? { ...item, sessionId, taskId: run.task_id, runId: run.run_id }
                : item
            )
          )
        }
        initialWorkspaceId={effectiveWorkspaceId}
        activeWorkspaceId={effectiveWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (tabType === 'terminal') {
    const effectiveWorkspaceId = tab.workspaceId || activeWorkspaceId || initialTerminalWorkspaceId;
    return frameTabResource(
      <TerminalWorkbenchPane
        key={tab.id}
        tabId={tab.id}
        isActive={isResourceActive}
        initialWorkspaceId={effectiveWorkspaceId}
        activeWorkspaceId={effectiveWorkspaceId}
        onSelectWorkspace={onSelectWorkspace}
        onOpenTab={onOpenTab}
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

  return null;
}

export const renderEngineeringPane = renderCodingPane;
