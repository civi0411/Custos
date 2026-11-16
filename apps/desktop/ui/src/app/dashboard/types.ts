import type { SupervisionFinding } from '@/types/workflow_telemetry';

export type RecordedRun = {
  run_id: string;
  status: string;
  model?: string;
  journal_occurred_at?: string;
  duration_ms?: number | null;
  tokens?: number | null;
  cost_usd?: number | null;
  workbench_id?: string | null;
};

export type WorkItem = RecordedRun & {
  sessionId?: string;
  taskId?: string;
  title: string;
  project?: string;
  lens?: string;
  workspaceId?: string;
  findings?: SupervisionFinding[];
};

export type Bucket = 'attention' | 'running' | 'review' | 'history';

