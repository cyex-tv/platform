-- Direct uploads (ADR 0036): a badge or paint picture sent straight to storage with a presigned
-- PUT, then finalized through the API. Additive.
--
-- One row per presigned upload. The bytes sit at `storage_key` (`uploads/{id}`, never public)
-- until `POST .../finalize` checks them and makes the picture, or the worker deletes them once
-- `expires_at` has passed.
CREATE TABLE direct_uploads (
    id             uuid PRIMARY KEY,
    account_id     uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind           text NOT NULL CHECK (kind IN ('badge', 'paint')),
    storage_key    text NOT NULL,
    content_type   text NOT NULL,
    -- The size the client declared, signed into the URL; finalize checks the object against it.
    declared_bytes bigint NOT NULL CHECK (declared_bytes > 0),
    created_at     timestamptz NOT NULL DEFAULT now(),
    -- Finalize is refused after this, and the worker removes the object and the row.
    expires_at     timestamptz NOT NULL,
    finalized_at   timestamptz,
    image_id       uuid REFERENCES cosmetic_images (id) ON DELETE SET NULL
);

-- Open uploads per account (the cap on unfinalized ones), and the clean-up's scan.
CREATE INDEX direct_uploads_open ON direct_uploads (account_id) WHERE finalized_at IS NULL;
CREATE INDEX direct_uploads_expires ON direct_uploads (expires_at);
