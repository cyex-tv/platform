-- Developer API keys (ADR 0033) and outbound webhooks (ADR 0034). Additive: four new tables, one
-- trigger on the events outbox.
--
-- API keys
-- - A key is shown once; only its sha256 is kept (like every session, ADR 0009). `prefix` is the
--   start of the key, shown so a person can tell their keys apart.
-- - A key reads public data only. It is not a session: no sign-out ends it. Revoking it, deleting
--   the account and a staff block do (the api deletes the rows), and while an account is deleted
--   or blocked its keys are refused anyway (`api_keys::find_active` joins `accounts`).
-- - `api_key_usage` counts accepted requests per key per UTC day, written in batches by the api.
CREATE TABLE api_keys (
    id            uuid PRIMARY KEY,
    account_id    uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    name          text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    prefix        text NOT NULL CHECK (char_length(prefix) BETWEEN 1 AND 32),
    key_hash      bytea NOT NULL UNIQUE CHECK (octet_length(key_hash) = 32),
    created_at    timestamptz NOT NULL DEFAULT now(),
    last_used_at  timestamptz
);

CREATE INDEX api_keys_account ON api_keys (account_id, created_at);

CREATE TABLE api_key_usage (
    key_id    uuid NOT NULL REFERENCES api_keys (id) ON DELETE CASCADE,
    day       date NOT NULL,
    requests  bigint NOT NULL DEFAULT 0 CHECK (requests >= 0),
    PRIMARY KEY (key_id, day)
);

-- Webhooks
-- - One channel's subscription: a URL and the event types it wants. `topic` is the channel's
--   outbox topic (`channel:<platform>:<id>`), kept so the trigger below matches on an index.
-- - `secret` signs every delivery (`X-Cyex-Signature`). It is needed in the clear to sign, so it
--   is stored as it is and shown only once, when made or rotated.
-- - `secret` may have a `previous_secret` for 24 h after a rotation; deliveries carry a signature
--   for each, so receivers can switch over without dropping any.
-- - `failed_deliveries` counts deliveries in a row that used every attempt and failed;
--   `window_*` count the finished ones and the failed ones since `window_started_at` (24 h). The
--   worker turns the webhook off (`disabled_reason = 'failures'`) at 5 in a row, or at 20 or more
--   failed and over 80% failed in the window, and tells the channel's owner at most once a day
--   (`disabled_notice_at`). A staff block turns its channels' webhooks off (`'blocked'`).
-- - `created_by` and the channel's owner see the whole URL (it may hold a token); editors see the
--   host only.
CREATE TABLE webhooks (
    id                 uuid PRIMARY KEY,
    channel_id         uuid NOT NULL REFERENCES channels (id) ON DELETE CASCADE,
    topic              text NOT NULL CHECK (topic ~ '^channel:(twitch|kick):[0-9]{1,20}$'),
    url                text NOT NULL CHECK (char_length(url) <= 2000 AND url LIKE 'https://%'),
    event_types        text[] NOT NULL CHECK (cardinality(event_types) BETWEEN 1 AND 20),
    secret             text NOT NULL CHECK (char_length(secret) BETWEEN 32 AND 128),
    previous_secret    text CHECK (char_length(previous_secret) BETWEEN 32 AND 128),
    previous_secret_until timestamptz,
    enabled            boolean NOT NULL DEFAULT true,
    disabled_reason    text CHECK (disabled_reason IN ('manual', 'failures', 'blocked')),
    failed_deliveries  integer NOT NULL DEFAULT 0 CHECK (failed_deliveries >= 0),
    window_started_at  timestamptz NOT NULL DEFAULT now(),
    window_finished    integer NOT NULL DEFAULT 0 CHECK (window_finished >= 0),
    window_failed      integer NOT NULL DEFAULT 0 CHECK (window_failed >= 0),
    disabled_notice_at timestamptz,
    created_by         uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX webhooks_channel ON webhooks (channel_id, created_at);
CREATE INDEX webhooks_live_topic ON webhooks (topic) WHERE enabled;
-- The live webhooks that ask for the cosmetics catalog's events (at most
-- `webhooks::MAX_COSMETIC_SUBSCRIBERS`, checked when one is saved).
CREATE INDEX webhooks_live_cosmetic ON webhooks (id)
    WHERE enabled AND event_types && ARRAY['cosmetic.created', 'cosmetic.updated', 'cosmetic.deleted'];

