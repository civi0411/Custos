export interface WorktreeManagerPaneProps {
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenWorkspaceResource?: (resource: 'files' | 'terminal' | 'agents', workspaceId: string) => void;
}

export interface CandidateComparison {
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
  inspection_error?: string;
  dirty_error?: string;
  diff_error?: string;
  is_ready_for_integration?: boolean;
}
