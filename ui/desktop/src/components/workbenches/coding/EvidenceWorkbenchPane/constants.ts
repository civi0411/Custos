import type {
  ReviewTargetType,
  ReviewMethod,
  ReviewStatus,
  FindingSeverity,
} from '@/types/domain';

export const TARGET_TYPES: { label: string; value: ReviewTargetType }[] = [
  { label: 'Artifact', value: 'artifact' },
  { label: 'Diff', value: 'diff' },
  { label: 'Task', value: 'task' },
  { label: 'Run', value: 'run' },
  { label: 'Note', value: 'note' },
  { label: 'Workspace', value: 'workspace' },
];

export const REVIEW_METHODS: { label: string; value: ReviewMethod }[] = [
  { label: 'Automated Verifier', value: 'automated_verifier' },
  { label: 'Contract Proof', value: 'contract_proof' },
  { label: 'Model Evaluation', value: 'model_evaluation' },
  { label: 'Peer Review', value: 'peer_review' },
  { label: 'Runtime Inspection', value: 'runtime_inspection' },
];

export const REVIEW_STATUSES: { label: string; value: ReviewStatus }[] = [
  { label: 'Approved', value: 'approved' },
  { label: 'Rejected', value: 'rejected' },
  { label: 'Degraded', value: 'degraded' },
  { label: 'Pending', value: 'pending' },
];

export const SEVERITIES: { label: string; value: FindingSeverity }[] = [
  { label: 'Info', value: 'info' },
  { label: 'Warning', value: 'warning' },
  { label: 'Error', value: 'error' },
  { label: 'Blocker', value: 'blocker' },
];
