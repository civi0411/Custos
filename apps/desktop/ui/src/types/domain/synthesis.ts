// Research Synthesis, Lineage, Notes, Reviewers, and Notebook Types
// Strictly mirrors platform/domain (custos-domain) and apps/daemon (custos-daemon) API

import type { WorkspaceFileContent } from './workspace';
import type { ArtifactLineageNode, AnnotationRecord } from '../research';

export interface ArtifactSummary {
  artifact_path: string;
  artifactPath?: string;
  latest_version: number;
  latestVersion?: number;
  latest_content_hash: string;
  latestContentHash?: string;
  versions_count: number;
  versionsCount?: number;
  produced_by_run_id?: string;
  producedByRunId?: string;
  updated_at: number;
  updatedAt?: number;
}

export interface LineageGraphNode {
  id: string;
  label: string;
  node_type: 'artifact' | 'run' | 'recipe' | 'note';
  nodeType?: 'artifact' | 'run' | 'recipe' | 'note';
  version?: number;
  content_hash?: string;
  contentHash?: string;
  timestamp: number;
  metadata: Record<string, unknown>;
}

export interface LineageGraphEdge {
  from: string;
  to: string;
  edge_type: string;
  edgeType?: string;
}

export interface ArtifactLineageGraph {
  nodes: LineageGraphNode[];
  edges: LineageGraphEdge[];
}

export interface ArtifactDetailResponse {
  artifact_path: string;
  artifactPath?: string;
  latest_version: number;
  latestVersion?: number;
  latest_content_hash: string;
  latestContentHash?: string;
  versions: ArtifactLineageNode[];
  annotations: AnnotationRecord[];
  content?: string;
  file?: WorkspaceFileContent;
  content_version?: number;
}

