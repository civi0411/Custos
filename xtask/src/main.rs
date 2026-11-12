use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{cmd, Shell};

fn source_capabilities(root: &std::path::Path) -> Result<std::collections::BTreeSet<String>> {
    let adapters = root.join("platform/adapters/src");
    let ai = root.join("platform/ai/src");
    let mut exports = std::fs::read_to_string(adapters.join("lib.rs"))?;
    if let Ok(ai_exports) = std::fs::read_to_string(ai.join("lib.rs")) {
        exports.push('\n');
        exports.push_str(&ai_exports);
    }
    let mut ids = std::collections::BTreeSet::new();

    fn collect_manifests(
        dir: &std::path::Path,
        manifests: &mut Vec<std::path::PathBuf>,
    ) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.join("manifest.json").is_file() {
                    manifests.push(path.join("manifest.json"));
                }
                collect_manifests(&path, manifests)?;
            }
        }
        Ok(())
    }

    let mut manifests = Vec::new();
    collect_manifests(&adapters, &mut manifests)?;
    collect_manifests(&ai, &mut manifests)?;

    for manifest in manifests {
        let parent = manifest.parent().unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest)?)?;
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|id| !id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Missing capability ID: {}", manifest.display()))?;
        let module = parent.file_name().unwrap().to_string_lossy().into_owned();
        if !parent.join("mod.rs").is_file()
            || !exports
                .lines()
                .any(|line| line.contains(&module))
        {
            anyhow::bail!(
                "Descriptor has no exported adapter module: {}",
                manifest.display()
            );
        }
        if !ids.insert(id.to_owned()) {
            anyhow::bail!("Duplicate capability ID: {id}");
        }
    }
    if ids.is_empty() {
        anyhow::bail!(
            "No source capability descriptors found in {}",
            adapters.display()
        );
    }
    Ok(ids)
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the workspace for formatting and linting
    Check,
    /// Run end-to-end tests
    TestE2e,
    /// Validate all workbench packages and capability manifests
    ValidatePacks,
    /// Package a workbench bundle
    Package {
        /// Target workbench name (e.g. research, quant, coding, assistant)
        workbench: Option<String>,
    },
    /// Run canonical quality gates (Desktop build, Canvas Smoke, Architecture Invariants, Workspace Check, Packs Validation)
    Gate,
    /// Check architectural boundaries and track kernel-workbench leaks
    CheckBoundaries,
    /// Run SADE R7 empirical footprint, memory, disk, and admission latency benchmark
    BenchmarkFootprint,
}


fn validate_packs() -> Result<()> {
    use std::fs;
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot find repo root"))?;

    println!("Checking workbench manifests & migrations...");
    let capabilities = source_capabilities(root)?;
    let workbenches = ["assistant", "coding", "research", "quant"];
    for wb in &workbenches {
        let manifest_path = root.join(format!("workbenches/{wb}/manifest/manifest.json"));
        if !manifest_path.exists() {
            anyhow::bail!("Missing workbench manifest: {:?}", manifest_path);
        }
        let content = fs::read_to_string(&manifest_path)?;
        let val: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in {:?}: {}", manifest_path, e))?;
        if val.get("id").and_then(|v| v.as_str()) != Some(*wb) {
            anyhow::bail!("Manifest id mismatch in {:?}", manifest_path);
        }
        let required = val
            .get("capabilities")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                anyhow::anyhow!("Missing capability list in {}", manifest_path.display())
            })?;
        for requirement in required {
            let id = requirement.as_str().ok_or_else(|| {
                anyhow::anyhow!("Non-string capability in {}", manifest_path.display())
            })?;
            if !capabilities.contains(id) {
                anyhow::bail!("Workbench {wb} requires unknown source capability: {id}");
            }
        }
        let migrations_dir = root.join(format!("workbenches/{wb}/migrations"));
        if !migrations_dir.exists() {
            anyhow::bail!("Missing migrations directory: {:?}", migrations_dir);
        }
        println!("  ✓ Workbench '{}' manifest & migrations valid", wb);
    }

    println!(
        "Validated {} source capability descriptors; installation/readiness is not assessed.",
        capabilities.len()
    );
    Ok(())
}

