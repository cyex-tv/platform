-- Phase 3: who reviewed an emote, when, and why it was rejected (additive).

ALTER TABLE emotes
    ADD COLUMN reviewed_by   uuid REFERENCES accounts (id) ON DELETE SET NULL,
    ADD COLUMN reviewed_at   timestamptz,
    ADD COLUMN review_reason text CHECK (char_length(review_reason) <= 500);
