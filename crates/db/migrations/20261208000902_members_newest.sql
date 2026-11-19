-- no-transaction
-- Scale (additive): The newest members of a set (profile and set cards' previews): the first few in index order, however large the set is.
-- CONCURRENTLY, so a live table isn't locked for writes while it builds; one statement per file,
-- since that can't run in a transaction.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emote_set_members_set_newest ON emote_set_members (set_id, added_at DESC, emote_id DESC);
