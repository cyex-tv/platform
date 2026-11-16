-- cyex.tv: the Phase 2 data model (PLAN.md Phase 2, ADR 0009, ADR 0010).
--
-- Conventions (ADR 0002):
-- - IDs are ULIDs stored as uuid, made by the services. The outbox is the one exception: its IDs
--   are a bigint sequence, so clients can resume from "the last event I saw".
-- - Times are timestamptz.
-- - Small closed sets of values are text with a CHECK, not Postgres enums, so adding a value is a
--   plain forward migration.
-- - The CHECKs are a backstop. The services validate input first and answer with named errors.

-- Accounts and identities (ADR 0009)

CREATE TABLE accounts (
    id            uuid PRIMARY KEY,
    created_at    timestamptz NOT NULL DEFAULT now(),
    restricted_at timestamptz,
    deleted_at    timestamptz
);

-- A Twitch or Kick account linked to a cyex.tv account. Keyed by the platform's numeric ID, never
-- the login. One identity per platform per account.
CREATE TABLE identities (
    platform         text NOT NULL CHECK (platform IN ('twitch', 'kick')),
    platform_user_id text NOT NULL CHECK (platform_user_id ~ '^[0-9]{1,20}$'),
    account_id       uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    login            text NOT NULL CHECK (char_length(login) BETWEEN 1 AND 64),
    display_name     text NOT NULL CHECK (char_length(display_name) BETWEEN 1 AND 64),
    avatar           text CHECK (avatar IS NULL OR char_length(avatar) <= 2048),
    linked_at        timestamptz NOT NULL DEFAULT now(),
    refreshed_at     timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (platform, platform_user_id),
    UNIQUE (account_id, platform)
);

CREATE INDEX identities_login ON identities (platform, lower(login));