fn package_workbench(target: Option<String>) -> Result<()> {
    validate_packs()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot find repo root"))?;
    let sh = Shell::new()?;
    let _dir = sh.push_dir(root);

    let workbenches: Vec<&str> = match target.as_deref() {
        Some(id) => {
            if !["assistant", "coding", "research", "quant"].contains(&id) {
                anyhow::bail!("Unknown workbench: {id}");
            }
            vec![id]
        }
        None => vec!["assistant", "coding", "research", "quant"],
    };

    let dist = root.join("dist/workbenches");
    std::fs::create_dir_all(&dist)?;

    for wb in workbenches {
        let wb_dir = root.join(format!("workbenches/{wb}"));
        let manifest_path = wb_dir.join("manifest/manifest.json");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;
        let version = manifest.get("version").and_then(|v| v.as_str()).unwrap_or("0.1.0");

        let out_archive = dist.join(format!("{wb}-{version}.tar.gz"));
        let out_manifest = dist.join(format!("{wb}-{version}-manifest.json"));
        std::fs::copy(&manifest_path, &out_manifest)?;

        cmd!(sh, "tar -czf {out_archive} -C {wb_dir} .").run()?;

        let sha_file = dist.join(format!("{wb}-{version}.sha256"));
        let sh_cmd = format!("shasum -a 256 {} > {}", out_archive.display(), sha_file.display());
        cmd!(sh, "sh -c {sh_cmd}").run()?;

        let metadata = std::fs::metadata(&out_archive)?;
        println!(
            "  ✓ Successfully packaged independent workbench '{}' v{} ({} bytes) -> {}",
            wb,
            version,
            metadata.len(),
            out_archive.display()
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot find repo root"))?;
    let _directory = sh.push_dir(root);

    match cli.command {
        Commands::Check => {
            cmd!(sh, "cargo fmt --all -- --check").run()?;
            cmd!(sh, "cargo clippy --workspace -- -D warnings").run()?;
        }
        Commands::TestE2e => {
            cmd!(sh, "cargo test -p custos-tests-e2e --offline").run()?;
        }
        Commands::ValidatePacks => {
            validate_packs()?;
        }
        Commands::Package { workbench } => {
            package_workbench(workbench)?;
        }
        Commands::Gate => {
            println!("==> [1/5] Workbench & Capability Manifest Invariants");
            validate_packs()?;
            println!("==> [2/5] Desktop UI Build & Typecheck");
            cmd!(sh, "npm --prefix ui/desktop run build").run()?;
            println!("==> [3/5] Canvas Smoke Invariants");
            cmd!(sh, "node --test tests/desktop/canvas-smoke.mjs tests/desktop/model-catalog-smoke.mjs xtask/scripts/checks-smoke.mjs").run()?;
            println!("==> [4/5] Architecture Invariants & Contracts");
            cmd!(sh, "bash xtask/scripts/check_deps.sh").run()?;
            cmd!(sh, "bash xtask/scripts/check_layering.sh").run()?;
            cmd!(sh, "bash xtask/scripts/check_naming.sh").run()?;
            check_file_line_ceiling(&sh)?;
            cmd!(
                sh,
                "cargo test -p custos-testkit -p custos-tests-contract --offline"
            )
            .run()?;
            println!("==> [5/5] Workspace Cargo Check");
            cmd!(sh, "cargo check --workspace --all-targets --offline").run()?;
            println!("==> All Quality Gates Passed cleanly!");
        }
        Commands::CheckBoundaries => {
            check_boundaries()?;
        }
        Commands::BenchmarkFootprint => {
            benchmark_footprint(&sh, root)?;
        }
    }

    Ok(())
}

fn benchmark_footprint(sh: &Shell, root: &std::path::Path) -> Result<()> {
    println!("==> Running SADE R7 Empirical Footprint Benchmark (tests/e2e/tests/sade_r7_footprint_benchmark.rs)...");
    cmd!(sh, "cargo test -p custos-tests-e2e --test sade_r7_footprint_benchmark --offline -- --nocapture").run()?;

    println!("\n==> [Disk Footprint Measurement]");
    let workbenches = ["assistant", "coding", "research", "quant"];
    let mut total_wb_bytes = 0u64;

    for wb in &workbenches {
        let wb_dir = root.join(format!("workbenches/{wb}"));
        let mut wb_bytes = 0u64;
        fn dir_size(dir: &std::path::Path, total: &mut u64) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(meta) = path.metadata() {
                            *total += meta.len();
                        }
                    } else if path.is_dir() {
                        dir_size(&path, total);
                    }
                }
            }
        }
        dir_size(&wb_dir, &mut wb_bytes);
        total_wb_bytes += wb_bytes;
        println!("  • Workbench '{wb}' source & asset footprint: {:.2} KB ({wb_bytes} bytes)", wb_bytes as f64 / 1024.0);
    }
    println!("  • Combined Multi-Workbench Disk Footprint: {:.2} KB ({total_wb_bytes} bytes)", total_wb_bytes as f64 / 1024.0);

    let dist_dir = root.join("dist/workbenches");
    if dist_dir.exists() {
        println!("\n==> [Packaged Bundle Footprint (dist/workbenches)]");
        for wb in &workbenches {
            if let Ok(entries) = std::fs::read_dir(&dist_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(wb) && name.ends_with(".tar.gz") {
                        if let Ok(meta) = entry.metadata() {
                            let size = meta.len();
                            println!("  • Archive '{name}': {:.2} KB ({size} bytes)", size as f64 / 1024.0);
                        }
                    }
                }
            }
        }
    }
    println!("\n==> SADE R7 Empirical Footprint & Latency Benchmark completed successfully!");
    Ok(())
}

