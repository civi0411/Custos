/**
 * Custos Research Workbench Domain Types
 * Strict typing for Source Records, Cryptographic Passage Anchors,
 * 4-Level Claim Grounding Matrix (L0-L3), Reproducible Experiment Runs,
 * and Domain Visualizers.
 */

export type ClaimGroundingLevel =
  | 'L0_UNGROUNDED'   // Raw claim, no citation or empirical evidence
  | 'L1_CITED'        // Linked to a cryptographically anchored passage, cross-check pending
  | 'L2_VERIFIED'     // Verified via reproducible execution / invariant check
  | 'L3_SEALED';      // Invariant proof sealed into CAS Ledger (cas://bafy...)

export interface SourceRecord {
  id: string;               // UUID or CAS URI
  sourceType: 'paper' | 'dataset' | 'web' | 'repo';
  title: string;
  doi?: string;
  authors?: string[];
  year?: number;
  contentHash: string;      // BLAKE3 or SHA-256
  localPath?: string;
  verified: boolean;
  abstractText?: string;
  createdAt: number;
}

export interface PassageAnchor {
  id: string;
  sourceId: string;
  sourceTitle?: string;
  sectionTitle?: string;
  pageNumber?: number;
  startOffset: number;
  endOffset: number;
  exactText: string;
  passageHash: string;
}

export interface ClaimEvidenceLink {
  passageAnchorId: string;
  sourceTitle?: string;
  exactText?: string;
  relation: 'SUPPORTS' | 'REFUTES' | 'QUALIFIES';
  rationale: string;
  verifiedBy: 'deterministic_engine' | 'expert_review';
}

export interface ResearchClaim {
  id: string;
  statement: string;
  level: ClaimGroundingLevel;
  confidenceScore: number;  // 0.0 -> 1.0
  invariants: string[];     // e.g. ["INV-CLAIM-ACCURACY", "INV-NO-CONTRADICTION"]
  evidenceLinks: ClaimEvidenceLink[];
  createdAt: number;
  sealedProofUri?: string;
}

export interface EnvSnapshot {
  pythonVersion: string;
  lockfileHash: string;
  packageCount?: number;
  hardware: string;
  platform?: string;
}

export type CheckpointResumabilityLevel =
  | 'export_only'
  | 'warm_start'
  | 'training_state_resume'
  | 'validated_resume';

export interface CheckpointRelayState {
  latestObservedStep?: number;
  latestLocalCheckpointStep?: number;
  latestProtectedCheckpointStep?: number;
  lastProtectedAt?: string;
  resumabilityLevel?: CheckpointResumabilityLevel;
  resumedFromCheckpointId?: string;
  resumedFromAttemptId?: string;
  outputNamespace?: string;
}

export interface ResearchExperimentRun {
  runId: string;
  sessionId: string;
  command: string;
  cwd: string;
  status: 'pending' | 'running' | 'ok' | 'failed';
  wallMs: number;
  surface?: 'local' | 'hpc' | 'modal' | 'jupyter' | 'sandbox';
  reproducibility: 'deterministic' | 'partial' | 'unverified';
  inputMerkleRoot: string;
  outputMerkleRoot: string;
  envSnapshot: EnvSnapshot;
  sadePermitId?: string;
  casLogUri?: string;
  logText?: string;
  codeFiles?: { path: string; hash: string }[];
  outputFiles?: { path: string; hash: string; size: number }[];
  checkpointRelay?: CheckpointRelayState;
  ts: number;
}

export interface ArtifactLineageNode {
  artifactPath?: string;
  artifact_path?: string;
  version: number;
  contentHash?: string;
  content_hash?: string;
  producedByRunId?: string;
  produced_by_run_id?: string;
  parentVersionHash?: string;
  parent_version_hash?: string;
  timestamp: number;
}

export interface ArtifactVersion {
  label: string; // "v1", "v2"
  code: string;
  contentAvailable?: boolean;
  executionLog?: string;
  environment?: string;
  reviewPassed?: boolean;
  timestamp?: number;
}

export type ArtifactTab =
  | 'Code'
  | 'Execution Log'
  | 'Messages'
  | 'Environment'
  | 'Review';

