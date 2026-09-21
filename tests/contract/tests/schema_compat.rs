//! Schema Compatibility & Contract Tests
//!
//! Validates that canonical schemas exist and match core domain expectations.

use std::fs;
use std::path::Path;

#[test]
fn test_all_canonical_schemas_exist_and_valid_json() {
    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().join("schemas/custos/v1");
    assert!(schema_dir.exists(), "Schema directory must exist at {:?}", schema_dir);

    let required_schemas = [
        "task.v1.schema.json",
        "continuation.v1.schema.json",
        "action.v1.schema.json",
        "capability.v1.schema.json",
        "claim.v1.schema.json",
        "context_pack.v1.schema.json",
        "evidence.v1.schema.json",
        "provider_capability.v1.schema.json",
        "workflow.v1.schema.json",
    ];

    for schema_file in &required_schemas {
        let path = schema_dir.join(schema_file);
        assert!(path.exists(), "Schema file {:?} is missing", schema_file);
        let content = fs::read_to_string(&path).expect("Must read schema file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Must be valid JSON");
        assert!(parsed.is_object(), "Schema must be a JSON object");
    }
}
