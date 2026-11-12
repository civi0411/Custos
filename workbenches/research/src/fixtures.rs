//! Research Pack Task Fixtures (RFC 003 §4)

use custos_domain::task::{ContractEvidence, EvidenceKind, TaskContract};

/// Fixture: Multi-source literature review and synthesis.
pub fn research_synthesis_fixture() -> TaskContract {
    TaskContract {
        pack_id: "research".into(),
        name: "Multi-Source Literature Synthesis".into(),
        description: "Fan-out extraction across multiple papers and synthesis of key results"
            .into(),
        required_capabilities: vec!["fs.read".into(), "web.search".into()],
        evidence_requirements: vec![ContractEvidence {
            kind: EvidenceKind::FileAnchor,
            required: true,
        }],
    }
}

/// Fixture: Citation graph and prior art survey.
pub fn research_survey_fixture() -> TaskContract {
    TaskContract {
        pack_id: "research".into(),
        name: "Prior Art Citation Survey".into(),
        description: "Deep crawl of academic preprints and citation graph validation".into(),
        required_capabilities: vec!["fs.read".into(), "web.search".into()],
        evidence_requirements: vec![ContractEvidence {
            kind: EvidenceKind::FileAnchor,
            required: true,
        }],
    }
}