export interface ArtifactInspectorData {
  title: string;
  filename: string;
  activeVersion: string;
  versions: ArtifactVersion[];
  inputs: string[];
  code: string;
  language: string;
  executionLog?: string;
  environment?: string;
  messages?: string[];
  reviewPassed?: boolean;
  reviewFindings?: {
    level: 'ok' | 'warn' | 'error';
    title: string;
    evidence?: string;
    check?: string;
  }[];
}

export interface NotebookCellData {
  id: number;
  cellType: 'code' | 'markdown';
  source: string;
  output?: string;
  outputImage?: string;
  executionCount?: number;
  status?: 'idle' | 'running' | 'error' | 'success';
}

export interface TableColumnSpec {
  name: string;
  index: number;
  numeric: boolean;
  values: (number | null)[];
}

export interface ParsedTableData {
  headers: string[];
  rows: string[][];
}

export type ChartType = 'line' | 'bar' | 'scatter';

export interface AnnotationTargetSpec {
  type: 'text' | 'image_pin' | 'dom_element';
  startOffset?: number;
  endOffset?: number;
  exactText?: string;
  xRatio?: number;
  yRatio?: number;
  pinNumber?: number;
  selector?: string;
}

export interface AnnotationRecord {
  id: string;
  artifactPath: string;
  artifactVersion: number;
  target: AnnotationTargetSpec;
  note: string;
  actor: string;
  status: 'pending' | 'submitted' | 'addressed' | 'dismissed';
  sideChatSessionId?: string;
  submittedTurnId?: string;
  createdAt: number;
}

export interface ResearchEnvironmentSpec {
  pythonVersion?: string;
  requirements: string[];
  containerImage?: string;
  hardware?: string;
}

export interface ResearchRecipeInput {
  name: string;
  sourceUri: string;
  required: boolean;
}

export interface ResearchRecipe {
  id: string;
  name: string;
  description?: string;
  command: string;
  environmentSpec: ResearchEnvironmentSpec;
  inputs: ResearchRecipeInput[];
  outputs: string[];
  createdAt: string;
}

export interface ResearchExecutionArtifact {
  path: string;
  contentHash: string;
  sizeBytes: number;
}

export interface ResearchExecutionRecord {
  id: string;
  recipeId: string;
  sessionId?: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled' | 'preempted';
  exitCode?: number;
  stdoutCasUri?: string;
  stderrCasUri?: string;
  startedAt: string;
  endedAt?: string;
  artifacts: ResearchExecutionArtifact[];
  backend?: 'local' | 'colab' | 'kaggle' | 'dedicated_ssh' | 'run_pod' | 'vast_ai' | 'lambda_cloud' | null;
  checkpoint?: TrainingCheckpointManifest | null;
  resumeFromAttemptId?: string | null;
  resumedFromCheckpointId?: string | null;
  outputNamespace?: string | null;
  latestObservedStep?: number | null;
  latestLocalCheckpointStep?: number | null;
  latestProtectedCheckpointStep?: number | null;
  lastProtectedAt?: string | null;
}

export interface TrainingCheckpointManifest {
  checkpointId: string;
  globalStep: number;
  precision: string;
  commitStatus: 'observed' | 'local_staged' | 'uploading' | 'committed_protected' | 'corrupted';
  resumabilityLevel: 'export_only' | 'warm_start' | 'training_state_resume' | 'validated_resume';
  files: { path: string; sizeBytes: number; digest: string; requiredForStateResume: boolean }[];
}

export interface CheckpointResumeAssessment {
  metadataValid: boolean;
  stateInventoryDeclared: boolean;
  canStart: boolean;
  blockers: string[];
  unverifiedChecks: string[];
}

// ---------------------------------------------------------
// Untrusted Client Proposals (Layer 0 Invariants)
// Client proposes; backend hashes, verifies and materializes.
// ---------------------------------------------------------

export interface ResearchClaimProposal {
  id: string;
  statement: string;
  invariants: string[];
  createdAt: number;
}

export interface PassageAnchorProposal {
  sourceId: string;
  sectionTitle?: string;
  pageNumber?: number;
  startOffset: number;
  endOffset: number;
  exactText: string;
}

export interface SourceProposal {
  id: string;
  sourceType: string;
  title: string;
  locator: string;
  doi?: string;
  authors?: string[];
  year?: number;
  abstractText?: string;
  createdAt: number;
}

