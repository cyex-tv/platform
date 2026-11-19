-- An account editor's transfer waits for the owner (ADR 0014, owner decision of 2026/11/19).
-- Additive: one new table.
--
-- - An editor's `transferEmote` makes a `pending` request; the emote moves only when the owner
--   accepts. The owner may decline, the editor may cancel, and a request left alone expires after
--   `expires_at` (7 days; marked `expired` the next time anything looks at it).
-- - One open request per emote (the partial unique index).
-- - Requests go with their emote and with either account (CASCADE); the audit log keeps the story.
CREATE TABLE emote_transfer_requests (
    id              uuid PRIMARY KEY,
    emote_id        uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    from_account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    to_account_id   uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    requested_by    uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    state           text NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'accepted', 'declined', 'cancelled', 'expired')),
    created_at      timestamptz NOT NULL DEFAULT now(),
    expires_at      timestamptz NOT NULL,
    resolved_at     timestamptz,
    resolved_by     uuid REFERENCES accounts (id) ON DELETE SET NULL,
    CHECK (from_account_id <> to_account_id),
    CHECK ((state = 'pending') = (resolved_at IS NULL))
);
CREATE UNIQUE INDEX emote_transfer_requests_one_open ON emote_transfer_requests (emote_id) WHERE state = 'pending';
CREATE INDEX emote_transfer_requests_from ON emote_transfer_requests (from_account_id) WHERE state = 'pending';
CREATE INDEX emote_transfer_requests_by ON emote_transfer_requests (requested_by) WHERE state = 'pending';
