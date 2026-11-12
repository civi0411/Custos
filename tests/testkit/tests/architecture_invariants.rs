use std::fs;
use std::path::Path;

#[test]
fn test_domain_zero_io_invariant() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domain_src = manifest_dir.join("../../platform/domain/src");
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
    let platform_dir = manifest_dir.join("../../platform");
    assert!(platform_dir.exists(), "platform directory must exist");

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

    check_dir(&platform_dir, &mut collisions);
    check_dir(&manifest_dir.join("../../apps"), &mut collisions);
    check_dir(&manifest_dir.join("../../sdk"), &mut collisions);
    assert!(
        collisions.is_empty(),
        "Found file/folder name collisions in crates:\n{}",
        collisions.join("\n")
    );
}

#[test]
fn test_domain_bounded_contexts_structure() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let domain_src = manifest_dir.join("../../platform/domain/src");

    let expected_contexts = [
        "authority",
        "context",
        "evidence",
        "fleet",
        "oi",
        "pipeline",
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
    let core_skill = manifest_dir.join("../../platform/core/src/contracts/skill.rs");
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
        content.contains("pub trait SkillSink"),
        "custos-core must define pub trait SkillSink"
    );
    let runtime_skills = fs::read_to_string(
        manifest_dir.join("../../platform/runtime/src/agent/skills/mod.rs"),
    )
    .unwrap();
    assert!(
        runtime_skills.contains("pub struct SkillRegistry"),
        "custos-runtime must define pub struct SkillRegistry"
    );
}

#[test]
fn test_platform_kernel_workbench_isolation() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..");

    // 1. platform/domain must never declare workbench modules directly
    let domain_lib = fs::read_to_string(root.join("platform/domain/src/lib.rs")).unwrap();
    assert!(!domain_lib.contains("pub mod quant;"), "platform/domain must not contain pub mod quant");
    assert!(!domain_lib.contains("pub mod research;"), "platform/domain must not contain pub mod research");

    // 2. platform/core contracts must not contain workbench-specific contracts
    let core_contracts = root.join("platform/core/src/contracts");
    for wb_contract in &["quant.rs", "research.rs", "research_ingress.rs"] {
        assert!(
            !core_contracts.join(wb_contract).exists(),
            "platform/core/src/contracts/{wb_contract} must not exist (belongs in workbenches)"
        );
    }

    // 3. platform/storage migrations must not contain decoupled research migrations
    let storage_migrations = root.join("platform/storage/migrations");
    let decoupled = [
        "0017_research_lineage_and_claims.sql",
        "0018_research_recipes_and_annotations.sql",
        "0020_notes_and_artifact_registry.sql",
        "0021_notebook_cells_and_kernels.sql",
        "0022_reviewer_records.sql",
        "0023_research_synthesis_proposals.sql",
        "0035_research_execution_snapshot.sql",
        "0041_research_evaluation_reports.sql",
    ];
    for mig in &decoupled {
        assert!(
            !storage_migrations.join(mig).exists(),
            "platform/storage/migrations/{mig} must not exist in core storage"
        );
    }

    // 4. platform/storage must not contain assistant repository or migration
    assert!(
        !root.join("platform/storage/src/repositories/assistant.rs").exists(),
        "platform/storage must not contain assistant repository (belongs in workbenches/assistant)"
    );
    assert!(
        !storage_migrations.join("0030_assistant_drafts.sql").exists(),
        "platform/storage/migrations/0030_assistant_drafts.sql must not exist (belongs in workbenches/assistant)"
    );

    // 5. platform/components must not contain builtins directory
    assert!(
        !root.join("platform/components/src/builtins").exists(),
        "platform/components must not contain builtins directory (workbenches are standalone crates)"
    );

    // 6. platform/components/Cargo.toml must have zero workbench dependencies
    let components_cargo = fs::read_to_string(root.join("platform/components/Cargo.toml")).unwrap();
    assert!(
        !components_cargo.contains("custos-workbench-"),
        "platform/components/Cargo.toml must not depend on any custos-workbench-* package"
    );
}