export interface NoteRecord {
  id: string;
  title: string;
  content: string;
  version: number;
  content_hash: string;
  contentHash?: string;
  session_id?: string;
  sessionId?: string;
  task_id?: string;
  taskId?: string;
  tags: string[];
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface NoteVersionRecord {
  id: string;
  note_id: string;
  noteId?: string;
  version: number;
  content: string;
  content_hash: string;
  contentHash?: string;
  created_at: number;
  createdAt?: number;
}

export interface SaveNoteParams {
  id?: string;
  title: string;
  content: string;
  session_id?: string;
  sessionId?: string;
  task_id?: string;
  taskId?: string;
  tags?: string[];
}

export type NotebookCellType = 'code' | 'markdown';
export type CellExecutionStatus = 'idle' | 'running' | 'success' | 'error';
export type KernelStatus = 'idle' | 'busy' | 'interrupted';

export interface NotebookCell {
  id: string;
  session_id: string;
  sessionId?: string;
  cell_type: NotebookCellType;
  cellType?: NotebookCellType;
  source: string;
  cell_index: number;
  cellIndex?: number;
  execution_count?: number | null;
  executionCount?: number | null;
  status: CellExecutionStatus;
  stdout?: string | null;
  stderr?: string | null;
  output_image?: string | null;
  outputImage?: string | null;
  wall_ms?: number | null;
  wallMs?: number | null;
  epoch: number;
  updated_at: number;
  updatedAt?: number;
  notebook_id?: string | null;
  notebookId?: string | null;
  cell_version?: number | null;
  cellVersion?: number | null;
}

export interface NotebookKernelState {
  session_id: string;
  sessionId?: string;
  epoch: number;
  status: KernelStatus;
  python_version: string;
  pythonVersion?: string;
  execution_counter: number;
  executionCounter?: number;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
  notebook_id?: string | null;
  notebookId?: string | null;
  kernel_id?: string | null;
  kernelId?: string | null;
}

export interface ExecuteCellParams {
  session_id: string;
  sessionId?: string;
  cell_id: string;
  cellId?: string;
  code: string;
  workspace_id?: string;
  workspaceId?: string;
  cwd?: string;
  task_id?: string;
  taskId?: string;
  notebook_id?: string;
  notebookId?: string;
  kernel_id?: string;
  kernelId?: string;
  cell_version?: number;
  cellVersion?: number;
  execution_request_id?: string;
  executionRequestId?: string;
}

export interface ExecuteCellResult {
  cell_id: string;
  cellId?: string;
  execution_count: number;
  executionCount?: number;
  status: CellExecutionStatus;
  stdout: string;
  stderr: string;
  output_image?: string | null;
  outputImage?: string | null;
  wall_ms: number;
  wallMs?: number;
  epoch: number;
  notebook_id?: string | null;
  notebookId?: string | null;
  kernel_id?: string | null;
  kernelId?: string | null;
  cell_version?: number | null;
  cellVersion?: number | null;
  execution_request_id?: string | null;
  executionRequestId?: string | null;
}

export type ReviewTargetType = 'task' | 'artifact' | 'diff' | 'run' | 'note' | 'workspace';
export type ReviewMethod = 'automated_verifier' | 'peer_review' | 'model_evaluation' | 'contract_proof' | 'runtime_inspection';
export type ReviewStatus = 'approved' | 'rejected' | 'degraded' | 'pending';
export type FindingSeverity = 'info' | 'warning' | 'error' | 'blocker';

export interface ReviewFinding {
  severity: FindingSeverity;
  criterion: string;
  message: string;
  file_path?: string;
  filePath?: string;
  line_number?: number;
  lineNumber?: number;
}

export interface ReviewerRecord {
  id: string;
  target_type: ReviewTargetType;
  targetType?: ReviewTargetType;
  target_id: string;
  targetId?: string;
  reviewer: string;
  method: ReviewMethod;
  status: ReviewStatus;
  evidence_summary: string;
  evidenceSummary?: string;
  evidence_digest?: string | null;
  evidenceDigest?: string | null;
  findings: ReviewFinding[];
  is_fresh: boolean;
  isFresh?: boolean;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface RecordReviewParams {
  target_type: ReviewTargetType;
  target_id: string;
  reviewer: string;
  method: ReviewMethod;
  status: ReviewStatus;
  evidence_summary: string;
  evidence_digest?: string | null;
  findings?: ReviewFinding[];
}

export type SynthesisProposalStatus = 'draft' | 'submitted' | 'handoff_completed' | 'rejected';

export interface ClaimHandoffSummary {
  claim_id: string;
  claimId?: string;
  statement: string;
  level: string;
  confidence_score: number;
  confidenceScore?: number;
  has_fresh_review: boolean;
  hasFreshReview?: boolean;
  sealed_proof_uri?: string | null;
  sealedProofUri?: string | null;
}

export interface RecipeHandoffSummary {
  recipe_id: string;
  recipeId?: string;
  name: string;
  command: string;
  inputs_count: number;
  inputsCount?: number;
  outputs: string[];
}

export interface ResearchSynthesisProposal {
  id: string;
  title: string;
  summary: string;
  claims: ClaimHandoffSummary[];
  recipes: RecipeHandoffSummary[];
  artifact_paths: string[];
  artifactPaths?: string[];
  workspace_id?: string | null;
  workspaceId?: string | null;
  target_branch?: string | null;
  targetBranch?: string | null;
  caveats: string[];
  status: SynthesisProposalStatus;
  created_at: number;
  createdAt?: number;
  updated_at: number;
  updatedAt?: number;
}

export interface SaveSynthesisProposalParams {
  id?: string;
  title: string;
  summary: string;
  claim_ids: string[];
  recipe_ids: string[];
  artifact_paths?: string[];
  workspace_id?: string;
  target_branch?: string;
}

export interface HandoffToCodingParams {
  proposal_id?: string;
  title: string;
  claim_ids: string[];
  recipe_ids: string[];
  artifact_paths?: string[];
  workspace_id?: string;
  target_branch?: string;
  enforce_verification: boolean;
}

export interface HandoffToCodingResult {
  handoff_id: string;
  handoffId?: string;
  proposal_id?: string | null;
  proposalId?: string | null;
  task_ids: string[];
  taskIds?: string[];
  workspace_id?: string | null;
  workspaceId?: string | null;
  target_branch?: string | null;
  targetBranch?: string | null;
  verified_claims_count: number;
  verifiedClaimsCount?: number;
  converted_recipes_count: number;
  convertedRecipesCount?: number;
  caveats: string[];
  timestamp: number;
}

