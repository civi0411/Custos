use custos_core::contracts::workspace_files::WorkspaceFilesPort;
use custos_domain::{
    ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceStatus, WriteWorkspaceFileParams,
};
use custos_adapters::workspace::LocalWorkspaceFilesAdapter;

#[tokio::test]
async fn inspection_group_test_without_dispatcher() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ws = ExecutionWorkspace::new(
        WorkspaceId::new("ws-test-inspect"),
        "test-inspect",
        WorkspaceKind::Folder {
            path: tmp.path().to_string_lossy().to_string(),
        },
        tmp.path().to_string_lossy().to_string(),
    );
    ws.status = WorkspaceStatus::Ready;

    let coordinator = LocalWorkspaceFilesAdapter::new();

    // Write a test file
    let write_params = WriteWorkspaceFileParams::new(
        ws.id.clone(),
        "hello.txt",
        "Hello, inspection!".to_string(),
    );
    let write_res = coordinator.write_file(&ws, write_params).await.unwrap();
    assert_eq!(write_res.content.len(), 18);

    // Read it back
    let content = coordinator.read_file(&ws, "hello.txt", None).await.unwrap();
    assert_eq!(content.content, "Hello, inspection!");

    // Get file tree
    let tree = coordinator.get_file_tree(&ws, None, None).await.unwrap();
    assert!(tree.entries.iter().any(|e| e.path == "hello.txt"));
}