fn check_boundaries() -> Result<()> {
    use std::fs;
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot find repo root"))?;

    println!("Checking Architecture Boundaries & Layer Invariants...");

    let sh = Shell::new()?;
    let _dir = sh.push_dir(root);
    cmd!(sh, "bash xtask/scripts/check_deps.sh").run()?;
    cmd!(sh, "bash xtask/scripts/check_layering.sh").run()?;

    println!("\nChecking Target Architecture Debt (Kernel-Workbench isolation):");
    let mut warnings = 0;

    let domain_lib = fs::read_to_string(root.join("platform/domain/src/lib.rs"))?;
    if domain_lib.contains("pub mod quant;") {
        println!("  ⚠ [LEAK] platform/domain/src/lib.rs declares 'pub mod quant;'");
        warnings += 1;
    }
    if domain_lib.contains("pub mod research;") {
        println!("  ⚠ [LEAK] platform/domain/src/lib.rs declares 'pub mod research;'");
        warnings += 1;
    }

    if root.join("platform/domain/src/evidence/claim.rs").exists() {
        println!("  ⚠ [LEAK] platform/domain/src/evidence/claim.rs must reside in custos-workbench-research");
        warnings += 1;
    }
    if root.join("platform/domain/src/evidence/synthesis.rs").exists() {
        println!("  ⚠ [LEAK] platform/domain/src/evidence/synthesis.rs must reside in custos-workbench-research");
        warnings += 1;
    }

    let core_contracts = root.join("platform/core/src/contracts");
    for wb_contract in &["quant.rs", "research.rs", "research_ingress.rs"] {
        if core_contracts.join(wb_contract).exists() {
            println!("  ⚠ [LEAK] platform/core/src/contracts/{wb_contract} contains workbench-specific contract");
            warnings += 1;
        }
    }

    let storage_migrations = root.join("platform/storage/migrations");
    for migration in &[
        "0017_research_lineage_and_claims.sql",
        "0018_research_recipes_and_annotations.sql",
        "0020_notes_and_artifact_registry.sql",
        "0021_notebook_cells_and_kernels.sql",
        "0022_reviewer_records.sql",
        "0023_research_synthesis_proposals.sql",
        "0035_research_execution_snapshot.sql",
        "0041_notebook_document_binding.sql",
        "0042_quant_live_pipeline.sql",
        "0030_assistant_drafts.sql",
    ] {
        if storage_migrations.join(migration).exists() {
            println!("  ⚠ [LEAK] platform/storage/migrations/{migration} contains workbench domain tables in core DB");
            warnings += 1;
        }
    }

    if root.join("platform/storage/src/repositories/assistant.rs").exists() {
        println!("  ⚠ [LEAK] platform/storage/src/repositories/assistant.rs exists in core storage");
        warnings += 1;
    }

    if root.join("platform/components/src/builtins").exists() {
        println!("  ⚠ [LEAK] platform/components/src/builtins directory exists in platform");
        warnings += 1;
    }

    let components_cargo = fs::read_to_string(root.join("platform/components/Cargo.toml"))?;
    if components_cargo.contains("custos-workbench-") {
        println!("  ⚠ [LEAK] platform/components/Cargo.toml has direct dependencies on workbench crates");
        warnings += 1;
    }

    if warnings == 0 {
        println!("  ✓ Zero kernel-workbench leaks detected! Clean target architecture achieved.");
    } else {
        println!("\n  Found {warnings} transitional architecture debt warnings (tracked for resolution in R3..R6).");
    }

    println!("\nChecking SADE Topic-Cohesion Invariants (Wave 1 & Wave 2):");
    let mut violations: Vec<String> = Vec::new();
    for (rel, rule) in [
        ("platform/core/src/oi", "OI logic must reside in platform/runtime/src/oi (only OiPlannerPort stays in core/contracts/oi.rs)"),
        ("platform/core/src/context", "ContextCompiler pack pipeline must reside in platform/runtime/src/context/pack"),
        ("platform/core/src/contracts/local_api", "Neutral wire DTOs belong in sdk/protocol/src/local_api"),
        ("platform/domain/src/pipeline/modality_invariants.rs", "Ring 3 concrete pipeline invariants belong in owning workbenches"),
        ("platform/ai/src/auth", "Vault and OAuth flows belong in platform/adapters/src/{host/vault,connectors/oauth}"),
        ("platform/ai/src/contracts/catalog", "Catalog service and models belong in platform/ai/src/catalog"),
        ("platform/ai/src/contracts/types/canonical", "Canonical model mapping belongs in platform/ai/src/catalog/canonical"),
        ("platform/ai/src/contracts/types/formats", "Provider wire codecs must reside in platform/ai/src/cloud/codecs"),
        ("platform/runtime/src/cognitive", "Legacy cognitive/ module must reside under platform/runtime/src/agent"),
        ("platform/runtime/src/agent/skills/fs", "Host FS skill must reside in platform/adapters/src/host/operations/fs"),
        ("platform/runtime/src/agent/skills/http", "Host HTTP skill must reside in platform/adapters/src/host/operations/http"),
        ("platform/runtime/src/agent/skills/process", "Host process skill must reside in platform/adapters/src/host/operations/process"),
        ("platform/runtime/src/agent/skills/search", "Host search skill must reside in platform/adapters/src/host/operations/search"),
        ("platform/runtime/src/workspace", "Workspace orchestration belongs in platform/runtime/src/environment/workspace (files adapter in platform/adapters/src/host/workspace/files)"),
        ("platform/runtime/src/terminal", "Terminal orchestration belongs in platform/runtime/src/environment/terminal"),
        ("platform/runtime/src/notebook", "Notebook orchestration belongs in platform/runtime/src/environment/notebook"),
        ("platform/runtime/src/compute", "Compute orchestration belongs in platform/runtime/src/environment/compute"),
        ("platform/runtime/src/bridge", "Session bridge belongs in platform/runtime/src/session/bridge"),
        ("platform/adapters/src/host/execution", "Compute and notebook drivers belong in platform/adapters/src/{connectors/compute,host/notebook}"),
        ("platform/adapters/src/host/harness", "Harnesses belong in platform/adapters/src/harnesses with data/harnesses.json"),
        ("platform/components/src/engine", "Package installer/inventory moved to adapters and OI benchmark to runtime"),
        ("platform/components/src/dynamic_registry", "Dynamic component registry flattened into platform/components/src/registry.rs"),
        ("apps/daemon/src/api/transport.rs", "Host binary/git helpers belong in platform/adapters/src/host"),
        ("apps/daemon/src/api/tasks.rs", "Task handler belongs in apps/daemon/src/application/tasks.rs"),
        ("apps/daemon/src/api/oi.rs", "OI handler belongs in apps/daemon/src/application/oi.rs"),
        ("apps/cli/build.rs", "apps/cli is a headless pure-Rust CLI without Tauri build hooks"),
        ("apps/cli/tauri.conf.json", "apps/cli is a headless pure-Rust CLI without Tauri config"),
        ("apps/cli/icons", "apps/cli is a headless pure-Rust CLI without Tauri icon assets"),
        ("apps/cli/capabilities", "apps/cli is a headless pure-Rust CLI without Tauri capability manifests"),
        ("apps/cli/src/frame-ui", "apps/cli is a headless pure-Rust CLI without Tauri frame assets"),
    ] {
        if root.join(rel).exists() {
            violations.push(format!("{rel} exists: {rule}"));
        }
    }
    if !root.join("sdk/protocol/Cargo.toml").exists() {
        violations.push("sdk/protocol/Cargo.toml missing: custos-protocol must own neutral wire schemas".into());
    }
    for (manifest, forbidden, rule) in [
        ("platform/core/Cargo.toml", "custos-protocol", "platform/core must stay transport-agnostic and never depend on custos-protocol"),
        ("platform/ai/Cargo.toml", "keyring", "platform/ai must stay pure model/provider logic without OS keychain crates"),
        ("platform/runtime/Cargo.toml", "reqwest", "platform/runtime must not depend on reqwest"),
        ("platform/components/Cargo.toml", "reqwest", "platform/components must stay pure pack registry/verifier without reqwest"),
        ("apps/cli/Cargo.toml", "tauri", "apps/cli must stay a headless pure-Rust CLI without tauri"),
    ] {
        let cargo_text = fs::read_to_string(root.join(manifest))?;
        if cargo_text.contains(forbidden) {
            violations.push(format!("{rule} ({manifest} references '{forbidden}')"));
        }
    }
    if violations.is_empty() {
        println!("  ✓ All T1–T5 topic-cohesion invariants verified.");
    } else {
        for v in &violations {
            eprintln!("  ✗ [VIOLATION] {v}");
        }
        anyhow::bail!("Topic-cohesion boundary check failed ({} violation(s))", violations.len());
    }

    check_file_line_ceiling(&sh)?;

    Ok(())
}

