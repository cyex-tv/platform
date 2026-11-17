-- Account editors (ADR 0014): an account lets others act on all its sets and channels, each with
-- named permission flags. The owner is never in the table. Additive: per-set editors stay.
CREATE TABLE account_editors (
    account_id  uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    editor_id   uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    permissions text[] NOT NULL CHECK (
        cardinality(permissions) BETWEEN 1 AND 4
        AND permissions <@ ARRAY['emotes', 'sets', 'editors', 'profile']::text[]
    ),
    granted_by  uuid REFERENCES accounts (id) ON DELETE SET NULL,
    granted_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, editor_id),
    CHECK (account_id <> editor_id)
);

-- "Which accounts do I edit" and every authorization check by editor.
CREATE INDEX account_editors_editor ON account_editors (editor_id);
