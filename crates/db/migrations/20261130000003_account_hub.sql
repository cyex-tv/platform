-- The account hub (additive): sessions you can list and sign out one by one, unread counts.

-- A public handle for a session. The token's hash stays the key and is never shown; existing
-- sessions get a random one.
ALTER TABLE sessions ADD COLUMN id uuid NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE sessions ADD CONSTRAINT sessions_id_key UNIQUE (id);
-- The browser or app that signed in, as it said (cut to 512 characters), to name the device.
ALTER TABLE sessions ADD COLUMN user_agent text
    CHECK (user_agent IS NULL OR char_length(user_agent) <= 512);

-- `unreadNotificationCount`.
CREATE INDEX notifications_unread ON notifications (account_id) WHERE read_at IS NULL;
