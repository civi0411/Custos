use std::fs;
use std::path::Path;

#[test]
fn test_domain_zero_io_invariant() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contracts_src = manifest_dir.join("../../platform/contracts/src");
    assert!(contracts_src.exists(), "contracts src directory must exist");

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

    walk_dir(&contracts_src, &forbidden_imports, &mut violations);
    assert!(
        violations.is_empty(),
        "Domain Zero-I/O violations detected in custos-contracts:\n{}",
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
fn test_contracts_bounded_contexts_structure() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contracts_src = manifest_dir.join("../../platform/contracts/src");

    let expected_contexts = [
        "authority",
        "context",
        "contributions",
        "conversation",
        "evidence",
        "execution",
        "identity",
        "inference",
        "resources",
        "task",
    ];

    for ctx in &expected_contexts {
        let ctx_dir = contracts_src.join(ctx);
        assert!(
            ctx_dir.is_dir(),
            "Expected bounded context directory '{}' does not exist in custos-contracts",
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
fn test_contracts_owns_skill_contract() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill_contract = manifest_dir.join("../../platform/contracts/src/contributions/skill.rs");
    assert!(
        skill_contract.exists(),
        "custos-contracts/src/contributions/skill.rs must exist and own the Skill trait contract"
    );

    let content = fs::read_to_string(&skill_contract).unwrap();
    assert!(
        content.contains("pub trait Skill"),
        "custos-contracts must define pub trait Skill"
    );
    assert!(
        content.contains("pub trait SkillSink"),
        "custos-contracts must define pub trait SkillSink"
    );
    let runtime_skills = fs::read_to_string(
        manifest_dir.join("../../platform/engine/src/execution/skills/mod.rs"),
    )
    .unwrap();
    assert!(
        runtime_skills.contains("pub struct SkillRegistry"),
        "custos-engine must define pub struct SkillRegistry"
    );
}

#[test]
fn test_platform_four_blocks_and_workbench_isolation() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..");
    let platform = root.join("platform");

    // 1. Legacy shims must be completely eradicated from disk
    let legacy_crates = ["domain", "core", "runtime", "components", "adapters", "ai"];
    for legacy in &legacy_crates {
        assert!(
            !platform.join(legacy).exists(),
            "Legacy directory platform/{legacy} must not exist on disk"
        );
    }

    // 2. Exactly the 4 canonical platform blocks must exist
    let canonical_blocks = ["contracts", "storage", "integrations", "engine"];
    for block in &canonical_blocks {
        assert!(
            platform.join(block).is_dir(),
            "Canonical platform block platform/{block} must exist"
        );
    }

    // 3. platform/contracts must never declare workbench modules directly
    let contracts_lib = fs::read_to_string(root.join("platform/contracts/src/lib.rs")).unwrap();
    assert!(!contracts_lib.contains("pub mod quant;"), "platform/contracts must not contain pub mod quant");
    assert!(!contracts_lib.contains("pub mod research;"), "platform/contracts must not contain pub mod research");

    // 4. platform/storage migrations must not contain decoupled research migrations
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

    // 5. platform/storage must not contain assistant repository or migration
    assert!(
        !root.join("platform/storage/src/repositories/assistant.rs").exists(),
        "platform/storage must not contain assistant repository (belongs in workbenches/assistant)"
    );
    assert!(
        !storage_migrations.join("0030_assistant_drafts.sql").exists(),
        "platform/storage/migrations/0030_assistant_drafts.sql must not exist (belongs in workbenches/assistant)"
    );

    // 6. platform/engine/Cargo.toml must have zero workbench dependencies
    let engine_cargo = fs::read_to_string(root.join("platform/engine/Cargo.toml")).unwrap();
    assert!(
        !engine_cargo.contains("custos-workbench-"),
        "platform/engine/Cargo.toml must not depend on any custos-workbench-* package"
    );
}
