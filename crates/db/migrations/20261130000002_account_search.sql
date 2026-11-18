-- User search (`searchAccounts`, additive): substring and similarity matches on linked
-- Twitch and Kick logins and display names. pg_trgm comes from the first migration.
CREATE INDEX identities_login_trgm ON identities USING gin (lower(login) gin_trgm_ops);
CREATE INDEX identities_display_name_trgm ON identities USING gin (lower(display_name) gin_trgm_ops);
