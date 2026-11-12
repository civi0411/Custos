import React from 'react';
import type { DomainPaneContext } from '../shared/DomainPaneContext';

const DraftsPane = React.lazy(() =>
  import('./DraftsPane').then((module) => ({ default: module.DraftsPane }))
);
const OutboxPane = React.lazy(() =>
  import('./OutboxPane').then((module) => ({ default: module.OutboxPane }))
);
const ContactsCalendarPane = React.lazy(() =>
  import('./ContactsCalendarPane').then((module) => ({ default: module.ContactsCalendarPane }))
);
const NotesWorkbenchPane = React.lazy(() =>
  import('../shared/NotesWorkbenchPane').then((module) => ({ default: module.NotesWorkbenchPane }))
);

export function renderAssistantPane(ctx: DomainPaneContext): React.ReactNode | null {
  const {
    tab,
    tabType,
    tabResource,
    session,
    frameTabResource,
    onShowToast,
    onAskAgent,
    onOpenTab,
  } = ctx;

  if (tabType === 'drafts') {
    return frameTabResource(
      <DraftsPane
        taskId={tab.taskId || session?.taskId}
        sessionId={session?.id}
        onShowToast={onShowToast}
        onOpenTab={onOpenTab}
      />
    );
  }

  if (tabType === 'outbox') {
    return frameTabResource(
      <OutboxPane
        taskId={tab.taskId}
        supportedOperations={tabResource?.supportedOperations || []}
        onShowToast={onShowToast}
      />
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

  return null;
}

export const renderCopilotPane = renderAssistantPane;
