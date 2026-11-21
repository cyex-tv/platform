-- Snowflake public IDs for accounts, emotes and emote sets (ADR 0027). The uuid primary keys and
-- every join stay as they are; `public_id` is what the API shows and takes.
--
-- An ID is 64 bits, positive: 41 bits of milliseconds since the cyex epoch 2026-01-01T00:00:00Z,
-- 10 bits of instance, 12 bits of sequence. They are made here, in Postgres, so any number of
-- api instances share one generator and can't collide; `cyex_tv.snowflake_instance` (0 to 1023,
-- default 0) keeps two databases apart should there ever be two.

-- The last ID handed out. A sequence because setval is not transactional: a rolled-back insert
-- never hands its ID out again, and no row lock is held until the caller's commit.
CREATE SEQUENCE snowflake_last AS bigint MINVALUE 0 START 0;

-- The next ID: strictly greater than every earlier one, even within one millisecond (4096 a
-- millisecond, then it borrows the next) and when the clock steps back. A short session-level
-- advisory lock orders concurrent callers; it is released before returning, also on errors.
CREATE FUNCTION snowflake_id() RETURNS bigint
LANGUAGE plpgsql VOLATILE AS $$
DECLARE
    epoch_ms constant bigint := 1767225600000;
    lock_key constant bigint := 7302026010100001;
    instance bigint := coalesce(nullif(current_setting('cyex_tv.snowflake_instance', true), '')::bigint, 0);
    now_ms bigint;
    last bigint;
    ms bigint;
    seq bigint;
    next_id bigint;
BEGIN
    IF instance < 0 OR instance > 1023 THEN
        RAISE EXCEPTION 'cyex_tv.snowflake_instance must be 0 to 1023, not %', instance;
    END IF;
    PERFORM pg_advisory_lock(lock_key);
    BEGIN
        SELECT last_value INTO last FROM snowflake_last;
        now_ms := greatest(floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint - epoch_ms, 1);
        IF now_ms > (last >> 22) THEN
            ms := now_ms;
            seq := 0;
        ELSIF (last & 4095) < 4095 THEN
            ms := last >> 22;
            seq := (last & 4095) + 1;
        ELSE
            ms := (last >> 22) + 1;
            seq := 0;
        END IF;
        next_id := (ms << 22) | (instance << 12) | seq;
        PERFORM setval('snowflake_last', next_id);
    EXCEPTION WHEN query_canceled OR OTHERS THEN
        PERFORM pg_advisory_unlock(lock_key);
        RAISE;
    END;
    PERFORM pg_advisory_unlock(lock_key);
    RETURN next_id;
END
$$;

-- The time an ID was made, for reading IDs back.
CREATE FUNCTION snowflake_time(id bigint) RETURNS timestamptz
LANGUAGE sql IMMUTABLE AS $$
    SELECT to_timestamp(((id >> 22) + 1767225600000) / 1000.0)
$$;

ALTER TABLE accounts ADD COLUMN public_id bigint;
ALTER TABLE emotes ADD COLUMN public_id bigint;
ALTER TABLE emote_sets ADD COLUMN public_id bigint;

-- Existing rows, oldest first across all three tables: the time part from created_at (rows from
-- before the epoch get its first millisecond), the sequence part unique. slot = ms * 4096 + seq;
-- each slot is at least its row's own and one above the one before (a running max), so IDs are
-- unique across the tables and follow created_at.
CREATE TEMPORARY TABLE snowflake_backfill ON COMMIT DROP AS
WITH all_rows AS (
    SELECT 'a' AS kind, id, created_at FROM accounts
    UNION ALL SELECT 'e', id, created_at FROM emotes
    UNION ALL SELECT 's', id, created_at FROM emote_sets
), ranked AS (
    SELECT kind, id,
           greatest(floor(extract(epoch FROM created_at) * 1000)::bigint - 1767225600000, 1) * 4096 AS base,
           row_number() OVER (ORDER BY created_at, kind, id) - 1 AS i
    FROM all_rows
), slotted AS (
    SELECT kind, id, i + max(base - i) OVER (ORDER BY i ROWS UNBOUNDED PRECEDING) AS slot
    FROM ranked
)
SELECT kind, id, ((slot / 4096) << 22) | (slot % 4096) AS public_id FROM slotted;

UPDATE accounts t SET public_id = b.public_id FROM snowflake_backfill b WHERE b.kind = 'a' AND b.id = t.id;
UPDATE emotes t SET public_id = b.public_id FROM snowflake_backfill b WHERE b.kind = 'e' AND b.id = t.id;
UPDATE emote_sets t SET public_id = b.public_id FROM snowflake_backfill b WHERE b.kind = 's' AND b.id = t.id;

-- New IDs start after the backfilled ones.
SELECT setval('snowflake_last', coalesce((SELECT max(public_id) FROM snowflake_backfill), 0));

ALTER TABLE accounts
    ALTER COLUMN public_id SET DEFAULT snowflake_id(),
    ALTER COLUMN public_id SET NOT NULL,
    ADD CONSTRAINT accounts_public_id_key UNIQUE (public_id);
ALTER TABLE emotes
    ALTER COLUMN public_id SET DEFAULT snowflake_id(),
    ALTER COLUMN public_id SET NOT NULL,
    ADD CONSTRAINT emotes_public_id_key UNIQUE (public_id);
ALTER TABLE emote_sets
    ALTER COLUMN public_id SET DEFAULT snowflake_id(),
    ALTER COLUMN public_id SET NOT NULL,
    ADD CONSTRAINT emote_sets_public_id_key UNIQUE (public_id);
