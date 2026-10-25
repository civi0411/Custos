//! Command admission and classification helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Read,
    Mutation,
}

/// Classify an API method as Read or Mutation.
pub fn classify_method(method: &str) -> MethodKind {
    let m = method.to_ascii_lowercase();
    if m == "v1.ping"
        || m == "v1.health"
        || m.ends_with(".get")
        || m.ends_with(".list")
        || m.ends_with(".read")
        || m.ends_with(".tree")
        || m.ends_with(".diff")
        || m.ends_with(".status")
        || m.ends_with(".history")
        || m.ends_with(".inspect_dirty")
        || m.ends_with(".detect")
        || m.ends_with(".pick_folder")
        || m.ends_with(".compare")
        || m.ends_with(".pricing")
        || m.ends_with(".probe")
        || m.ends_with(".catalog")
        || m.ends_with(".snapshot")
        || m.ends_with(".screenshot")
    {
        MethodKind::Read
    } else {
        MethodKind::Mutation
    }
}

/// Compute a canonical fingerprint digest for command deduplication.
/// Strips top-level client metadata such as `command_id` so identical payloads hash equally.
pub fn compute_command_fingerprint(params: &serde_json::Value) -> String {
    match params {
        serde_json::Value::Object(map) => {
            let mut filtered = serde_json::Map::new();
            for (k, v) in map {
                if k != "command_id" && k != "request_id" {
                    filtered.insert(k.clone(), v.clone());
                }
            }
            let canonical_json = serde_json::to_string(&filtered).unwrap_or_default();
            custos_domain::digest(canonical_json.as_bytes())
        }
        other => {
            let canonical_json = serde_json::to_string(other).unwrap_or_default();
            custos_domain::digest(canonical_json.as_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_method() {
        assert_eq!(classify_method("v1.ping"), MethodKind::Read);
        assert_eq!(classify_method("v1.health"), MethodKind::Read);
        assert_eq!(classify_method("v1.tasks.list"), MethodKind::Read);
        assert_eq!(classify_method("v1.tasks.get"), MethodKind::Read);
        assert_eq!(classify_method("v1.workspace.files.read"), MethodKind::Read);
        assert_eq!(classify_method("v1.notes.history"), MethodKind::Read);

        assert_eq!(classify_method("v1.tasks.create"), MethodKind::Mutation);
        assert_eq!(classify_method("v1.tasks.cancel"), MethodKind::Mutation);
        assert_eq!(classify_method("v1.terminal.spawn"), MethodKind::Mutation);
        assert_eq!(classify_method("v1.workspace.files.write"), MethodKind::Mutation);
    }

    #[test]
    fn test_compute_command_fingerprint() {
        let p1 = serde_json::json!({
            "command_id": "cmd-1",
            "title": "Build feature",
            "contract": "coding"
        });
        let p2 = serde_json::json!({
            "command_id": "cmd-2",
            "title": "Build feature",
            "contract": "coding"
        });
        let p3 = serde_json::json!({
            "command_id": "cmd-1",
            "title": "Different feature",
            "contract": "coding"
        });

        assert_eq!(
            compute_command_fingerprint(&p1),
            compute_command_fingerprint(&p2)
        );
        assert_ne!(
            compute_command_fingerprint(&p1),
            compute_command_fingerprint(&p3)
        );
    }
}
