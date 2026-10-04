//! Engineering Pack Task Fixtures (RFC 003 §4)

use custos_domain::task::{ContractEvidence, EvidenceKind, TaskContract};

/// Fixture: Multi-crate build and test repair task.
pub fn engineering_repair_fixture() -> TaskContract {
    TaskContract {
        pack_id: "engineering".into(),
        name: "Fix Workspace Compile & Test Errors".into(),
        description: "Multi-crate workspace repair requiring clean build and passing test suite"
            .into(),
        required_capabilities: vec![
            "fs.read".into(),
            "fs.write".into(),
            "exec.process".into(),
        ],
        evidence_requirements: vec![
            ContractEvidence {
                kind: EvidenceKind::Diff,
                required: true,
            },
            ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            },
        ],
    }
}

/// Fixture: AST-safe refactoring across multiple modules.
pub fn engineering_refactor_fixture() -> TaskContract {
    TaskContract {
        pack_id: "engineering".into(),
        name: "AST-Safe Codebase Refactoring".into(),
        description: "Automated refactoring of public interfaces with worktree isolation".into(),
        required_capabilities: vec![
            "fs.read".into(),
            "fs.write".into(),
            "ast.index".into(),
        ],
        evidence_requirements: vec![
            ContractEvidence {
                kind: EvidenceKind::FileAnchor,
                required: true,
            },
            ContractEvidence {
                kind: EvidenceKind::Diff,
                required: true,
            },
        ],
    }
}
