-- Default sets (ADR 0017, amended 2026/11/19). Every account has a default channel set named
-- after its display name: new accounts get one at sign-up, and existing live accounts without a
-- channel set of that name get one here. The display name is the one `reads::summaries` shows
-- (Twitch before Kick). IDs are ULIDs (ADR 0002) stamped with the account's creation time, so the
-- default set sorts before the account's other sets. Running it twice adds nothing.
INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity, visibility)
SELECT
    encode(
        overlay(
            uuid_send(gen_random_uuid())
            PLACING substring(int8send(floor(extract(epoch FROM n.created_at) * 1000)::bigint) FROM 3)
            FROM 1 FOR 6
        ),
        'hex'
    )::uuid,
    n.account_id, left(n.display_name, 100), 'channel', 1000, 'listed'
FROM (
    SELECT DISTINCT ON (i.account_id) i.account_id, btrim(i.display_name) AS display_name,
           a.created_at
    FROM identities i
    JOIN accounts a ON a.id = i.account_id AND a.deleted_at IS NULL
    ORDER BY i.account_id, i.platform DESC
) n
WHERE n.display_name <> ''
  AND NOT EXISTS (
      SELECT 1 FROM emote_sets s
      WHERE s.owner_account_id = n.account_id
        AND s.kind = 'channel'
        AND lower(s.name) = lower(left(n.display_name, 100))
  );
