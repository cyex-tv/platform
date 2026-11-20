-- Retire unscoped app sessions (ADR 0021, amendment of 2026/11/20). Before app sign-in, an `app`
-- session had no client and no scopes: a Bearer token with the account's full rights. Nothing
-- issued them in production, but any left are signed out here, and the CHECK keeps new ones from
-- being made: every `app` session now names the client that signed in through `/v1/auth/app`.
-- Site (cookie) sessions are untouched. Their refresh tokens go with them (ON DELETE CASCADE).
DELETE FROM sessions WHERE kind = 'app' AND app_client_id IS NULL;

ALTER TABLE sessions ADD CONSTRAINT sessions_app_has_client
    CHECK (kind <> 'app' OR app_client_id IS NOT NULL);