fn check_file_line_ceiling(sh: &Shell) -> Result<()> {
    println!("\nChecking Source File Line Ceiling (<= 600 lines for rs, ts, tsx, css)...");
    let files = cmd!(sh, "git ls-files '*.rs' '*.ts' '*.tsx' '*.css'").read()?;
    let allowlist: [&str; 0] = [];

    let mut violations = Vec::new();
    for file in files.lines().filter(|l| !l.is_empty()) {
        if allowlist.contains(&file) {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(file) {
            let line_count = content.lines().count();
            if line_count > 600 {
                violations.push((file.to_string(), line_count));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("  FAIL: Found {} files exceeding the 600-line ceiling:", violations.len());
        for (f, count) in &violations {
            eprintln!("    - {f}: {count} lines");
        }
        anyhow::bail!("Source line ceiling invariant violated (must be <= 600 lines per file)");
    }
    println!("  ✓ 100% of tracked source files comply with the <= 600 line ceiling.");
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_inventory_rejects_missing_modules_and_duplicate_ids() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "custos-source-capabilities-{}-{unique}",
            std::process::id()
        ));
        let adapters = root.join("platform/adapters/src");
        std::fs::create_dir_all(adapters.join("first")).unwrap();
        std::fs::write(adapters.join("lib.rs"), "pub mod first;\npub mod second;\n").unwrap();
        std::fs::write(adapters.join("first/manifest.json"), r#"{"id":"shared"}"#).unwrap();
        assert!(
            source_capabilities(&root).is_err(),
            "metadata without implementation must fail"
        );
        std::fs::write(adapters.join("first/mod.rs"), "").unwrap();
        assert_eq!(source_capabilities(&root).unwrap().len(), 1);
        std::fs::create_dir(adapters.join("second")).unwrap();
        std::fs::write(adapters.join("second/mod.rs"), "").unwrap();
        std::fs::write(adapters.join("second/manifest.json"), r#"{"id":"shared"}"#).unwrap();
        assert!(source_capabilities(&root)
            .unwrap_err()
            .to_string()
            .contains("Duplicate"));
        std::fs::write(adapters.join("second/manifest.json"), r#"{"id":""}"#).unwrap();
        assert!(source_capabilities(&root)
            .unwrap_err()
            .to_string()
            .contains("Missing capability ID"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_command_never_claims_an_unbuilt_artifact() {
        assert!(package_workbench(Some("research".into())).is_ok());
        assert!(package_workbench(Some("unknown".into()))
            .unwrap_err()
            .to_string()
            .contains("Unknown workbench"));
    }
}
