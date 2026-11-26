-- no-transaction
-- Cursor paging (additive, ADR 0030): an account's own emotes (myEmotes, accountEmotes), newest first, read after a position as one range.
-- CONCURRENTLY, so a live table isn't locked for writes while it builds; one statement per file.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emotes_owner_newest ON emotes (owner_account_id, created_at DESC, id DESC) WHERE deleted_at IS NULL;
