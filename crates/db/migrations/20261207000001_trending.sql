-- Trending emotes (ADR 0004): an hourly rollup of how many distinct channels added or removed an
-- emote, from the `set.emote_added` and `set.emote_removed` events of sets that are a channel's
-- active set (their `channel:*` topics). The worker fills it from the events outbox before the
-- outbox is pruned (48 hours).
CREATE TABLE emote_channel_adds (
    emote_id uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    hour     timestamptz NOT NULL,
    adds     integer NOT NULL DEFAULT 0 CHECK (adds >= 0),
    removes  integer NOT NULL DEFAULT 0 CHECK (removes >= 0),
    PRIMARY KEY (emote_id, hour)
);

CREATE INDEX emote_channel_adds_hour ON emote_channel_adds (hour, emote_id);
