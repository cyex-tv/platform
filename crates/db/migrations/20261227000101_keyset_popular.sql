-- no-transaction
-- Cursor paging (additive, ADR 0030): Browse by most used (searchEmotes RELEVANCE with no text) read after a position as one range of this index, ties by ID.
-- CONCURRENTLY, so a live table isn't locked for writes while it builds; one statement per file,
-- since that can't run in a transaction.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emotes_keyset_popular ON emotes ((-popularity), id) WHERE state = 'approved' AND deleted_at IS NULL AND NOT private AND NOT unlisted;
