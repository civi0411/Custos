-- 0005_evidence.sql
-- Custos Evidence Bundles and Cryptographic Verification Receipts
-- Conforms to Super Plan Section 1.4, 2.8, and PR-02 requirements

CREATE TABLE IF NOT EXISTS evidence_bundles (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    source_version TEXT,
    status TEXT NOT NULL CHECK(status IN ('pass', 'fail', 'unknown', 'stale')),
    manifest_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_evidence_bundles_task ON evidence_bundles(task_id);
CREATE INDEX IF NOT EXISTS idx_evidence_bundles_status ON evidence_bundles(status);

CREATE TABLE IF NOT EXISTS verification_receipts (
    id TEXT PRIMARY KEY,
    bundle_id TEXT NOT NULL REFERENCES evidence_bundles(id) ON DELETE CASCADE,
    criterion_id TEXT NOT NULL,
    verifier_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pass', 'fail', 'unknown', 'stale')),
    evidence_digest TEXT NOT NULL,
    details_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_verification_receipts_bundle ON verification_receipts(bundle_id);
CREATE INDEX IF NOT EXISTS idx_verification_receipts_criterion ON verification_receipts(criterion_id);
