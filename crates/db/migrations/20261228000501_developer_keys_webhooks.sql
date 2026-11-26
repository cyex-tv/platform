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
-- - `failed_deliveries` counts deliveries in a row that used every attempt and failed; the worker
--   turns the webhook off at 5 (`disabled_reason = 'failures'`) and tells the channel's owner.
CREATE TABLE webhooks (
    id                 uuid PRIMARY KEY,
    channel_id         uuid NOT NULL REFERENCES channels (id) ON DELETE CASCADE,
    topic              text NOT NULL CHECK (topic ~ '^channel:(twitch|kick):[0-9]{1,20}$'),
    url                text NOT NULL CHECK (char_length(url) <= 2000 AND url LIKE 'https://%'),
    event_types        text[] NOT NULL CHECK (cardinality(event_types) BETWEEN 1 AND 20),
    secret             text NOT NULL CHECK (char_length(secret) BETWEEN 32 AND 128),
    enabled            boolean NOT NULL DEFAULT true,
    disabled_reason    text CHECK (disabled_reason IN ('manual', 'failures')),
    failed_deliveries  integer NOT NULL DEFAULT 0 CHECK (failed_deliveries >= 0),
    created_by         uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX webhooks_channel ON webhooks (channel_id, created_at);
CREATE INDEX webhooks_live_topic ON webhooks (topic) WHERE enabled;

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

-- Every outbox event that a live webhook subscribes to becomes a delivery and a
-- `webhook.deliver` job, in the transaction that wrote the event: nothing is missed or sent twice
-- by a reader that falls behind. Channel events match the webhook's channel topic; the cosmetics
-- catalog's events (topic `cosmetics`) reach every webhook that asks for them.
CREATE FUNCTION webhooks_fan_out() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    WITH made AS (
        INSERT INTO webhook_deliveries (webhook_id, event_id, event_type, body)
        SELECT w.id, NEW.id, NEW.type,
               jsonb_build_object('id', NEW.id::text, 'type', NEW.type,
                                  'created_at', NEW.created_at, 'data', NEW.payload)
          FROM webhooks w
         WHERE w.enabled
           AND NEW.type = ANY (w.event_types)
           AND (w.topic = ANY (NEW.topics)
                OR (NEW.type LIKE 'cosmetic.%' AND 'cosmetics' = ANY (NEW.topics)))
        RETURNING id
    )
    INSERT INTO jobs (kind, payload, max_attempts)
    SELECT 'webhook.deliver', jsonb_build_object('delivery_id', made.id), 8 FROM made;
    RETURN NULL;
END;
$$;

CREATE TRIGGER events_outbox_webhooks
    AFTER INSERT ON events_outbox FOR EACH ROW EXECUTE FUNCTION webhooks_fan_out();
