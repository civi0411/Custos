import type {
  ExecutionWorkspace,
  WorkspaceFileTree,
  WorkspaceFileContent,
  WriteWorkspaceFileParams,
  WorkspaceDiffSummary,
  WorkspaceFileDiff,
  HarnessDescriptor,
  RunNativeHarnessParams,
  HarnessExecutionResult,
  ResourceActionCapability,
} from '@/types/domain';
import {
  fromDaemon,
  toDaemon,
  normalizeWorkspace,
  normalizeWorkspaceFile,
  normalizeHarness,
} from '../normalizers';
import type { DaemonTransport } from './quant_client';

export class CodingClient {
  constructor(private transport: DaemonTransport) {}

  async spawnCandidates(params: {
    task_id: string;
    base_workspace_id?: string;
    repo_path?: string;
    count?: number;
    base_commit_hash?: string;
    harness_ids?: string[];
    setup_script?: string;
    command_id?: string;
  }): Promise<{
    task_id: string;
    candidates: ExecutionWorkspace[];
    partial: boolean;
    failed: { candidate_index: number; error: string }[];
  }> {
    const raw = await this.transport.request<{
      task_id: string;
      candidates: unknown[];
      partial?: boolean;
      failed?: { candidate_index: number; error: string }[];
    }>('v1.workspaces.candidates.spawn', params);
    return {
      task_id: raw.task_id,
      partial: raw.partial === true,
      failed: raw.failed || [],
      candidates: (raw.candidates || []).map(normalizeWorkspace),
    };
  }

  async compareCandidates(params: {
    task_id?: string;
    workspace_ids?: string[];
  }): Promise<{
    count: number;
    candidates: Array<{
      workspace_id: string;
      name: string;
      path: string;
      branch: string;
      base_commit_hash?: string;
      is_dirty: boolean;
      modified_count: number;
      untracked_count: number;
      deleted_count: number;
      modified_files: string[];
      untracked_files: string[];
      deleted_files: string[];
      diff: string;
      status: string;
    }>;
  }> {
    return this.transport.request('v1.workspaces.candidates.compare', params);
  }

  async runCandidateRace(params: {
    task_id: string;
    base_workspace_id?: string;
    candidates?: Array<{
      candidate_id: string;
      model_id: string;
      harness_id: string;
      workspace_id?: string;
    }>;
    test_command?: string;
    timeout_seconds?: number;
  }): Promise<{
    race_id: string;
    task_id: string;
    winner_candidate_id?: string;
    candidate_reports: Array<{
      candidate_id: string;
      model_id: string;
      workspace_id: string;
      worktree_path: string;
      test_passed: boolean;
      tests_succeeded: number;
      tests_failed: number;
      test_output: string;
      modified_files: string[];
      total_tokens: number;
      execution_duration_ms: number;
    }>;
    completed_at: string;
  }> {
    return this.transport.request('v1.workspaces.candidates.race', params);
  }

  async integrateCandidate(params: {
    winner_workspace_id: string;
    target_branch?: string;
    loser_workspace_ids?: string[];
    force_loser_cleanup?: boolean;
    preview?: boolean;
    expected_target_head?: string;
    expected_winner_head?: string;
  }): Promise<{
    winner_workspace_id: string;
    winner_branch: string;
    target_branch: string;
    is_stale_base: boolean;
    current_target_head?: string;
    winner_base_commit?: string;
    winner_head?: string;
    merge_status?: string;
    preview?: boolean;
    merge_success: boolean;
    merge_detail: string;
    loser_cleanup: Array<{ workspace_id: string; success: boolean; error?: string }>;
  }> {
    return this.transport.request('v1.workspaces.integrate', params);
  }

  async getWorkspaceFileTree(
    workspaceId: string,
    relativeDir?: string,
    maxDepth?: number
  ): Promise<WorkspaceFileTree> {
    return fromDaemon<WorkspaceFileTree>(
      await this.transport.request<unknown>('v1.workspace.files.tree', {
        workspace_id: workspaceId,
        relative_dir: relativeDir,
        max_depth: maxDepth,
      })
    );
  }

  async readWorkspaceFile(
    workspaceId: string,
    path: string,
    maxBytes?: number
  ): Promise<WorkspaceFileContent> {
    return normalizeWorkspaceFile(
      await this.transport.request<WorkspaceFileContent>('v1.workspace.files.read', {
        workspace_id: workspaceId,
        path,
        max_bytes: maxBytes,
      })
    );
  }

