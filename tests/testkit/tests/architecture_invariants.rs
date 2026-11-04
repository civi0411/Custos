use std::fs;
use std::path::Path;

#[test]
fn test_domain_zero_io_invariant() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domain_src = manifest_dir.join("../../crates/custos-domain/src");
    assert!(domain_src.exists(), "domain src directory must exist");

    let forbidden_imports = [
        "std::fs",
        "std::net",
        "tokio::fs",
        "tokio::net",
        "reqwest",
        "rusqlite",
    ];

    let mut violations = Vec::new();

    fn walk_dir(dir: &Path, forbidden: &[&str], violations: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, forbidden, violations);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("use ") {
                                for &f in forbidden {
                                    if trimmed.contains(f) {
                                        violations.push(format!("{}: {}", path.display(), line));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    walk_dir(&domain_src, &forbidden_imports, &mut violations);
    assert!(
        violations.is_empty(),
        "Domain Zero-I/O violations detected in custos-domain:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_no_file_folder_name_collisions_in_crates() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir.join("../../crates");
    assert!(crates_dir.exists(), "crates directory must exist");

    let mut collisions = Vec::new();

    fn check_dir(dir: &Path, collisions: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    check_dir(&path, collisions);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if file_name != "mod.rs" && file_name != "lib.rs" {
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        let sibling_dir = path.with_file_name(stem);
                        if sibling_dir.is_dir() {
                            collisions.push(format!(
                                "Collision: file '{}' has matching directory '{}'",
                                path.display(),
                                sibling_dir.display()
                            ));
                        }
                    }
                }
            }
        }
    }

    check_dir(&crates_dir, &mut collisions);
    assert!(
        collisions.is_empty(),
        "Found file/folder name collisions in crates:\n{}",
        collisions.join("\n")
    );
}

#[test]
fn test_domain_bounded_contexts_structure() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domain_src = manifest_dir.join("../../crates/custos-domain/src");

    let expected_contexts = [
        "authority",
        "context",
        "evidence",
        "fleet",
        "oi",
        "research",
        "session",
        "shared",
        "task",
        "workspace",
    ];

    for ctx in &expected_contexts {
        let ctx_dir = domain_src.join(ctx);
        assert!(
            ctx_dir.is_dir(),
            "Expected bounded context directory '{}' does not exist in custos-domain",
            ctx
        );
        let mod_rs = ctx_dir.join("mod.rs");
        assert!(
            mod_rs.exists(),
            "Bounded context '{}' must contain mod.rs",
            ctx
        );
    }
}

#[test]
fn test_core_owns_skill_contract() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let core_skill = manifest_dir.join("../../crates/custos-core/src/contracts/skill.rs");
    assert!(
        core_skill.exists(),
        "custos-core/src/contracts/skill.rs must exist and own the Skill trait contract"
    );

    let content = fs::read_to_string(&core_skill).unwrap();
    assert!(
        content.contains("pub trait Skill"),
        "custos-core must define pub trait Skill"
    );
    assert!(
        content.contains("pub struct SkillRegistry"),
        "custos-core must define pub struct SkillRegistry"
    );
}

