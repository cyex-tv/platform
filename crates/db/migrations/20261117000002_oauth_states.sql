-- Phase 2 PR 2: the short-lived state of a sign-in in progress (ADR 0009).
--
-- `GET /v1/auth/{platform}/login` stores a row and sets a pre-login cookie holding the random
-- `state`; the callback must bring the same `state` back in the query and in that cookie. The row
-- is taken (deleted) on first use. Only the sha256 of `state` is stored; the PKCE verifier stays
-- here and never reaches the browser.

CREATE TABLE oauth_states (
    state_hash      bytea PRIMARY KEY CHECK (length(state_hash) = 32),
    platform        text NOT NULL CHECK (platform IN ('twitch', 'kick')),
    code_verifier   text NOT NULL CHECK (length(code_verifier) BETWEEN 43 AND 128),
    return_to       text NOT NULL,
    -- Set when a signed-in account is linking a second platform.
    link_account_id uuid REFERENCES accounts (id) ON DELETE CASCADE,
    created_at      timestamptz NOT NULL DEFAULT now(),
    expires_at      timestamptz NOT NULL,
    CHECK (expires_at > created_at)
);

CREATE INDEX oauth_states_expires_at ON oauth_states (expires_at);
