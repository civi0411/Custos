//! Schema Compatibility & Contract Tests
//!
//! Checks reference schema fixtures and installed source manifest structure.

use std::fs;
use std::path::Path;

#[test]
fn test_reference_schema_fixtures_are_valid_json() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/schemas");
    let protocol_dir = fixture_dir.join("protocol");
    assert!(
        protocol_dir.exists(),
        "Protocol schema directory must exist at {:?}",
        protocol_dir
    );

    let required_protocol_schemas = [
        "envelope.v1.schema.json",
        "command-envelope.v1.schema.json",
        "event-envelope.v1.schema.json",
        "error.v1.schema.json",
        "provider-request.v1.schema.json",
        "provider-event.v1.schema.json",
        "judgment-request.v1.schema.json",
        "judgment-result.v1.schema.json",
        "capability-request.v1.schema.json",
        "execution-receipt.v1.schema.json",
        "continuation-packet.v1.schema.json",
    ];

    for schema_file in &required_protocol_schemas {
        let path = protocol_dir.join(schema_file);
        assert!(
            path.exists(),
            "Protocol schema file {:?} is missing",
            schema_file
        );
        let content = fs::read_to_string(&path).expect("Must read schema file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Must be valid JSON");
        assert!(parsed.is_object(), "Schema must be a JSON object");
    }

    let packs_dir = fixture_dir.join("packs");
    assert!(
        packs_dir.exists(),
        "Packs schema directory must exist at {:?}",
        packs_dir
    );

    let required_pack_schemas = [
        "pack-manifest.v1.schema.json",
        "workflow.v1.schema.json",
        "verifier-profile.v1.schema.json",
    ];

    for schema_file in &required_pack_schemas {
        let path = packs_dir.join(schema_file);
        assert!(
            path.exists(),
            "Pack schema file {:?} is missing",
            schema_file
        );
        let content = fs::read_to_string(&path).expect("Must read schema file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Must be valid JSON");
        assert!(parsed.is_object(), "Schema must be a JSON object");
    }
}

#[test]
fn test_all_workbench_manifests_exist_and_valid_json() {
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let workbenches = ["assistant", "coding", "research", "quant"];
    for wb in &workbenches {
        let manifest_path = root_dir.join(format!("workbenches/{wb}/manifest/manifest.json"));
        assert!(
            manifest_path.exists(),
            "Workbench manifest must exist at {:?}",
            manifest_path
        );
        let content = fs::read_to_string(&manifest_path).expect("Must read manifest file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Must be valid JSON");
        assert_eq!(
            parsed.get("id").and_then(|v| v.as_str()),
            Some(*wb),
            "Workbench manifest id must match directory name"
        );
    }
}

#[test]
fn test_all_capability_manifests_exist_and_valid_json() {
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let capabilities = [
        ("notebook", "platform/adapters/src/host/notebook", "notebook"),
        ("workspace", "platform/adapters/src/host/workspace", "workspace"),
        ("terminal", "platform/adapters/src/host/terminal", "terminal"),
        ("compute", "platform/adapters/src/connectors/compute", "compute"),
        ("documents", "platform/adapters/src/host/documents", "documents"),
        ("local-inference", "platform/ai/src/local/local_inference", "local_inference"),
        ("mcp", "platform/adapters/src/connectors/mcp", "mcp"),
        ("remote-agents", "platform/adapters/src/connectors/a2a", "a2a"),
    ];
    let exports = format!(
        "{}\n{}",
        fs::read_to_string(root_dir.join("platform/adapters/src/lib.rs")).unwrap(),
        fs::read_to_string(root_dir.join("platform/ai/src/lib.rs")).unwrap(),
    );
    for (cap, path, export_name) in capabilities {
        let module_dir = root_dir.join(path);
        assert!(
            module_dir.join("mod.rs").is_file(),
            "Descriptor needs a real module at {:?}",
            module_dir
        );
        assert!(
            exports.lines().any(|line| line.contains(export_name)),
            "lib.rs must export {:?}",
            export_name
        );
        let manifest_path = module_dir.join("manifest.json");
        assert!(
            manifest_path.exists(),
            "Capability manifest must exist at {:?}",
            manifest_path
        );
        let content = fs::read_to_string(&manifest_path).expect("Must read manifest file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Must be valid JSON");
        assert_eq!(
            parsed.get("id").and_then(|v| v.as_str()),
            Some(cap),
            "Capability descriptor must preserve its stable ID"
        );
    }
}
