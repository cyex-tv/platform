-- Review notes on emotes: what staff tell each other about one upload in the review queue.
-- Additive: one new table and one index for the queue's resolved part.
--
-- Only staff with emotes.review read or write them; the uploader never sees them. A deleted note
-- keeps its row (deleted_at, deleted_by), so the audit trail and the note agree.
CREATE TABLE emote_notes (
    id         uuid PRIMARY KEY,
    emote_id   uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    author_id  uuid REFERENCES accounts (id) ON DELETE SET NULL,
    body       text NOT NULL CHECK (char_length(body) BETWEEN 1 AND 2000),
    created_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz,
    deleted_by uuid REFERENCES accounts (id) ON DELETE SET NULL
);

CREATE INDEX emote_notes_emote ON emote_notes (emote_id, created_at DESC) WHERE deleted_at IS NULL;

-- The queue's resolved part lists reviewed emotes, the latest review first.
CREATE INDEX emotes_reviewed ON emotes (reviewed_at DESC, id) WHERE reviewed_at IS NOT NULL;
