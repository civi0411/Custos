#[test]
fn test_candidate_command_id_validation() {
    let valid_id = "cmd-123_abc";
    assert!(
        !valid_id.is_empty()
            && valid_id.len() <= 128
            && valid_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    );

    let invalid_id = "cmd with spaces!";
    assert!(!invalid_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')));
}
