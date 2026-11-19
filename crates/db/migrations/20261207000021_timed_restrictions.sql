-- Timed restrictions, a ban from filing reports, and their history (ADR 0010 amendment
-- 2026/11/19). Additive: new columns, a new table, a replaced constraint and trigger function.
--
-- An account has three holds: uploads (restricted_at, as before), tickets (new: no reports or
-- tickets) and the whole account (blocked_at, as before). Each may now end: *_until is when, and
-- NULL means for good. A hold whose *_until has passed is over; every read treats it as absent,
-- so nothing has to sweep it away.
ALTER TABLE accounts
    ADD COLUMN restricted_until      timestamptz,
    ADD COLUMN blocked_until         timestamptz,
    ADD COLUMN tickets_blocked_at    timestamptz,
    ADD COLUMN tickets_blocked_until timestamptz;

-- Holds follow the person (migration identity_holds), with their end time and the new kind.
ALTER TABLE identity_holds
    ADD COLUMN restricted_until      timestamptz,
    ADD COLUMN blocked_until         timestamptz,
    ADD COLUMN tickets_blocked_at    timestamptz,
    ADD COLUMN tickets_blocked_until timestamptz;
ALTER TABLE identity_holds DROP CONSTRAINT identity_holds_check;
ALTER TABLE identity_holds ADD CONSTRAINT identity_holds_some
    CHECK (restricted_at IS NOT NULL OR blocked_at IS NOT NULL OR tickets_blocked_at IS NOT NULL);

-- Only holds still running are kept; a second hold on the same identity keeps the longer one.
CREATE OR REPLACE FUNCTION identities_hold_restricted() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO identity_holds (platform, platform_user_id, account_id,
                                restricted_at, restricted_until, blocked_at, blocked_until,
                                tickets_blocked_at, tickets_blocked_until)
    SELECT OLD.platform, OLD.platform_user_id, a.id,
           r.at, r.until, b.at, b.until, t.at, t.until
      FROM accounts a,
           LATERAL (SELECT CASE WHEN a.restricted_until IS NULL OR a.restricted_until > now() THEN a.restricted_at END AS at,
                           CASE WHEN a.restricted_until > now() THEN a.restricted_until END AS until) r,
           LATERAL (SELECT CASE WHEN a.blocked_until IS NULL OR a.blocked_until > now() THEN a.blocked_at END AS at,
                           CASE WHEN a.blocked_until > now() THEN a.blocked_until END AS until) b,
           LATERAL (SELECT CASE WHEN a.tickets_blocked_until IS NULL OR a.tickets_blocked_until > now() THEN a.tickets_blocked_at END AS at,
                           CASE WHEN a.tickets_blocked_until > now() THEN a.tickets_blocked_until END AS until) t
     WHERE a.id = OLD.account_id
       AND (r.at IS NOT NULL OR b.at IS NOT NULL OR t.at IS NOT NULL)
    ON CONFLICT (platform, platform_user_id) DO UPDATE
       SET account_id = EXCLUDED.account_id,
           restricted_until = CASE
               WHEN EXCLUDED.restricted_at IS NULL THEN identity_holds.restricted_until
               WHEN identity_holds.restricted_at IS NULL THEN EXCLUDED.restricted_until
               WHEN identity_holds.restricted_until IS NULL OR EXCLUDED.restricted_until IS NULL THEN NULL
               ELSE greatest(identity_holds.restricted_until, EXCLUDED.restricted_until) END,
           restricted_at = coalesce(identity_holds.restricted_at, EXCLUDED.restricted_at),
           blocked_until = CASE
               WHEN EXCLUDED.blocked_at IS NULL THEN identity_holds.blocked_until
               WHEN identity_holds.blocked_at IS NULL THEN EXCLUDED.blocked_until
               WHEN identity_holds.blocked_until IS NULL OR EXCLUDED.blocked_until IS NULL THEN NULL
               ELSE greatest(identity_holds.blocked_until, EXCLUDED.blocked_until) END,
           blocked_at = coalesce(identity_holds.blocked_at, EXCLUDED.blocked_at),
           tickets_blocked_until = CASE
               WHEN EXCLUDED.tickets_blocked_at IS NULL THEN identity_holds.tickets_blocked_until
               WHEN identity_holds.tickets_blocked_at IS NULL THEN EXCLUDED.tickets_blocked_until
               WHEN identity_holds.tickets_blocked_until IS NULL OR EXCLUDED.tickets_blocked_until IS NULL THEN NULL
               ELSE greatest(identity_holds.tickets_blocked_until, EXCLUDED.tickets_blocked_until) END,
           tickets_blocked_at = coalesce(identity_holds.tickets_blocked_at, EXCLUDED.tickets_blocked_at),
           held_at = now();
    RETURN OLD;
END
$$;

-- Every hold placed or lifted, with who, why and until when. The accounts columns say what holds
-- now; this says how it got there. Rows are only ever added to (lifted_* set once).
CREATE TABLE account_restrictions (
    id         uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind       text NOT NULL CHECK (kind IN ('uploads', 'tickets', 'account')),
    reason     text CHECK (reason IS NULL OR char_length(reason) BETWEEN 1 AND 500),
    -- NULL: for good.
    ends_at    timestamptz,
    created_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    -- Lifted early by staff (or replaced by a newer hold of the same kind).
    lifted_at  timestamptz,
    lifted_by  uuid REFERENCES accounts (id) ON DELETE SET NULL,
    CHECK (ends_at IS NULL OR ends_at > created_at)
);

CREATE INDEX account_restrictions_account ON account_restrictions (account_id, created_at DESC);
