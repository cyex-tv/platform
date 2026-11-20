-- Stored channel counts (scale audit P1, additive). `emotes.channel_count` is how many channels
-- show the emote (their active set holds it; channels of deleted or blocked accounts don't count).
-- Reads take it instead of counting set members on every list and set read. The worker keeps it
-- at most about a minute old:
--
-- - These triggers queue every emote whose count may have changed (`emote_channel_count_queue`):
--   a member added to a set some channel shows, any member removed (a set deleted with its
--   channel's link cleared in the same statement is caught this way), a channel switching its active set
--   or going away, an account deleted, blocked or unblocked. The queue is append-only, so a busy
--   set edit never waits on the worker or on another edit.
-- - Every minute the worker recounts the queued emotes and removes the rows it read.
-- - Every hour it recounts all emotes in batches (a timed block running out changes counts
--   without any write, and it repairs anything missed).
--
-- Adding a column with a constant default writes no rows (Postgres 11+). The backfill is the next
-- migration, in batches.
ALTER TABLE emotes ADD COLUMN channel_count integer NOT NULL DEFAULT 0;

CREATE TABLE emote_channel_count_queue (
    id        bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    emote_id  uuid NOT NULL,
    queued_at timestamptz NOT NULL DEFAULT now()
);

-- The emotes of these sets, when a channel shows the set.
CREATE FUNCTION queue_channel_counts_of_sets(set_ids uuid[]) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO emote_channel_count_queue (emote_id)
    SELECT DISTINCT m.emote_id
    FROM emote_set_members m
    WHERE m.set_id = ANY(set_ids)
$$;

CREATE FUNCTION queue_channel_counts_members_added() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO emote_channel_count_queue (emote_id)
    SELECT DISTINCT n.emote_id FROM added n
    WHERE EXISTS (SELECT 1 FROM channels c WHERE c.active_set_id = n.set_id);
    RETURN NULL;
END $$;

CREATE FUNCTION queue_channel_counts_members_removed() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO emote_channel_count_queue (emote_id)
    SELECT DISTINCT o.emote_id FROM removed o;
    RETURN NULL;
END $$;

CREATE FUNCTION queue_channel_counts_member_moved() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO emote_channel_count_queue (emote_id) VALUES (OLD.emote_id), (NEW.emote_id);
    RETURN NULL;
END $$;

CREATE TRIGGER emote_set_members_counts_added
    AFTER INSERT ON emote_set_members
    REFERENCING NEW TABLE AS added
    FOR EACH STATEMENT EXECUTE FUNCTION queue_channel_counts_members_added();

CREATE TRIGGER emote_set_members_counts_removed
    AFTER DELETE ON emote_set_members
    REFERENCING OLD TABLE AS removed
    FOR EACH STATEMENT EXECUTE FUNCTION queue_channel_counts_members_removed();

CREATE TRIGGER emote_set_members_counts_moved
    AFTER UPDATE OF set_id, emote_id ON emote_set_members
    FOR EACH ROW
    WHEN (OLD.set_id IS DISTINCT FROM NEW.set_id OR OLD.emote_id IS DISTINCT FROM NEW.emote_id)
    EXECUTE FUNCTION queue_channel_counts_member_moved();

CREATE FUNCTION queue_channel_counts_channel() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM queue_channel_counts_of_sets(ARRAY[NEW.active_set_id]);
    ELSIF TG_OP = 'DELETE' THEN
        PERFORM queue_channel_counts_of_sets(ARRAY[OLD.active_set_id]);
    ELSIF OLD.active_set_id IS DISTINCT FROM NEW.active_set_id
       OR OLD.account_id IS DISTINCT FROM NEW.account_id THEN
        PERFORM queue_channel_counts_of_sets(ARRAY[OLD.active_set_id, NEW.active_set_id]);
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER channels_counts
    AFTER INSERT OR DELETE OR UPDATE OF active_set_id, account_id ON channels
    FOR EACH ROW EXECUTE FUNCTION queue_channel_counts_channel();

CREATE FUNCTION queue_channel_counts_account() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM queue_channel_counts_of_sets(
        ARRAY(SELECT c.active_set_id FROM channels c
              WHERE c.account_id = NEW.id AND c.active_set_id IS NOT NULL));
    RETURN NULL;
END $$;

CREATE TRIGGER accounts_counts
    AFTER UPDATE OF deleted_at, blocked_at, blocked_until ON accounts
    FOR EACH ROW
    WHEN (OLD.deleted_at IS DISTINCT FROM NEW.deleted_at
       OR OLD.blocked_at IS DISTINCT FROM NEW.blocked_at
       OR OLD.blocked_until IS DISTINCT FROM NEW.blocked_until)
    EXECUTE FUNCTION queue_channel_counts_account();
