-- App sign-in (ADR 0021): a desktop app gets an `app` session through the authorization code flow
-- with PKCE and a loopback redirect. Additive: two columns on `sessions`, two new tables.
--
-- - A session made this way names its client (`app_client_id`). Its access token (the session's
--   `id_hash`) dies at `access_expires_at`; `expires_at` is when the refresh token stops working.
--   A refresh rotates both tokens on the same row, so the session's public `id` stays put and
--   `mySessions` lists one entry per signed-in app.
-- - Refresh tokens hang off the session: signing the session out (any way, account deletion too)
--   removes them by CASCADE. Used ones are kept a while so a replay is noticed (reuse detection).
-- - Authorization codes are single-use and short-lived; a used one remembers the session it gave,
--   so replaying it signs that session out.
ALTER TABLE sessions ADD COLUMN app_client_id text
    CHECK (app_client_id IS NULL OR (kind = 'app' AND char_length(app_client_id) <= 64));
ALTER TABLE sessions ADD COLUMN access_expires_at timestamptz;

CREATE TABLE app_auth_codes (
    code_hash      bytea PRIMARY KEY CHECK (length(code_hash) = 32),
    client_id      text NOT NULL CHECK (char_length(client_id) <= 64),
    account_id     uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    redirect_uri   text NOT NULL CHECK (char_length(redirect_uri) <= 512),
    code_challenge text NOT NULL CHECK (char_length(code_challenge) = 43),
    created_at     timestamptz NOT NULL DEFAULT now(),
    expires_at     timestamptz NOT NULL,
    used_at        timestamptz,
    session_id     uuid REFERENCES sessions (id) ON DELETE SET NULL
);

CREATE INDEX app_auth_codes_expires ON app_auth_codes (expires_at);

CREATE TABLE app_refresh_tokens (
    token_hash bytea PRIMARY KEY CHECK (length(token_hash) = 32),
    session_id uuid NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    used_at    timestamptz
);

CREATE INDEX app_refresh_tokens_session ON app_refresh_tokens (session_id);
