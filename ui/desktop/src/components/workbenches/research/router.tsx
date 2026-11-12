import React from 'react';
import type { DomainPaneContext } from '../shared/DomainPaneContext';
import { synthesisClaimFromUrl } from '../workbenchResources';

const LiteraturePane = React.lazy(() =>
  import('./LiteraturePane').then((module) => ({ default: module.LiteraturePane }))
);
const ResearchMethodsPane = React.lazy(() =>
  import('./ResearchMethodsPane').then((module) => ({ default: module.ResearchMethodsPane }))
);
const ArtifactsWorkbenchPane = React.lazy(() =>
  import('./ArtifactsWorkbenchPane').then((module) => ({ default: module.ArtifactsWorkbenchPane }))
);
const ResearchSynthesisPane = React.lazy(() =>
  import('./ResearchSynthesisPane').then((module) => ({ default: module.ResearchSynthesisPane }))
);
const FindingsPane = React.lazy(() =>
  import('./FindingsPane').then((module) => ({ default: module.FindingsPane }))
);
const LabPane = React.lazy(() =>
  import('./LabPane').then((module) => ({ default: module.LabPane }))
);

export function renderResearchPane(ctx: DomainPaneContext): React.ReactNode | null {
  const {
    tab,
    tabType,
    session,
    workspaces,
    frameTabResource,
    onShowToast,
    onAskAgent,
    onOpenTab,
    onHandoffToCoding,
  } = ctx;

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

  if (tabType === 'lab') {
    return frameTabResource(
      <LabPane
        workspace={workspaces?.find((w) => w.id === tab.workspaceId) || null}
        sessionId={session?.id}
        initialView="notebook"
        onAskAgent={onAskAgent}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (tabType === 'runs') {
    return frameTabResource(
      <LabPane
        workspace={workspaces?.find((w) => w.id === tab.workspaceId) || null}
        sessionId={session?.id}
        initialView="runs"
        onAskAgent={onAskAgent}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (tabType === 'experiments' || tabType === 'training') {
    return frameTabResource(
      <LabPane
        workspace={workspaces?.find((w) => w.id === tab.workspaceId) || null}
        sessionId={session?.id}
        initialView="training"
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
    return frameTabResource(
      <ResearchSynthesisPane
        initialWorkspaceId={tab.workspaceId}
        initialClaimId={synthesisClaimFromUrl(tab.url)}
        onShowToast={onShowToast}
        onNavigateToTask={
          onHandoffToCoding
            ? (taskId, receipt) => onHandoffToCoding([], { ...receipt, task_ids: [taskId] })
            : undefined
        }
      />
    );
  }

  if (tabType === 'artifacts') {
    return frameTabResource(
      <ArtifactsWorkbenchPane
        workspaceId={tab.workspaceId}
        selectedArtifactPath={tab.artifactId}
        onShowToast={onShowToast}
        onAskAgent={onAskAgent}
      />
    );
  }

  return null;
}
