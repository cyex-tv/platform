-- Public emote sets: who can find and open a set (additive).
--
-- `listed`: anyone can open it and it shows on the owner's profile. `unlisted`: anyone with its ID
-- can open it, as every set could before this migration. `private`: only its owner, editors and
-- staff, except while a channel shows it (a channel's active set is public in chat anyway).
--
-- Existing rows keep what they could do: nothing becomes private. Channel sets and the global set
-- start listed, the column's default for new channel sets; personal sets start unlisted.
ALTER TABLE emote_sets
    ADD COLUMN visibility text NOT NULL DEFAULT 'listed'
        CHECK (visibility IN ('listed', 'unlisted', 'private'));

UPDATE emote_sets SET visibility = 'unlisted' WHERE kind = 'personal';

-- The personal set is shown in chat to everyone; hiding it would be a promise we can't keep.
ALTER TABLE emote_sets
    ADD CONSTRAINT emote_sets_personal_not_private CHECK (kind <> 'personal' OR visibility <> 'private'),
    ADD CONSTRAINT emote_sets_global_listed CHECK (kind <> 'global' OR visibility = 'listed');

-- `accountSets`: an account's listed sets.
CREATE INDEX emote_sets_owner_listed ON emote_sets (owner_account_id, id) WHERE visibility = 'listed';
