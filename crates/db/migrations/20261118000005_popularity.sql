-- Phase 6: search ranking (ADR 0004). `popularity` is how many channels show the emote in their
-- active set; the worker refreshes it every hour.

ALTER TABLE emotes ADD COLUMN popularity integer NOT NULL DEFAULT 0 CHECK (popularity >= 0);

CREATE INDEX emotes_searchable_popularity ON emotes (popularity DESC, id)
    WHERE state = 'approved' AND deleted_at IS NULL AND NOT private;

-- Tag prefix matches.
CREATE INDEX emote_tags_tag_prefix ON emote_tags (tag text_pattern_ops);
