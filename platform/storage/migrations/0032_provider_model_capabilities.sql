-- Preserve advertised capabilities without changing model or account identity.
ALTER TABLE provider_model_catalog
    ADD COLUMN capabilities_json TEXT NOT NULL DEFAULT '{}';
