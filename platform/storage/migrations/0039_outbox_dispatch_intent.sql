-- Migration 0039: Outbox Dispatch Intent & Failure Audit (Protocol Layer S0)
--
-- Adds recipient, payload_json, and error_message columns to outbox_entries
-- so that an outbox record contains everything needed for mediated dispatch
-- and audited delivery.

ALTER TABLE outbox_entries ADD COLUMN recipient TEXT;
ALTER TABLE outbox_entries ADD COLUMN payload_json TEXT;
ALTER TABLE outbox_entries ADD COLUMN error_message TEXT;
