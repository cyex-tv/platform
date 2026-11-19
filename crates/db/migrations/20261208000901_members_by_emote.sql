-- no-transaction
-- Scale (additive): Which channels show an emote, newest add first (emoteChannels), and how many (channel counts on every emote list and set read): read from the index alone, in order, instead of every member row.
-- CONCURRENTLY, so a live table isn't locked for writes while it builds; one statement per file,
-- since that can't run in a transaction.
CREATE INDEX CONCURRENTLY IF NOT EXISTS emote_set_members_emote_added ON emote_set_members (emote_id, added_at DESC, set_id);
