-- no-transaction
-- Scale (additive): Browse sorted TOP (searchEmotes with no text): the page in index order instead of sorting every searchable emote.
-- CONCURRENTLY, so a live table isn't locked for writes while it builds; one statement per file,
-- since that can't run in a transaction.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emotes_browse_top ON emotes (popularity DESC, lower(name), id) WHERE state = 'approved' AND deleted_at IS NULL AND NOT private AND NOT unlisted;
