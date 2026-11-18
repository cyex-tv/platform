-- Restrictions follow the person, not the account (sign-off security review). Additive: a new
-- table and a trigger.
--
-- When an identity leaves an account that is restricted or blocked (unlinked, or the account
-- deleted), its platform ID is kept here with the restriction. The next account that signs in
-- with that identity, or links it, gets the restriction and block copied on, and the hold is
-- used up. Without this, a restricted user could drop the identity and sign in fresh.
CREATE TABLE identity_holds (
    platform         text NOT NULL CHECK (platform IN ('twitch', 'kick')),
    platform_user_id text NOT NULL CHECK (platform_user_id ~ '^[0-9]{1,20}$'),
    -- The account it left. No foreign key: the hold outlives it.
    account_id       uuid NOT NULL,
    restricted_at    timestamptz,
    blocked_at       timestamptz,
    held_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (platform, platform_user_id),
    CHECK (restricted_at IS NOT NULL OR blocked_at IS NOT NULL)
);

-- Every delete of an identity, whatever path it takes, keeps a hold when its account is
-- restricted or blocked.
CREATE FUNCTION identities_hold_restricted() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO identity_holds (platform, platform_user_id, account_id, restricted_at, blocked_at)
    SELECT OLD.platform, OLD.platform_user_id, a.id, a.restricted_at, a.blocked_at
      FROM accounts a
     WHERE a.id = OLD.account_id
       AND (a.restricted_at IS NOT NULL OR a.blocked_at IS NOT NULL)
    ON CONFLICT (platform, platform_user_id) DO UPDATE
       SET account_id = EXCLUDED.account_id,
           restricted_at = coalesce(identity_holds.restricted_at, EXCLUDED.restricted_at),
           blocked_at = coalesce(identity_holds.blocked_at, EXCLUDED.blocked_at),
           held_at = now();
    RETURN OLD;
END
$$;

CREATE TRIGGER identities_hold_restricted
    BEFORE DELETE ON identities
    FOR EACH ROW EXECUTE FUNCTION identities_hold_restricted();