  async writeWorkspaceFile(params: WriteWorkspaceFileParams): Promise<WorkspaceFileContent> {
    return normalizeWorkspaceFile(
      await this.transport.request<WorkspaceFileContent>(
        'v1.workspace.files.write',
        toDaemon(params)
      )
    );
  }

  async getWorkspaceDiff(workspaceId: string, staged?: boolean): Promise<WorkspaceDiffSummary> {
    return fromDaemon<WorkspaceDiffSummary>(
      await this.transport.request<unknown>('v1.workspace.diff', {
        workspace_id: workspaceId,
        staged,
      })
    );
  }

  async getWorkspaceFileDiff(
    workspaceId: string,
    path: string,
    staged?: boolean
  ): Promise<WorkspaceFileDiff> {
    return fromDaemon<WorkspaceFileDiff>(
      await this.transport.request<unknown>('v1.workspace.diff.file', {
        workspace_id: workspaceId,
        path,
        staged,
      })
    );
  }

  async stageWorkspaceFile(
    workspaceId: string,
    path: string
  ): Promise<{ staged: boolean; path: string }> {
    return this.transport.request<{ staged: boolean; path: string }>('v1.workspace.git.stage', {
      workspace_id: workspaceId,
      path,
    });
  }

  async unstageWorkspaceFile(
    workspaceId: string,
    path: string
  ): Promise<{ unstaged: boolean; path: string }> {
    return this.transport.request<{ unstaged: boolean; path: string }>('v1.workspace.git.unstage', {
      workspace_id: workspaceId,
      path,
    });
  }

  async discardWorkspaceFile(
    workspaceId: string,
    path: string
  ): Promise<{ discarded: boolean; path: string }> {
    return this.transport.request<{ discarded: boolean; path: string }>('v1.workspace.git.discard', {
      workspace_id: workspaceId,
      path,
    });
  }

  async commitWorkspace(
    workspaceId: string,
    message: string
  ): Promise<{ committed: boolean; output: string }> {
    return this.transport.request<{ committed: boolean; output: string }>('v1.workspace.git.commit', {
      workspace_id: workspaceId,
      message,
    });
  }

  async listHarnesses(): Promise<HarnessDescriptor[]> {
    return (await this.transport.request<HarnessDescriptor[]>('v1.harness.list', {})).map(
      normalizeHarness
    );
  }

  async getHarness(harnessId: string): Promise<HarnessDescriptor> {
    return normalizeHarness(
      await this.transport.request<HarnessDescriptor>('v1.harness.get', {
        harness_id: harnessId,
      })
    );
  }

  async runNativeHarness(params: RunNativeHarnessParams): Promise<HarnessExecutionResult> {
    return this.transport.request<HarnessExecutionResult>('v1.harness.run_native', {
      harness_id: params.harness_id ?? params.harnessId,
      instruction: params.instruction,
      workspace_id: params.workspace_id ?? params.workspaceId,
      cwd: params.cwd,
    });
  }

  async cancelHarness(harnessId: string, runId: string): Promise<{ cancelled: boolean }> {
    return this.transport.request<{ cancelled: boolean }>('v1.harness.cancel', {
      harness_id: harnessId,
      run_id: runId,
    });
  }

  async steerHarness(
    harnessId: string,
    runId: string,
    guidance: string
  ): Promise<{ steered: boolean }> {
    return this.transport.request<{ steered: boolean }>('v1.harness.steer', {
      harness_id: harnessId,
      run_id: runId,
      guidance,
    });
  }

  async steerWorkspaceAgent(workspaceId: string, instruction: string): Promise<any> {
    return this.transport.request<any>('v1.workspaces.agent_steer', {
      workspace_id: workspaceId,
      instruction,
    });
  }

  async getResourceCapabilities(
    resourceId: string,
    workspaceId?: string
  ): Promise<ResourceActionCapability[]> {
    const list = await this.transport.request<unknown[]>('v1.resources.capabilities', {
      resource_id: resourceId,
      workspace_id: workspaceId,
    });
    return list.map((c) => fromDaemon<ResourceActionCapability>(c));
  }
}
