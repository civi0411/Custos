export type OperationalMode = 'Code' | 'Research' | 'Assitant';

export type TaskStatus = 
  | 'Draft' 
  | 'Queued' 
  | 'Running' 
  | 'Blocked' 
  | 'Succeeded' 
  | 'Failed' 
  | 'Cancelled';

export type RiskLevel = 'Low' | 'Medium' | 'High' | 'Critical';

export type TaskLifecycleState = 
  | 'Idle' 
  | 'InspectionApproval' 
  | 'RunningCoding' 
  | 'Succeeded' 
  | 'Failed' 
  | 'Cancelled';

export type ResponsiveTier = 'Compact' | 'Standard' | 'Wide' | 'UltraWide';

export type ViewMode = 'terminal' | 'visual' | 'split';

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  epoch: number;
  created_at: string;
  updated_at: string;
  metadata?: Record<string, unknown> | null;
  result_summary?: string;
  failure_reason?: string;
  rationale?: string;
}

export interface TaskSpan {
  id: number;
  task_id: string;
  name: string;
  stage: string;
  started_at: string;
  duration_ms?: number;
  details?: string;
}

export interface ExecutionPermit {
  id: string;
  taskId: string;
  actionType: string;
  target: string;
  risk: RiskLevel;
  filePath: string;
  oldCode: string;
  newCode: string;
  status: 'pending' | 'approved' | 'rejected';
  createdAt: string;
}

export type TerminalLineType = 
  | 'banner'
  | 'prompt'
  | 'system'
  | 'input'
  | 'output'
  | 'info'
  | 'success'
  | 'warning'
  | 'error'
  | 'diff'
  | 'card'
  | 'spinner';

export interface TerminalLine {
  id: string;
  type: TerminalLineType;
  content: string;
  timestamp?: string;
  metadata?: {
    mode?: OperationalMode;
    risk?: RiskLevel;
    filePath?: string;
    additions?: number;
    deletions?: number;
    permitId?: string;
    taskId?: string;
    status?: TaskStatus;
    showControls?: boolean;
  };
}

export interface ModeConfig {
  mode: OperationalMode;
  name: string;
  mascotName: string;
  mascotImage: string;
  badgeColor: string;
  tagline: string;
  description: string;
  capabilities: string[];
}
