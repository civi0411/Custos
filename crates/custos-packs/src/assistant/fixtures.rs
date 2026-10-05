//! Assistant Pack Task Fixtures (RFC 003 §4)

use custos_domain::task::TaskContract;

/// Fixture: Daily agenda triage and schedule consolidation.
pub fn assistant_agenda_fixture() -> TaskContract {
    TaskContract {
        pack_id: "assistant".into(),
        name: "Daily Agenda & Inbox Triage".into(),
        description: "Parse notifications, extract urgent action items, and draft calendar block"
            .into(),
        required_capabilities: vec!["calendar.read".into(), "inbox.read".into()],
        evidence_requirements: vec![],
    }
}

/// Fixture: Draft response for delegation request.
pub fn assistant_draft_reply_fixture() -> TaskContract {
    TaskContract {
        pack_id: "assistant".into(),
        name: "Compose Safe Delegation Reply".into(),
        description: "Draft message reply with proposed timeline and resource requirements".into(),
        required_capabilities: vec!["inbox.read".into()],
        evidence_requirements: vec![],
    }
}
