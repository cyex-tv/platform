-- no-transaction
-- Stored channel counts, the backfill: 2000 emotes at a time in ID order, each batch its own
-- transaction (COMMIT inside DO works because this file runs outside a transaction and is one
-- statement), so no long transaction holds row locks on emotes. Counts changed meanwhile are
-- queued by the triggers of the previous migration and recounted by the worker.
DO $$
DECLARE
    last uuid := '00000000-0000-0000-0000-000000000000';
    upto uuid;
BEGIN
    LOOP
        -- The batch's last ID (uuid has no max()); NULL when no emotes are left.
        upto := NULL;
        SELECT b.id INTO upto FROM (
            SELECT id FROM emotes WHERE id > last ORDER BY id LIMIT 2000) b
        ORDER BY b.id DESC LIMIT 1;
        EXIT WHEN upto IS NULL;
        UPDATE emotes e SET channel_count = n.c
        FROM (
            SELECT m.emote_id, count(DISTINCT ch.id)::integer AS c
            FROM emote_set_members m
            JOIN channels ch ON ch.active_set_id = m.set_id
            JOIN accounts a ON a.id = ch.account_id AND a.deleted_at IS NULL
                AND (a.blocked_at IS NULL OR a.blocked_until <= now())
            WHERE m.emote_id > last AND m.emote_id <= upto
            GROUP BY m.emote_id
        ) n
        WHERE e.id = n.emote_id AND e.channel_count <> n.c;
        last := upto;
        COMMIT;
    END LOOP;
END $$;
