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
  abstract?: string;
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
  ts: number;
}

export interface ArtifactLineageNode {
  artifactPath: string;
  version: number;
  contentHash: string;
  producedByRunId?: string;
  parentVersionHash?: string;
  timestamp: number;
}

export interface ArtifactVersion {
  label: string; // "v1", "v2"
  code: string;
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
