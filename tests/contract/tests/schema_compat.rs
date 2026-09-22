//! Schema Compatibility & Contract Tests
//!
//! Validates that canonical schemas exist and match core domain expectations.

use std::fs;
use std::path::Path;

#[test]
fn test_all_canonical_schemas_exist_and_valid_json() {
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let protocol_dir = root_dir.join("schemas/protocol");
    assert!(
        protocol_dir.exists(),
        "Protocol schema directory must exist at {:?}",
        protocol_dir
    );

    let required_protocol_schemas = [
        "envelope.v1.schema.json",
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

    let packs_dir = root_dir.join("schemas/packs");
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
