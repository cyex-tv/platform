-- Staff notes on accounts (ADR 0010 amendment 2026/11/19). Additive: one new table.
--
-- Only staff with users.notes read or write them; the account never sees them. A deleted note
-- keeps its row (deleted_at, deleted_by), so the audit trail and the note agree.
CREATE TABLE staff_notes (
    id         uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    author_id  uuid REFERENCES accounts (id) ON DELETE SET NULL,
    body       text NOT NULL CHECK (char_length(body) BETWEEN 1 AND 2000),
    created_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz,
    deleted_by uuid REFERENCES accounts (id) ON DELETE SET NULL
);

CREATE INDEX staff_notes_account ON staff_notes (account_id, created_at DESC) WHERE deleted_at IS NULL;

-- The entitlement graph reads the audit log by item: who was given or lost one badge, paint or
-- role. These expression indexes keep that read from scanning the log.
CREATE INDEX audit_log_grant_item ON audit_log (
    (coalesce(after_json->>'cosmetic_id', before_json->>'cosmetic_id', after_json->>'role_id', before_json->>'role_id'))
) WHERE action IN ('cosmetic.granted', 'cosmetic.revoked', 'role.assigned', 'role.unassigned');
