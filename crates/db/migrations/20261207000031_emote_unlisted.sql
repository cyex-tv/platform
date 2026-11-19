-- Emote visibility gets a third state, unlisted (ADR 0020, owner's answer of 2026/11/19).
-- Additive: one column with a default, and a CHECK.
--
-- - public (neither flag): found by search and browse, listed on its creator's page.
-- - unlisted: anyone with its link (its ID) can open it and add it to sets, but search, browse,
--   activity feeds and its creator's page leave it out for everyone but its owner.
-- - private (the existing flag): only its owner can add it to sets; also left out of listings.
--
-- An emote is never both. Existing rows keep what they were.
ALTER TABLE emotes ADD COLUMN unlisted boolean NOT NULL DEFAULT false;
ALTER TABLE emotes ADD CONSTRAINT emotes_one_visibility CHECK (NOT (unlisted AND private));