-- One row per delivery: what was sent and how it went. The worker keeps the latest 50 per webhook.
-- `body` is the event as it was when it happened, so a retry sends the same thing even after the
-- outbox is cleaned.
CREATE TABLE webhook_deliveries (
    id               bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    webhook_id       uuid NOT NULL REFERENCES webhooks (id) ON DELETE CASCADE,
    event_id         bigint,
    event_type       text NOT NULL,
    body             jsonb NOT NULL,
    state            text NOT NULL DEFAULT 'pending'
                     CHECK (state IN ('pending', 'delivered', 'failed')),
    attempts         integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    response_status  integer,
    error            text,
    duration_ms      integer,
    created_at       timestamptz NOT NULL DEFAULT now(),
    finished_at      timestamptz
);

CREATE INDEX webhook_deliveries_recent ON webhook_deliveries (webhook_id, id DESC);
CREATE INDEX webhook_deliveries_pending ON webhook_deliveries (webhook_id) WHERE state = 'pending';

-- Every outbox event that a live webhook subscribes to becomes a delivery and a
-- `webhook.deliver` job, in the transaction that wrote the event: nothing is missed or sent twice
-- by a reader that falls behind. Events of other types return at once. Channel events match the
-- webhook's channel topic (index `webhooks_live_topic`); the cosmetics catalog's events reach every
-- webhook that asks for them (index `webhooks_live_cosmetic`, a bounded set).
--
-- Backpressure: a webhook with 100 deliveries waiting, or a channel with 300, gets no new job; the
-- delivery is logged as failed (`dropped`) instead, so a slow receiver can't fill the queue.
CREATE FUNCTION webhooks_fan_out() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.type NOT IN ('set.emote_added', 'set.emote_removed', 'set.emote_renamed',
                        'channel.active_set_changed', 'emote.updated',
                        'cosmetic.created', 'cosmetic.updated', 'cosmetic.deleted') THEN
        RETURN NULL;
    END IF;
    WITH matched AS (
        SELECT w.id, w.channel_id FROM webhooks w
         WHERE w.enabled AND w.topic = ANY (NEW.topics) AND NEW.type = ANY (w.event_types)
        UNION ALL
        SELECT w.id, w.channel_id FROM webhooks w
         WHERE NEW.type LIKE 'cosmetic.%' AND 'cosmetics' = ANY (NEW.topics)
           AND w.enabled
           AND w.event_types && ARRAY['cosmetic.created', 'cosmetic.updated', 'cosmetic.deleted']
           AND NEW.type = ANY (w.event_types)
    ), judged AS (
        SELECT m.id,
               (SELECT count(*) FROM webhook_deliveries d
                 WHERE d.webhook_id = m.id AND d.state = 'pending') >= 100
            OR (SELECT count(*) FROM webhook_deliveries d JOIN webhooks c ON c.id = d.webhook_id
                 WHERE c.channel_id = m.channel_id AND d.state = 'pending') >= 300 AS full
          FROM matched m
    ), made AS (
        INSERT INTO webhook_deliveries (webhook_id, event_id, event_type, body, state, error, finished_at)
        SELECT j.id, NEW.id, NEW.type,
               jsonb_build_object('id', NEW.id::text, 'type', NEW.type,
                                  'created_at', NEW.created_at, 'data', NEW.payload),
               CASE WHEN j.full THEN 'failed' ELSE 'pending' END,
               CASE WHEN j.full THEN 'dropped: too many deliveries waiting' END,
               CASE WHEN j.full THEN now() END
          FROM judged j
        RETURNING id, state
    )
    INSERT INTO jobs (kind, payload, max_attempts)
    SELECT 'webhook.deliver', jsonb_build_object('delivery_id', made.id), 8
      FROM made WHERE made.state = 'pending';
    RETURN NULL;
END;
$$;

CREATE TRIGGER events_outbox_webhooks
    AFTER INSERT ON events_outbox FOR EACH ROW EXECUTE FUNCTION webhooks_fan_out();
