-- Per-role limits: a role can raise or lower the per-account limits of everyone who holds it
-- (`setRoleLimits`). A map from limit key (`gql_account`, `pending_emotes`, ...) to its count;
-- the api checks the keys and values before it writes. An account's limit is the most generous
-- override among its roles, else the default the api was started with. Additive: every role
-- starts with none, so nothing changes until a role manager sets one.
ALTER TABLE roles ADD COLUMN limits jsonb NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE roles ADD CONSTRAINT roles_limits_object CHECK (jsonb_typeof(limits) = 'object');
