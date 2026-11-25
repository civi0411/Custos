import type {
  ExecutionWorkspace,
  HarnessDescriptor,
  TerminalSession,
  WorkspaceFileContent,
  DirtyManifest,
} from '@/types/domain';
import type { AnnotationRecord } from '@/types/research';

export const camelKey = (key: string): string => key.replace(/_([a-z])/g, (_, char: string) => char.toUpperCase());
export const snakeKey = (key: string): string => key.replace(/[A-Z]/g, (char) => `_${char.toLowerCase()}`);

export const mapKeysDeep = (value: unknown, transform: (key: string) => string): any => {
  if (Array.isArray(value)) return value.map((item) => mapKeysDeep(item, transform));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, entry]) => [
        transform(key),
        mapKeysDeep(entry, transform),
      ])
    );
  }
  return value;
};

export const fromDaemon = <T,>(value: unknown): T => mapKeysDeep(value, camelKey) as T;
export const toDaemon = (value: unknown): Record<string, any> => mapKeysDeep(value, snakeKey);

export function normalizeTerminalSession(value: TerminalSession): TerminalSession {
  return {
    ...value,
    workspace_id: value.workspace_id ?? value.workspaceId ?? '',
    working_dir: value.working_dir ?? value.workingDir ?? '',
    created_at: value.created_at ?? value.createdAt ?? 0,
    closed_at: value.closed_at ?? value.closedAt,
  };
}

export function normalizeHarness(value: HarnessDescriptor): HarnessDescriptor {
  const profile = value.profile;
  return {
    ...value,
    binary_path: value.binary_path ?? value.binaryPath ?? '',
    is_available: value.is_available ?? value.isAvailable ?? false,
    launch_cmd: value.launch_cmd ?? value.launchCmd,
    profile: {
      ...profile,
      harness_id: profile.harness_id ?? profile.harnessId ?? value.id,
      tool_mediation: profile.tool_mediation ?? profile.toolMediation,
      worktree_ownership: profile.worktree_ownership ?? profile.worktreeOwnership,
      supports_cancel: profile.supports_cancel ?? profile.supportsCancel ?? false,
      supports_steer: profile.supports_steer ?? profile.supportsSteer ?? false,
      cost_visibility: profile.cost_visibility ?? profile.costVisibility,
    },
  };
}

export function annotationFromWire(value: unknown): AnnotationRecord {
  const normalized = fromDaemon<AnnotationRecord>(value);
  const created = normalized.createdAt as unknown;
  return { ...normalized, createdAt: typeof created === 'string' ? Date.parse(created) : Number(created) };
}

export function annotationToWire(annotation: AnnotationRecord): Record<string, unknown> {
  if (!Number.isFinite(annotation.createdAt)) throw new Error('Annotation timestamp is invalid');
  return { ...toDaemon(annotation), created_at: new Date(annotation.createdAt).toISOString() };
}

export function normalizeWorkspaceFile(wire: WorkspaceFileContent): WorkspaceFileContent {
  return {
    ...wire,
    workspace_id: wire.workspace_id ?? wire.workspaceId ?? '',
    is_binary: wire.is_binary ?? wire.isBinary ?? false,
    size_bytes: wire.size_bytes ?? wire.sizeBytes ?? 0,
    line_count: wire.line_count ?? wire.lineCount ?? 0,
  };
}

export function normalizeWorkspace(wire: any): ExecutionWorkspace {
  const failure = wire.status && typeof wire.status === 'object' ? wire.status.setup_failed : null;
  const dirtyManifest = wire.dirty_manifest
    ? (fromDaemon(wire.dirty_manifest) as DirtyManifest)
    : wire.dirtyManifest ?? null;
  const ownerTaskId = wire.owner_task_id ?? wire.ownerTaskId ?? null;
  const baseCommitHash = wire.base_commit_hash ?? wire.baseCommitHash ?? null;
  const runningAgentId = wire.running_agent_id ?? wire.runningAgentId ?? null;
  const agentStatus = wire.agent_status ?? wire.agentStatus ?? null;
  const activeBranch = wire.active_branch ?? wire.activeBranch ?? null;

  return {
    ...wire,
    status: failure ? 'setup_failed' : wire.status,
    status_reason: failure?.reason,
    owner_task_id: ownerTaskId,
    ownerTaskId,
    base_commit_hash: baseCommitHash,
    baseCommitHash,
    dirty_manifest: dirtyManifest,
    dirtyManifest,
    running_agent_id: runningAgentId,
    runningAgentId,
    agent_status: agentStatus,
    agentStatus,
    active_branch: activeBranch,
    activeBranch,
  } as ExecutionWorkspace;
}
