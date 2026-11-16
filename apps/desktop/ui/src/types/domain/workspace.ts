// Workspace and Terminal Domain Types
// Strictly mirrors platform/domain (custos-domain) and apps/daemon (custos-daemon) API

export type WorkspaceKind =
  | { type: 'git'; repo_path: string; branch: string; base_commit?: string }
  | { type: 'folder'; path: string }
  | { type: 'remote_ssh'; host: string; remote_path: string; user?: string; port?: number };

export type WorkspaceStatus = 'initializing' | 'ready' | 'setup_failed' | 'archived';

export interface WorkspaceLineage {
  parent_workspace_id?: string;
  base_commit?: string;
  target_branch?: string;
  head_commit?: string;
}

export interface DirtyManifest {
  is_dirty: boolean;
  isDirty?: boolean;
  modified_files: string[];
  modifiedFiles?: string[];
  untracked_files: string[];
  untrackedFiles?: string[];
  deleted_files: string[];
  deletedFiles?: string[];
  head_commit?: string | null;
  headCommit?: string | null;
  checked_at: number;
  checkedAt?: number;
}

export interface ExecutionWorkspace {
  id: string;
  name: string;
  kind: WorkspaceKind;
  path: string;
  status: WorkspaceStatus;
  status_reason?: string;
  lineage: WorkspaceLineage;
  owner_task_id?: string | null;
  ownerTaskId?: string | null;
  base_commit_hash?: string | null;
  baseCommitHash?: string | null;
  dirty_manifest?: DirtyManifest | null;
  dirtyManifest?: DirtyManifest | null;
  running_agent_id?: string | null;
  runningAgentId?: string | null;
  agent_status?: string | null;
  agentStatus?: string | null;
  active_branch?: string | null;
  activeBranch?: string | null;
  branch?: string | null;
  metadata: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface WorkspaceDetectionResult {
  path: string;
  root_path: string;
  name: string;
  is_git: boolean;
  branch?: string | null;
  is_dirty: boolean;
  dirty_count: number;
  remote_url?: string | null;
  github_host?: string | null;
  github_owner?: string | null;
  github_repo?: string | null;
  github_avatar_url?: string | null;
  project_type: string;
  is_worktree: boolean;
  worktrees?: Array<{
    path: string;
    head?: string | null;
    branch?: string | null;
    is_main: boolean;
    is_bare: boolean;
    is_detached: boolean;
  }>;
  sub_repositories?: Array<{
    name: string;
    path: string;
    branch?: string;
    is_git: boolean;
    is_dirty?: boolean;
    dirty_count?: number;
    remote_url?: string | null;
    github_host?: string | null;
    github_owner?: string | null;
    github_repo?: string | null;
    github_avatar_url?: string | null;
    worktrees?: Array<{
      path: string;
      head?: string | null;
      branch?: string | null;
      is_main: boolean;
      is_bare: boolean;
      is_detached: boolean;
    }>;
  }>;
}

export interface WorkspaceNode {
  id: string;
  name: string;
  path: string;
  kind: 'folder' | 'git' | 'ssh';
  isActive: boolean;
  branch?: string;
  isDirty?: boolean;
  dirtyCount?: number;
  remoteUrl?: string;
  githubOwner?: string;
  githubRepo?: string;
  githubAvatarUrl?: string;
  isWorktree?: boolean;
  worktrees?: WorkspaceNode[]; // child worktrees
}

export interface CreateWorkspaceParams {
  name: string;
  kind: WorkspaceKind;
  path: string;
  lineage?: WorkspaceLineage;
  owner_task_id?: string;
  ownerTaskId?: string;
  metadata?: Record<string, any>;
  setup_script?: string;
}

// ---------------------------------------------------------
// Terminal / Bounded PTY Contracts
// ---------------------------------------------------------

export type TerminalSessionStatus =
  | { type: 'active' }
  | { type: 'exited'; exit_code?: number | null; exitCode?: number | null }
  | { type: 'terminated' };

export interface TerminalSession {
  id: string;
  workspace_id: string;
  workspaceId?: string;
  working_dir: string;
  workingDir?: string;
  command: string;
  cols: number;
  rows: number;
  status: TerminalSessionStatus;
  created_at: number;
  createdAt?: number;
  closed_at?: number | null;
  closedAt?: number | null;
}

export interface TerminalOutputChunk {
  session_id: string;
  sessionId?: string;
  start_seq: number;
  startSeq?: number;
  next_seq: number;
  nextSeq?: number;
  data: string;
  is_eof: boolean;
  isEof?: boolean;
}

export interface SpawnTerminalParams {
  command_id?: string;
  workspace_id: string;
  workspaceId?: string;
  command?: string;
  working_dir?: string;
  workingDir?: string;
  cols?: number;
  rows?: number;
}

// ---------------------------------------------------------
// Workspace Files & Git Diff Contracts
// ---------------------------------------------------------

export interface WorkspaceFileEntry {
  path: string;
  name: string;
  is_dir: boolean;
  isDir?: boolean;
  size_bytes: number;
  sizeBytes?: number;
  modified_at?: number | null;
  modifiedAt?: number | null;
  is_readonly: boolean;
  isReadonly?: boolean;
}

export interface WorkspaceFileTree {
  workspace_id: string;
  workspaceId?: string;
  root_path: string;
  rootPath?: string;
  relative_dir?: string | null;
  relativeDir?: string | null;
  entries: WorkspaceFileEntry[];
  total_files: number;
  totalFiles?: number;
  total_dirs: number;
  totalDirs?: number;
  truncated: boolean;
}

export interface WorkspaceFileContent {
  workspace_id: string;
  workspaceId?: string;
  path: string;
  content: string;
  is_binary: boolean;
  isBinary?: boolean;
  size_bytes: number;
  sizeBytes?: number;
  truncated: boolean;
  line_count: number;
  lineCount?: number;
}

export interface WriteWorkspaceFileParams {
  workspace_id?: string;
  workspaceId?: string;
  path: string;
  content: string;
  create_parents?: boolean;
  createParents?: boolean;
  overwrite?: boolean;
}

export interface WorkspaceDiffEntry {
  path: string;
  old_path?: string | null;
  oldPath?: string | null;
  status: string; // 'M' | 'A' | 'D' | 'R' | '??'
  is_staged: boolean;
  isStaged?: boolean;
  additions: number;
  deletions: number;
}

export interface WorkspaceDiffSummary {
  workspace_id: string;
  workspaceId?: string;
  head_hash?: string | null;
  headHash?: string | null;
  base_hash?: string | null;
  baseHash?: string | null;
  branch?: string | null;
  files: WorkspaceDiffEntry[];
  raw_diff: string;
  rawDiff?: string;
  total_additions: number;
  totalAdditions?: number;
  total_deletions: number;
  totalDeletions?: number;
  is_clean: boolean;
  isClean?: boolean;
}

export interface WorkspaceFileDiff {
  workspace_id: string;
  workspaceId?: string;
  path: string;
  diff: string;
  is_staged: boolean;
  isStaged?: boolean;
}

