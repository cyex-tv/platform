-- Phase 7: reports, owner notifications and blocked accounts (additive).

-- A user reports an emote; a moderator dismisses it or takes the emote down.
CREATE TABLE reports (
    id          uuid PRIMARY KEY,
    emote_id    uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    reporter_id uuid REFERENCES accounts (id) ON DELETE SET NULL,
    reason      text NOT NULL CHECK (char_length(reason) BETWEEN 1 AND 500),
    state       text NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'dismissed', 'actioned')),
    resolved_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    resolution  text CHECK (char_length(resolution) <= 500),
    created_at  timestamptz NOT NULL DEFAULT now(),
    resolved_at timestamptz,
    CHECK ((state = 'open') = (resolved_at IS NULL))
);

-- One open report per emote and reporter.
CREATE UNIQUE INDEX reports_one_open ON reports (emote_id, reporter_id) WHERE state = 'open';
CREATE INDEX reports_open ON reports (created_at) WHERE state = 'open';
CREATE INDEX reports_reporter ON reports (reporter_id, created_at);

-- What the platform tells an account (a takedown, for now).
CREATE TABLE notifications (
    id         uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind       text NOT NULL CHECK (kind ~ '^[a-z][a-z0-9_.]{0,63}$'),
    payload    jsonb NOT NULL DEFAULT '{}',
    created_at timestamptz NOT NULL DEFAULT now(),
    read_at    timestamptz
);

CREATE INDEX notifications_account ON notifications (account_id, created_at DESC);

-- `users.restrict` can block an account outright: no sign-in, no sessions.
ALTER TABLE accounts ADD COLUMN blocked_at timestamptz;
