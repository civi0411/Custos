//! Workspace inspection and file operations use cases over WorkspaceFilesPort and RepositoryGitPort.

pub mod changes;
pub mod detect;
pub mod files;
pub mod repository_git;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspaces_inspection(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACES_DETECT => self.detect_workspace(req).await,
            METHOD_GIT_REPOSITORY
            | METHOD_GIT_SWITCH
            | METHOD_GIT_FETCH
            | METHOD_GIT_ACCOUNT
            | METHOD_GIT_GITHUB => self.handle_repository_git(req).await,
            METHOD_WORKSPACE_FILES_TREE
            | METHOD_WORKSPACE_FILES_READ
            | METHOD_WORKSPACE_FILES_WRITE => self.handle_workspace_files(req).await,
            METHOD_WORKSPACE_DIFF
            | METHOD_WORKSPACE_FILE_DIFF
            | METHOD_WORKSPACE_GIT_STAGE
            | METHOD_WORKSPACE_GIT_UNSTAGE
            | METHOD_WORKSPACE_GIT_DISCARD
            | METHOD_WORKSPACE_GIT_COMMIT => self.handle_workspace_changes(req).await,
            unknown => ApiResponse::error(req.id, format!("Unknown inspection method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
