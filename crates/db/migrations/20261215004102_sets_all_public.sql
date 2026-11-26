-- Every set is public (ADR 0017, amendment 2026/11/22): users no longer pick listed, unlisted or
-- private. Every unlisted and private set becomes listed. Only staff with `sets.edit_any` can still
-- take a set off the listings (moderation). Private EMOTES are not touched: they stay usable only
-- by their owner, in their owner's sets. Data only; the column, its default and its checks stay.
-- Running it twice changes nothing.
UPDATE emote_sets SET visibility = 'listed', updated_at = now() WHERE visibility <> 'listed';