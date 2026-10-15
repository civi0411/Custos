import { ActivityPane, type ActivityPaneProps } from './ActivityPane';

/** Legacy entry point: run events are a timeline, not a verified workflow DAG. */
export type WorkflowDagWorkbenchPaneProps = ActivityPaneProps;
export const WorkflowDagWorkbenchPane = ActivityPane;
