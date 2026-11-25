import React from 'react';
import type { DomainPaneContext } from '../shared/DomainPaneContext';

const AlphaStudioPane = React.lazy(() =>
  import('./AlphaStudioPane').then((module) => ({ default: module.AlphaStudioPane }))
);
const TearsheetPane = React.lazy(() =>
  import('./TearsheetPane').then((module) => ({ default: module.TearsheetPane }))
);
const RiskAuditPane = React.lazy(() =>
  import('./RiskAuditPane').then((module) => ({ default: module.RiskAuditPane }))
);
const ExecutionPane = React.lazy(() =>
  import('./ExecutionPane').then((module) => ({ default: module.ExecutionPane }))
);

export function renderQuantPane(ctx: DomainPaneContext): React.ReactNode | null {
  const { tabType, session, frameTabResource, onShowToast, onOpenTab } = ctx;

  if (tabType === 'alpha' || tabType === 'alphastudio') {
    return frameTabResource(
      <AlphaStudioPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (tabType === 'tearsheet') {
    return frameTabResource(
      <TearsheetPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (tabType === 'risk_audit' || tabType === 'riskaudit') {
    return frameTabResource(
      <RiskAuditPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  if (tabType === 'execution') {
    return frameTabResource(
      <ExecutionPane sessionId={session?.id} onShowToast={onShowToast} onOpenTab={onOpenTab} />
    );
  }

  return null;
}
