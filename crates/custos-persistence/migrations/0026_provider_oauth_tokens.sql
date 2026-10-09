-- Migration 0026: Provider OAuth Tokens Store
--
-- Persists OAuth 2.0 access and refresh tokens, scopes, and expiration timestamps
-- for AI model providers (OpenAI PKCE, Copilot, Google OAuth).

CREATE TABLE IF NOT EXISTS provider_oauth_tokens (
    provider_id TEXT PRIMARY KEY,
    service_type TEXT NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at INTEGER NOT NULL,
    token_type TEXT NOT NULL DEFAULT 'Bearer',
    scope TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_oauth_tokens_expiry ON provider_oauth_tokens(expires_at);
