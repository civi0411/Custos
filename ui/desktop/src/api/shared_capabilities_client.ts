import type {
  ArtifactDetailResponse,
  ArtifactLineageGraph,
  ExecuteCellParams,
  ExecuteCellResult,
  NotebookCell,
  NotebookKernelState,
  NoteRecord,
  NoteVersionRecord,
  RecordReviewParams,
  ReviewerRecord,
  ReviewStatus,
  ReviewTargetType,
  SaveNoteParams,
  ArtifactSummary,
} from '@/types/domain';
import type {
  ArtifactLineageNode,
  ComputeTargetInfo,
  ExecutionAttemptRecord,
  PreflightReport,
} from '@/types/research';
import { annotationFromWire, fromDaemon } from './normalizers';
import type { DaemonTransport } from './workbenches/quant_client';

/**
 * Shared cross-workbench capabilities client (Ring 2):
 * Notebook kernels, cross-domain review ledger, compute targets/experiments,
 * CAS artifacts/lineage, and versioned persistent notes.
 */
export class SharedCapabilitiesClient {
  constructor(private transport: DaemonTransport) {}

  // Notebook kernel & cells
  async listNotebookCells(sessionId: string, notebookId?: string): Promise<NotebookCell[]> {
    return this.transport.request<NotebookCell[]>('v1.notebook.cells.list', {
      session_id: sessionId,
      ...(notebookId ? { notebook_id: notebookId } : {}),
    });
  }

  async saveNotebookCells(
    sessionId: string,
    cells: NotebookCell[],
    notebookId?: string
  ): Promise<{ saved: boolean; count: number }> {
    return this.transport.request<{ saved: boolean; count: number }>('v1.notebook.cells.save', {
      session_id: sessionId,
      cells,
      ...(notebookId ? { notebook_id: notebookId } : {}),
    });
  }

  async executeNotebookCell(params: ExecuteCellParams): Promise<ExecuteCellResult> {
    return this.transport.request<ExecuteCellResult>('v1.notebook.execute', params);
  }

  async interruptNotebookKernel(
    sessionId: string,
    notebookId?: string
  ): Promise<{ interrupted: boolean }> {
    return this.transport.request<{ interrupted: boolean }>('v1.notebook.interrupt', {
      session_id: sessionId,
      ...(notebookId ? { notebook_id: notebookId } : {}),
    });
  }

  async resetNotebookKernel(
    sessionId: string,
    notebookId?: string
  ): Promise<NotebookKernelState> {
    return this.transport.request<NotebookKernelState>('v1.notebook.reset', {
      session_id: sessionId,
      ...(notebookId ? { notebook_id: notebookId } : {}),
    });
  }

  async getNotebookKernelStatus(
    sessionId: string,
    notebookId?: string
  ): Promise<NotebookKernelState> {
    return this.transport.request<NotebookKernelState>('v1.notebook.status', {
      session_id: sessionId,
      ...(notebookId ? { notebook_id: notebookId } : {}),
    });
  }

  // Cross-domain review & evidence ledger
  async listReviews(params?: {
    target_type?: ReviewTargetType;
    target_id?: string;
    reviewer?: string;
    status?: ReviewStatus;
    fresh_only?: boolean;
  }): Promise<ReviewerRecord[]> {
    return this.transport.request<ReviewerRecord[]>('v1.reviews.list', params ?? {});
  }

  async getReview(id: string): Promise<ReviewerRecord | null> {
    return this.transport.request<ReviewerRecord | null>('v1.reviews.get', { id });
  }

  async recordReview(params: RecordReviewParams): Promise<ReviewerRecord> {
    return this.transport.request<ReviewerRecord>('v1.reviews.record', params);
  }

  async markReviewStale(id: string): Promise<{ marked_stale: boolean }> {
    return this.transport.request<{ marked_stale: boolean }>('v1.reviews.mark_stale', { id });
  }

  // Compute targets & supervised experiment attempts
  async listComputeTargets(): Promise<ComputeTargetInfo[]> {
    const res = await this.transport.request<unknown[]>('v1.compute.targets.list', {});
    return (res || []).map((t) => fromDaemon<ComputeTargetInfo>(t));
  }

