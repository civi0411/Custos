import React from 'react';
import { NotebookWorkbenchPane, type NotebookWorkbenchPaneProps } from './NotebookWorkbenchPane';
import { RunsLedgerPane } from './RunsLedgerPane';
import { TrainingRunsPane } from './TrainingRunsPane';
export type LabPaneProps = NotebookWorkbenchPaneProps & {
  initialView?: 'notebook' | 'runs' | 'training';
};
/** Navigation never launches, cancels or retargets a run. */
export const LabPane: React.FC<LabPaneProps> = (props) => {
  if (props.initialView === 'runs') return <RunsLedgerPane sessionId={props.sessionId} onShowToast={props.onShowToast} onOpenTab={props.onOpenTab} />;
  if (props.initialView === 'training') return <TrainingRunsPane key={props.sessionId ?? 'no-session'} sessionId={props.sessionId} />;
  return <NotebookWorkbenchPane {...props} />;
};
