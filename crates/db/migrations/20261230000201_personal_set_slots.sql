-- Personal sets: slots by role limit, a staff-only override, and a personal set for every account
-- (ADR 0038).
--
-- A personal set holds 3 emotes, or the most a role the owner holds allows through the role limit
-- `personal_set_slots` (a subscriber role sets 10), or the staff override `capacity_override` when
-- there is one. `capacity` is not used for personal sets any more: queries read the effective
-- number through `personal_set_capacity`. Members beyond a lowered capacity stay; only new adds
-- stop until the set is under the limit.
ALTER TABLE emote_sets
    ADD COLUMN capacity_override integer
        CHECK (capacity_override IS NULL OR capacity_override BETWEEN 1 AND 1000000),
    ADD CONSTRAINT emote_sets_override_personal_only
        CHECK (capacity_override IS NULL OR kind = 'personal');

-- Staff had raised some personal sets above the old default of 5: those keep their number as an
-- override. Sets an owner lowered below 5 go to the default.
UPDATE emote_sets SET capacity_override = capacity WHERE kind = 'personal' AND capacity > 5;

-- The effective capacity of a personal set: the override, else the most generous
-- `personal_set_slots` among the owner's roles (the same rule as every role limit), else 3.
CREATE FUNCTION personal_set_capacity(owner_id uuid, override_slots integer) RETURNS integer
    LANGUAGE sql STABLE AS $$
    SELECT COALESCE(override_slots, (
        SELECT max(v.n)
        FROM (
            SELECT CASE WHEN (r.limits ->> 'personal_set_slots') ~ '^[0-9]{1,7}$'
                        THEN (r.limits ->> 'personal_set_slots')::integer END AS n
            FROM account_roles ar
            JOIN roles r ON r.id = ar.role_id
            WHERE ar.account_id = owner_id
        ) v
        WHERE v.n BETWEEN 1 AND 1000000
    ), 3)
$$;

-- Every live account gets a personal set (new accounts get one at sign-up). IDs are ULIDs stamped
-- with the account's creation time, like the default sets. Running it twice adds nothing.
INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity, visibility)
SELECT
    encode(
        overlay(
            uuid_send(gen_random_uuid())
            PLACING substring(int8send(floor(extract(epoch FROM a.created_at) * 1000)::bigint) FROM 3)
            FROM 1 FOR 6
        ),
        'hex'
    )::uuid,
    a.id, 'Personal', 'personal', 3, 'listed'
FROM accounts a
WHERE a.deleted_at IS NULL
  AND NOT EXISTS (
      SELECT 1 FROM emote_sets s WHERE s.owner_account_id = a.id AND s.kind = 'personal'
  );
