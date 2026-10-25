//! Workspaces application use cases organized by domain submodules:
//! - lifecycle: create, get, list, archive, recover, inspect dirty, pick folder, steer
//! - inspection: detect, file tree, read, write, diff, git status/switch/fetch/account
//! - changes: candidates spawn, compare, integrate
//! - github: accounts, login, auth status, viewer

pub mod changes;
pub mod github;
pub mod inspection;
pub mod lifecycle;

use crate::api::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_workspaces(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACES_CREATE
            | METHOD_WORKSPACES_GET
            | METHOD_WORKSPACES_LIST
            | METHOD_WORKSPACES_ARCHIVE
            | METHOD_WORKSPACES_INSPECT_DIRTY
            | METHOD_WORKSPACES_RECOVER
            | METHOD_WORKSPACES_PICK_FOLDER
            | METHOD_WORKSPACES_AGENT_STEER => self.handle_workspaces_lifecycle(req).await,

            METHOD_WORKSPACES_DETECT
            | METHOD_WORKSPACE_FILES_TREE
            | METHOD_WORKSPACE_FILES_READ
            | METHOD_WORKSPACE_FILES_WRITE
            | METHOD_WORKSPACE_DIFF
            | METHOD_WORKSPACE_FILE_DIFF
            | METHOD_WORKSPACE_GIT_STAGE
            | METHOD_WORKSPACE_GIT_UNSTAGE
            | METHOD_WORKSPACE_GIT_DISCARD
            | METHOD_WORKSPACE_GIT_COMMIT
            | METHOD_GIT_REPOSITORY
            | METHOD_GIT_SWITCH
            | METHOD_GIT_FETCH
            | METHOD_GIT_ACCOUNT
            | METHOD_GIT_GITHUB => self.handle_workspaces_inspection(req).await,

            METHOD_WORKSPACES_CANDIDATES_SPAWN
            | METHOD_WORKSPACES_CANDIDATES_COMPARE
            | METHOD_WORKSPACES_INTEGRATE => self.handle_workspaces_changes(req).await,

            METHOD_GITHUB_ACCOUNTS
            | METHOD_GITHUB_LOGIN
            | METHOD_GITHUB_AUTH_STATUS
            | METHOD_GITHUB_VIEWER => self.handle_workspaces_github(req).await,

            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
