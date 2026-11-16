-- Phase 3: cyex.tv's own image blocklist (PLAN.md Phase 3, step 3). An upload is refused when its
-- sha256 matches exactly, or its dHash is within a few bits of a blocked one. Entries are added
-- by moderators (Phase 7); every addition is audited there.

CREATE TABLE blocked_images (
    id         uuid PRIMARY KEY,
    sha256     bytea UNIQUE CHECK (length(sha256) = 32),
    dhash      bigint,
    reason     text NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    created_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (sha256 IS NOT NULL OR dhash IS NOT NULL)
);

CREATE INDEX blocked_images_dhash ON blocked_images (dhash) WHERE dhash IS NOT NULL;

-- The per-account upload rate limit counts recent emotes.
CREATE INDEX emotes_owner_created ON emotes (owner_account_id, created_at);
