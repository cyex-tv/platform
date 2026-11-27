-- Set versions with lighter locks (ADR 0035, additive; 20261228000201 stays as it ran).
--
-- 1. Every version bump locks its sets `FOR NO KEY UPDATE` instead of `FOR UPDATE`. Only
--    `version` changes, never a key, so the bump needn't block the `KEY SHARE` lock that every
--    insert into `emote_set_members` (its foreign key) takes on the set: a member added while a
--    bump of the same set runs no longer waits on it, nor it on the insert.
-- 2. An emote change bumps its sets once per statement, not once per emote row: the trigger is
--    statement-level and reads the changed rows from the transition tables, so all the sets a
--    statement touches are locked in one pass, ordered by ID. Row by row, an `UPDATE` of several
--    emotes locked their sets in the order of the rows, and two such statements could each hold
--    a set the other was waiting for.
--
-- A trigger with transition tables can't list columns (`UPDATE OF ...`), so this one fires on
-- every `UPDATE` of `emotes`; it compares the old and new rows and does nothing when none of the
-- columns a set's answer shows changed. The nightly popularity and channel-count refreshes only
-- update the rows whose numbers changed, so what it reads stays small.

CREATE OR REPLACE FUNCTION set_versions_members_added() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    WITH locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT set_id FROM added)
        ORDER BY s.id
        FOR NO KEY UPDATE
    ), bumped AS (
        UPDATE emote_sets s SET version = s.version + 1
        FROM locked l WHERE s.id = l.id
        RETURNING s.id, s.version
    )
    INSERT INTO emote_set_changes (set_id, version, emote_id, kind)
    SELECT b.id, b.version, a.emote_id, 'added'
    FROM added a JOIN bumped b ON b.id = a.set_id;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION set_versions_members_removed() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    WITH locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT set_id FROM removed)
        ORDER BY s.id
        FOR NO KEY UPDATE
    ), bumped AS (
        UPDATE emote_sets s SET version = s.version + 1
        FROM locked l WHERE s.id = l.id
        RETURNING s.id, s.version
    )
    INSERT INTO emote_set_changes (set_id, version, emote_id, kind)
    SELECT b.id, b.version, r.emote_id, 'removed'
    FROM removed r JOIN bumped b ON b.id = r.set_id;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION set_versions_members_changed() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    WITH touched AS (
        SELECT DISTINCT n.set_id, n.emote_id, 'renamed' AS kind
        FROM after_rows n
        JOIN before_rows o ON o.set_id = n.set_id AND o.emote_id = n.emote_id
        WHERE o.name IS DISTINCT FROM n.name OR o.zero_width IS DISTINCT FROM n.zero_width
        UNION
        SELECT o.set_id, o.emote_id, 'removed' FROM before_rows o
        WHERE NOT EXISTS (SELECT 1 FROM after_rows n
                          WHERE n.set_id = o.set_id AND n.emote_id = o.emote_id)
        UNION
        SELECT n.set_id, n.emote_id, 'added' FROM after_rows n
        WHERE NOT EXISTS (SELECT 1 FROM before_rows o
                          WHERE o.set_id = n.set_id AND o.emote_id = n.emote_id)
    ), locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT set_id FROM touched)
        ORDER BY s.id
        FOR NO KEY UPDATE
    ), bumped AS (
        UPDATE emote_sets s SET version = s.version + 1
        FROM locked l WHERE s.id = l.id
        RETURNING s.id, s.version
    )
    INSERT INTO emote_set_changes (set_id, version, emote_id, kind)
    SELECT b.id, b.version, r.emote_id, r.kind
    FROM touched r JOIN bumped b ON b.id = r.set_id;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION set_versions_bump_emotes(emote_ids uuid[]) RETURNS void
LANGUAGE sql AS $$
    WITH locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT m.set_id FROM emote_set_members m
                       WHERE m.emote_id = ANY(emote_ids))
        ORDER BY s.id
        FOR NO KEY UPDATE
    ), bumped AS (
        UPDATE emote_sets s SET version = s.version + 1
        FROM locked l WHERE s.id = l.id
        RETURNING s.id, s.version
    )
    INSERT INTO emote_set_changes (set_id, version, emote_id, kind)
    SELECT b.id, b.version, m.emote_id, 'emote'
    FROM emote_set_members m JOIN bumped b ON b.id = m.set_id
    WHERE m.emote_id = ANY(emote_ids)
$$;

DROP TRIGGER emotes_set_versions ON emotes;
DROP FUNCTION set_versions_emote_changed();

-- The emotes a statement changed in a way a set's answer shows, all bumped together.
CREATE FUNCTION set_versions_emotes_changed() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    changed uuid[];
BEGIN
    SELECT array_agg(n.id ORDER BY n.id) INTO changed
    FROM new_emotes n JOIN old_emotes o ON o.id = n.id
    WHERE (o.name, o.zero_width, o.private, o.unlisted, o.nsfw, o.animated, o.state, o.file_id,
           o.owner_account_id, o.deleted_at)
          IS DISTINCT FROM
          (n.name, n.zero_width, n.private, n.unlisted, n.nsfw, n.animated, n.state, n.file_id,
           n.owner_account_id, n.deleted_at);
    IF changed IS NOT NULL THEN
        PERFORM set_versions_bump_emotes(changed);
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER emotes_set_versions
    AFTER UPDATE ON emotes
    REFERENCING OLD TABLE AS old_emotes NEW TABLE AS new_emotes
    FOR EACH STATEMENT EXECUTE FUNCTION set_versions_emotes_changed();
