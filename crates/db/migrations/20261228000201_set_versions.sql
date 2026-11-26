-- Set versions and a bounded change log for delta sync (ADR 0032, additive).
--
-- `emote_sets.version` goes up by one for every statement that changes what the set shows: a
-- member added, removed, renamed or its zero-width flag changed, the set renamed, or an emote in
-- it changed (state, name, flags, file, owner, deleted) or its file finished or failed.
-- Triggers do it in the same transaction as the write, so no write path can forget it. Each bump
-- writes the members it touched to `emote_set_changes`; `GET /v1/sets/{id}?since=` and
-- `setChanges` read them. The worker keeps 7 days and at most the last 1.000 versions of each
-- set; `change_floor` is the newest version whose rows were pruned, so a `since` below it answers
-- a full resync.
--
-- Lock order: every bump locks the sets it touches by ID, in order, so two statements touching
-- the same sets never wait on each other in a circle.
--
-- Adding columns with a constant default writes no rows (Postgres 11+).
ALTER TABLE emote_sets
    ADD COLUMN version      bigint NOT NULL DEFAULT 0,
    ADD COLUMN change_floor bigint NOT NULL DEFAULT 0;

CREATE TABLE emote_set_changes (
    id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    set_id     uuid NOT NULL REFERENCES emote_sets (id) ON DELETE CASCADE,
    version    bigint NOT NULL,
    -- Null for a change of the set itself (its name).
    emote_id   uuid,
    kind       text NOT NULL CHECK (kind IN ('added', 'removed', 'renamed', 'emote', 'set')),
    changed_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX emote_set_changes_set_version ON emote_set_changes (set_id, version);
CREATE INDEX emote_set_changes_changed_at ON emote_set_changes (changed_at);

-- Members added: one bump per set the statement added to.
CREATE FUNCTION set_versions_members_added() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    WITH locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT set_id FROM added)
        ORDER BY s.id
        FOR UPDATE
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

-- Members removed. A set deleted in the same statement is gone already and is skipped.
CREATE FUNCTION set_versions_members_removed() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    WITH locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT set_id FROM removed)
        ORDER BY s.id
        FOR UPDATE
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

-- Members changed: their name or zero-width flag in the set (or, never done today, moved to
-- another set or emote: logged as removed from the old place and added to the new one).
CREATE FUNCTION set_versions_members_changed() RETURNS trigger
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
        FOR UPDATE
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

CREATE TRIGGER emote_set_members_versions_added
    AFTER INSERT ON emote_set_members
    REFERENCING NEW TABLE AS added
    FOR EACH STATEMENT EXECUTE FUNCTION set_versions_members_added();

CREATE TRIGGER emote_set_members_versions_removed
    AFTER DELETE ON emote_set_members
    REFERENCING OLD TABLE AS removed
    FOR EACH STATEMENT EXECUTE FUNCTION set_versions_members_removed();

CREATE TRIGGER emote_set_members_versions_changed
    AFTER UPDATE ON emote_set_members
    REFERENCING OLD TABLE AS before_rows NEW TABLE AS after_rows
    FOR EACH STATEMENT EXECUTE FUNCTION set_versions_members_changed();

-- The set renamed: bumped on the row itself, logged after.
CREATE FUNCTION set_versions_renamed_bump() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.version := OLD.version + 1;
    RETURN NEW;
END $$;

CREATE FUNCTION set_versions_renamed_log() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO emote_set_changes (set_id, version, emote_id, kind)
    VALUES (NEW.id, NEW.version, NULL, 'set');
    RETURN NULL;
END $$;

CREATE TRIGGER emote_sets_versions_renamed_bump
    BEFORE UPDATE OF name ON emote_sets
    FOR EACH ROW
    WHEN (OLD.name IS DISTINCT FROM NEW.name)
    EXECUTE FUNCTION set_versions_renamed_bump();

CREATE TRIGGER emote_sets_versions_renamed_log
    AFTER UPDATE OF name ON emote_sets
    FOR EACH ROW
    WHEN (OLD.name IS DISTINCT FROM NEW.name)
    EXECUTE FUNCTION set_versions_renamed_log();

-- Every set holding these emotes, bumped once, each emote logged as changed.
CREATE FUNCTION set_versions_bump_emotes(emote_ids uuid[]) RETURNS void
LANGUAGE sql AS $$
    WITH locked AS (
        SELECT s.id FROM emote_sets s
        WHERE s.id IN (SELECT DISTINCT m.set_id FROM emote_set_members m
                       WHERE m.emote_id = ANY(emote_ids))
        ORDER BY s.id
        FOR UPDATE
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

-- An emote changed in a way a set's answer shows. Row-level with a column list, so the nightly
-- popularity and channel-count refreshes, which touch none of these, never fire it.
CREATE FUNCTION set_versions_emote_changed() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM set_versions_bump_emotes(ARRAY[NEW.id]);
    RETURN NULL;
END $$;

CREATE TRIGGER emotes_set_versions
    AFTER UPDATE OF name, zero_width, private, unlisted, nsfw, animated, state, file_id,
                    owner_account_id, deleted_at ON emotes
    FOR EACH ROW
    WHEN (OLD.name IS DISTINCT FROM NEW.name
       OR OLD.zero_width IS DISTINCT FROM NEW.zero_width
       OR OLD.private IS DISTINCT FROM NEW.private
       OR OLD.unlisted IS DISTINCT FROM NEW.unlisted
       OR OLD.nsfw IS DISTINCT FROM NEW.nsfw
       OR OLD.animated IS DISTINCT FROM NEW.animated
       OR OLD.state IS DISTINCT FROM NEW.state
       OR OLD.file_id IS DISTINCT FROM NEW.file_id
       OR OLD.owner_account_id IS DISTINCT FROM NEW.owner_account_id
       OR OLD.deleted_at IS DISTINCT FROM NEW.deleted_at)
    EXECUTE FUNCTION set_versions_emote_changed();

-- An emote's file got its variants (the transcode finished) or failed for good.
CREATE FUNCTION set_versions_file_changed() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM set_versions_bump_emotes(ARRAY(SELECT e.id FROM emotes e WHERE e.file_id = NEW.id));
    RETURN NULL;
END $$;

CREATE TRIGGER files_set_versions
    AFTER UPDATE OF variants, processing_failed_at ON files
    FOR EACH ROW
    WHEN (OLD.variants IS DISTINCT FROM NEW.variants
       OR OLD.processing_failed_at IS DISTINCT FROM NEW.processing_failed_at)
    EXECUTE FUNCTION set_versions_file_changed();
