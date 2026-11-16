import type { HandoffToCodingResult } from '@/types/domain';

export interface ResearchSynthesisPaneProps {
  onShowToast?: (msg: string) => void;
  initialWorkspaceId?: string;
  initialClaimId?: string;
  onNavigateToTask?: (taskId: string, result: HandoffToCodingResult) => void;
}
