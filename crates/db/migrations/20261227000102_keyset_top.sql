-- no-transaction
-- Cursor paging (additive, ADR 0030): Browse sorted TOP read after a position as one range of this index.
-- CONCURRENTLY, so a live table isn't locked for writes while it builds; one statement per file.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emotes_keyset_top ON emotes ((-popularity), lower(name), id) WHERE state = 'approved' AND deleted_at IS NULL AND NOT private AND NOT unlisted;
