use super::*;
use custos_domain::{WorkspaceId, WorkspaceKind};
use tokio::fs;

#[tokio::test]
async fn test_resolve_safe_path_rejects_traversal() {
    let temp_dir = std::env::temp_dir().join("custos_safe_path_test");
    let _ = fs::create_dir_all(&temp_dir).await;

    let err = resolve_safe_path(&temp_dir, "../outside.txt").unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));

    let err = resolve_safe_path(&temp_dir, "foo/../../outside.txt").unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));

    let ok = resolve_safe_path(&temp_dir, "src/main.rs").unwrap();
    assert_eq!(ok, temp_dir.join("src/main.rs"));

    let _ = fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn test_file_read_write_lifecycle() {
    let test_dir =
        std::env::temp_dir().join(format!("custos_ws_file_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&test_dir).await;

    let ws = ExecutionWorkspace::new(
        WorkspaceId::generate(),
        "test_ws".to_string(),
        WorkspaceKind::Folder {
            path: test_dir.to_string_lossy().to_string(),
        },
        test_dir.to_string_lossy().to_string(),
    );

    let coord = LocalWorkspaceFilesAdapter::new();

    // 1. Write file with parent creation
    let write_params = WriteWorkspaceFileParams::new(
        ws.id.clone(),
        "nested/dir/hello.txt",
        "Hello Custos Workspace File API!\nLine 2\n",
    );
    let written = coord
        .write_file(&ws, write_params)
        .await
        .expect("write file");
    assert_eq!(written.path, "nested/dir/hello.txt");
    assert_eq!(written.line_count, 2);
    assert!(!written.is_binary);

    // 2. Read file back
    let read = coord
        .read_file(&ws, "nested/dir/hello.txt", None)
        .await
        .expect("read file");
    assert_eq!(read.content, "Hello Custos Workspace File API!\nLine 2\n");
    assert_eq!(read.line_count, 2);

    // 3. Get file tree
    let tree = coord
        .get_file_tree(&ws, None, None)
        .await
        .expect("file tree");
    assert!(tree.total_files >= 1);
    assert!(tree
        .entries
        .iter()
        .any(|e| e.path == "nested/dir/hello.txt"));

    // 4. Test binary file read encodes base64
    let binary_path = test_dir.join("sample.bin");
    let sample_bytes = vec![0u8, 1, 2, 3, 255, 0, 42];
    fs::write(&binary_path, &sample_bytes)
        .await
        .expect("write binary");
    let read_bin = coord
        .read_file(&ws, "sample.bin", None)
        .await
        .expect("read binary");
    assert!(read_bin.is_binary);
    assert!(!read_bin.content.is_empty());
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&read_bin.content)
        .expect("valid base64");
    assert_eq!(decoded, sample_bytes);

    // An ASCII PDF header and non-UTF8 binary must survive byte-for-byte.
    for (name, bytes) in [
        (
            "paper.pdf",
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n%%EOF".as_slice(),
        ),
        ("non-null.bin", &[0xff, 0xfe, 0x80, 1, 2]),
    ] {
        fs::write(test_dir.join(name), bytes).await.unwrap();
        let read = coord.read_file(&ws, name, None).await.unwrap();
        assert!(read.is_binary, "{name}");
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(read.content)
                .unwrap(),
            bytes
        );
    }
    fs::write(test_dir.join("large.pdf"), b"%PDF-1.7\ntruncated payload")
        .await
        .unwrap();
    let limited = coord.read_file(&ws, "large.pdf", Some(8)).await.unwrap();
    assert!(limited.truncated && limited.is_binary);
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(limited.content)
            .unwrap()
            .len(),
        8
    );
    fs::write(test_dir.join("utf8.txt"), "abcédef")
        .await
        .unwrap();
    let utf8 = coord.read_file(&ws, "utf8.txt", Some(4)).await.unwrap();
    assert!(!utf8.is_binary && utf8.truncated);
    assert_eq!(utf8.content, "abc");

    let _ = fs::remove_dir_all(&test_dir).await;
}