  async preflightExperiment(params: {
    backend: string;
    recipe_id?: string;
    ssh_host?: string;
    ssh_port?: number;
    ssh_user?: string;
    pod_id?: string;
    cloud_api_key?: string;
  }): Promise<PreflightReport> {
    const res = await this.transport.request<unknown>('v1.experiment.preflight', params);
    return fromDaemon<PreflightReport>(res);
  }

  async submitExperiment(params: {
    experiment_id: string;
    recipe_id: string;
    backend: string;
    parent_checkpoint_id?: string;
    ssh_host?: string;
    ssh_port?: number;
    ssh_user?: string;
    pod_id?: string;
    cloud_api_key?: string;
  }): Promise<{ attempt: ExecutionAttemptRecord; externalRef: any }> {
    const res = await this.transport.request<{ attempt: unknown; external_ref: unknown }>(
      'v1.experiment.submit',
      params
    );
    return {
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
      externalRef: fromDaemon<any>(res.external_ref),
    };
  }

  async observeExperiment(
    attemptId: string
  ): Promise<{ attempt: ExecutionAttemptRecord; observation: any }> {
    const res = await this.transport.request<{ attempt: unknown; observation: unknown }>(
      'v1.experiment.observe',
      { attempt_id: attemptId }
    );
    return {
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
      observation: fromDaemon<any>(res.observation),
    };
  }

  async stopExperiment(
    attemptId: string
  ): Promise<{ stopped: boolean; attempt: ExecutionAttemptRecord }> {
    const res = await this.transport.request<{ stopped: boolean; attempt: unknown }>(
      'v1.experiment.stop',
      { attempt_id: attemptId }
    );
    return {
      stopped: res.stopped,
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
    };
  }

  async releaseExperiment(
    attemptId: string
  ): Promise<{ released: boolean; attempt: ExecutionAttemptRecord }> {
    const res = await this.transport.request<{ released: boolean; attempt: unknown }>(
      'v1.experiment.release',
      { attempt_id: attemptId }
    );
    return {
      released: res.released,
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
    };
  }

  async collectExperimentArtifacts(
    attemptId: string
  ): Promise<{ artifacts: any[]; attempt: ExecutionAttemptRecord }> {
    const res = await this.transport.request<{ artifacts: any[]; attempt: unknown }>(
      'v1.experiment.collect',
      { attempt_id: attemptId }
    );
    return {
      artifacts: res.artifacts || [],
      attempt: fromDaemon<ExecutionAttemptRecord>(res.attempt),
    };
  }

  // Artifacts & lineage
  listArtifacts = (): Promise<ArtifactSummary[]> =>
    this.transport.request<ArtifactSummary[]>('v1.artifacts.list');

  async getArtifact(path: string, workspaceId?: string): Promise<ArtifactDetailResponse> {
    const detail = await this.transport.request<ArtifactDetailResponse>('v1.artifacts.get', {
      artifact_path: path,
      workspace_id: workspaceId,
    });
    return { ...detail, annotations: (detail.annotations || []).map(annotationFromWire) };
  }

  getArtifactLineageGraph = (path?: string): Promise<ArtifactLineageGraph> =>
    this.transport.request<ArtifactLineageGraph>('v1.artifacts.lineage_graph', {
      artifact_path: path,
    });

  recordArtifactLineage = (node: ArtifactLineageNode): Promise<{ recorded: boolean }> =>
    this.transport.request<{ recorded: boolean }>('v1.artifacts.record_lineage', node);

  // Notes
  listNotes = (sessionId?: string): Promise<NoteRecord[]> =>
    this.transport.request<NoteRecord[]>('v1.notes.list', { session_id: sessionId });

  getNote = (id: string): Promise<NoteRecord> =>
    this.transport.request<NoteRecord>('v1.notes.get', { id });

  saveNote = (params: SaveNoteParams): Promise<NoteRecord> =>
    this.transport.request<NoteRecord>('v1.notes.save', params);

  listNoteVersions = (noteId: string): Promise<NoteVersionRecord[]> =>
    this.transport.request<NoteVersionRecord[]>('v1.notes.history', { note_id: noteId });
}

