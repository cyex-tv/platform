-- Public profiles (additive): a role can be shown on its holders' public profiles, like 7TV's
-- role chips. Off for every role until someone with roles.manage turns it on, so staff-only roles
-- stay hidden.
ALTER TABLE roles ADD COLUMN public boolean NOT NULL DEFAULT false;