-- Site and app sessions. Only the sha256 of the token is stored.
CREATE TABLE sessions (
    id_hash      bytea PRIMARY KEY CHECK (length(id_hash) = 32),
    account_id   uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind         text NOT NULL CHECK (kind IN ('site', 'app')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL,
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    CHECK (expires_at > created_at)
);

CREATE INDEX sessions_account ON sessions (account_id);
CREATE INDEX sessions_expires ON sessions (expires_at);

-- Files and emotes

-- One stored image, shared by every emote uploaded with the same bytes (Phase 3 fills it).
CREATE TABLE files (
    id          uuid PRIMARY KEY,
    sha256      bytea NOT NULL UNIQUE CHECK (length(sha256) = 32),
    -- 64-bit perceptual hash, for the blocklist.
    dhash       bigint,
    mime        text NOT NULL,
    width       integer NOT NULL CHECK (width BETWEEN 1 AND 1000),
    height      integer NOT NULL CHECK (height BETWEEN 1 AND 1000),
    frame_count integer NOT NULL DEFAULT 1 CHECK (frame_count BETWEEN 1 AND 1000),
    variants    jsonb NOT NULL DEFAULT '[]',
    storage_key text NOT NULL UNIQUE,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX files_dhash ON files (dhash) WHERE dhash IS NOT NULL;

CREATE TABLE emotes (
    id               uuid PRIMARY KEY,
    -- Null once the owner's account is gone; the emote can stay in other people's sets.
    owner_account_id uuid REFERENCES accounts (id) ON DELETE SET NULL,
    file_id          uuid REFERENCES files (id),
    name             text NOT NULL CHECK (name ~ '^\S{1,100}$'),
    zero_width       boolean NOT NULL DEFAULT false,
    private          boolean NOT NULL DEFAULT false,
    nsfw             boolean NOT NULL DEFAULT false,
    animated         boolean NOT NULL DEFAULT false,
    state            text NOT NULL DEFAULT 'pending'
                     CHECK (state IN ('pending', 'approved', 'rejected', 'taken_down')),
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    deleted_at       timestamptz
);

CREATE INDEX emotes_owner ON emotes (owner_account_id);
CREATE INDEX emotes_state ON emotes (state, created_at);
CREATE INDEX emotes_name_lower ON emotes (lower(name));
-- Substring and similarity search on names (ADR 0004). pg_trgm ignores case.
CREATE INDEX emotes_name_trgm ON emotes USING gin (name gin_trgm_ops);

CREATE TABLE emote_tags (
    emote_id uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    tag      text NOT NULL CHECK (tag ~ '^[a-z0-9_]{1,30}$'),
    PRIMARY KEY (emote_id, tag)
);

CREATE INDEX emote_tags_tag ON emote_tags (tag);

-- Sets

CREATE TABLE emote_sets (
    id               uuid PRIMARY KEY,
    -- Null only for the global set.
    owner_account_id uuid REFERENCES accounts (id) ON DELETE CASCADE,
    name             text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    kind             text NOT NULL CHECK (kind IN ('channel', 'personal', 'global')),
    capacity         integer NOT NULL CHECK (capacity BETWEEN 1 AND 100000),
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    CHECK ((kind = 'global') = (owner_account_id IS NULL))
);

CREATE INDEX emote_sets_owner ON emote_sets (owner_account_id);
-- One personal set per account (D21) and one global set (D12).
CREATE UNIQUE INDEX emote_sets_one_personal ON emote_sets (owner_account_id) WHERE kind = 'personal';
CREATE UNIQUE INDEX emote_sets_one_global ON emote_sets ((true)) WHERE kind = 'global';

-- An emote in a set. `name` is what it's typed as in this set; it starts as the emote's own name
-- and doesn't follow later renames of the emote.
CREATE TABLE emote_set_members (
    set_id     uuid NOT NULL REFERENCES emote_sets (id) ON DELETE CASCADE,
    emote_id   uuid NOT NULL REFERENCES emotes (id) ON DELETE CASCADE,
    name       text NOT NULL CHECK (name ~ '^\S{1,100}$'),
    zero_width boolean NOT NULL DEFAULT false,
    added_by   uuid REFERENCES accounts (id) ON DELETE SET NULL,
    added_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (set_id, emote_id),
    UNIQUE (set_id, name)
);

CREATE INDEX emote_set_members_emote ON emote_set_members (emote_id);

CREATE TABLE set_editors (
    set_id     uuid NOT NULL REFERENCES emote_sets (id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    granted_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (set_id, account_id)
);

CREATE INDEX set_editors_account ON set_editors (account_id);

-- Channels. One row per platform channel; an account can have a Twitch and a Kick channel. The
-- per-channel settings live on the row (PLAN.md listed them as `channel_settings`; it's 1:1).
CREATE TABLE channels (
    id                    uuid PRIMARY KEY,
    account_id            uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    platform              text NOT NULL CHECK (platform IN ('twitch', 'kick')),
    platform_channel_id   text NOT NULL CHECK (platform_channel_id ~ '^[0-9]{1,20}$'),
    login                 text NOT NULL CHECK (char_length(login) BETWEEN 1 AND 64),
    active_set_id         uuid REFERENCES emote_sets (id) ON DELETE SET NULL,
    allow_personal_emotes boolean NOT NULL DEFAULT true,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    UNIQUE (platform, platform_channel_id)
);

CREATE INDEX channels_account ON channels (account_id);
CREATE INDEX channels_login ON channels (platform, lower(login));

-- Roles and permissions (ADR 0010)

CREATE TABLE roles (
    id          uuid PRIMARY KEY,
    name        text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 50),
    description text CHECK (description IS NULL OR char_length(description) <= 500),
    -- Higher is more senior.
    position    integer NOT NULL CHECK (position BETWEEN 0 AND 1000000),
    created_by  uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX roles_name ON roles (lower(name));

CREATE TABLE role_permissions (
    role_id    uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    -- One of the names in crates/shared/src/permissions.rs.
    permission text NOT NULL CHECK (permission ~ '^[a-z_]+\.[a-z_]+$'),
    PRIMARY KEY (role_id, permission)
);

CREATE INDEX role_permissions_permission ON role_permissions (permission);

CREATE TABLE account_roles (
    account_id uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    role_id    uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    granted_by uuid REFERENCES accounts (id) ON DELETE SET NULL,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, role_id)
);

CREATE INDEX account_roles_role ON account_roles (role_id);

-- Audit log: every change to roles, grants and moderation state. Rows are never updated.
CREATE TABLE audit_log (
    id               uuid PRIMARY KEY,
    -- Null for changes the system makes on its own.
    actor_account_id uuid REFERENCES accounts (id) ON DELETE SET NULL,
    action           text NOT NULL CHECK (action ~ '^[a-z_]+\.[a-z_]+$'),
    target_kind      text NOT NULL,
    target_id        text NOT NULL,
    before_json      jsonb,
    after_json       jsonb,
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX audit_log_created ON audit_log (created_at DESC);
CREATE INDEX audit_log_target ON audit_log (target_kind, target_id, created_at DESC);
CREATE INDEX audit_log_actor ON audit_log (actor_account_id, created_at DESC);

-- Events outbox (ADR 0005). Written in the same transaction as the change; NOTIFY wakes the events
-- service and the api's subscription feed. Cleaned after 48 h.
CREATE TABLE events_outbox (
    id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    type       text NOT NULL CHECK (type ~ '^[a-z_]+(\.[a-z_]+)?$'),
    -- For example `channel:twitch:123`, `set:<id>`, `user:kick:456`.
    topics     text[] NOT NULL CHECK (cardinality(topics) >= 1),
    set_id     uuid,
    payload    jsonb NOT NULL DEFAULT '{}',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX events_outbox_topics ON events_outbox USING gin (topics);
CREATE INDEX events_outbox_created ON events_outbox (created_at);

CREATE FUNCTION events_outbox_notify() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('cyex_tv_events', NEW.id::text);
    RETURN NEW;
END;
$$;

CREATE TRIGGER events_outbox_notify
    AFTER INSERT ON events_outbox
    FOR EACH ROW EXECUTE FUNCTION events_outbox_notify();
