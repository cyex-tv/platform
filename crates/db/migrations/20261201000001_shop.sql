-- The cosmetics shop and redeem codes (ADR 0016). Additive: new tables, two nullable columns on
-- each entitlement table, and a new entitlement source `code`.
-- Payments go through cyex.tv's own Paddle account. No card data is ever stored here; an order
-- only keeps Paddle's transaction and adjustment IDs and the amounts.

-- What staff sell: one or more badges and paints as a bundle.
CREATE TABLE shop_items (
    id              uuid PRIMARY KEY,
    name            text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    description     text NOT NULL DEFAULT '' CHECK (char_length(description) <= 500),
    -- For display; Paddle's price is what the buyer pays.
    price_cents     integer NOT NULL CHECK (price_cents >= 0),
    currency        text NOT NULL DEFAULT 'EUR' CHECK (currency ~ '^[A-Z]{3}$'),
    paddle_price_id text CHECK (paddle_price_id ~ '^pri_[a-z0-9]{1,64}$'),
    active_from     timestamptz,
    active_to       timestamptz,
    -- Most copies that can be sold. NULL: no limit.
    stock_limit     integer CHECK (stock_limit > 0),
    visible         boolean NOT NULL DEFAULT false,
    sort_order      integer NOT NULL DEFAULT 0,
    created_by      uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    -- Deleted items stay for their orders and code batches; they are never listed again.
    deleted_at      timestamptz,
    CHECK (active_from IS NULL OR active_to IS NULL OR active_to > active_from)
);
CREATE INDEX shop_items_listed ON shop_items (sort_order, id) WHERE deleted_at IS NULL;

-- The cosmetics in an item. Deleting a cosmetic takes it out of every item.
CREATE TABLE shop_item_cosmetics (
    item_id  uuid NOT NULL REFERENCES shop_items (id) ON DELETE CASCADE,
    badge_id uuid REFERENCES badges (id) ON DELETE CASCADE,
    paint_id uuid REFERENCES paints (id) ON DELETE CASCADE,
    CHECK ((badge_id IS NULL) <> (paint_id IS NULL)),
    UNIQUE (item_id, badge_id),
    UNIQUE (item_id, paint_id)
);
CREATE INDEX shop_item_cosmetics_item ON shop_item_cosmetics (item_id);

CREATE TABLE shop_orders (
    id                      uuid PRIMARY KEY,
    -- Who pays, and who gets the cosmetics (the same account unless it is a gift).
    account_id              uuid REFERENCES accounts (id) ON DELETE SET NULL,
    recipient_id            uuid REFERENCES accounts (id) ON DELETE SET NULL,
    item_id                 uuid NOT NULL REFERENCES shop_items (id),
    item_name               text NOT NULL,
    status                  text NOT NULL DEFAULT 'pending'
                                CHECK (status IN ('pending', 'paid', 'refunded', 'failed')),
    -- The item's price when the checkout started.
    amount_cents            integer NOT NULL CHECK (amount_cents >= 0),
    currency                text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    provider                text NOT NULL DEFAULT 'paddle' CHECK (provider IN ('paddle')),
    provider_transaction_id text UNIQUE,
    -- What the transaction was made for, checked against Paddle's completion: the price, and
    -- the currency Paddle gave the transaction.
    paddle_price_id         text,
    provider_currency       text,
    -- What Paddle says was paid (lowest denomination, tax included).
    paid_total_cents        bigint,
    paid_currency           text,
    provider_adjustment_id  text,
    refund_reason           text,
    created_at              timestamptz NOT NULL DEFAULT now(),
    updated_at              timestamptz NOT NULL DEFAULT now(),
    paid_at                 timestamptz,
    refunded_at             timestamptz,
    failed_at               timestamptz
);
CREATE INDEX shop_orders_account ON shop_orders (account_id, id DESC);
CREATE INDEX shop_orders_recipient ON shop_orders (recipient_id, id DESC);
CREATE INDEX shop_orders_item_status ON shop_orders (item_id, status);
CREATE INDEX shop_orders_status ON shop_orders (status, id DESC);

-- Paddle webhook events already handled, so a redelivered or replayed event does nothing.
CREATE TABLE paddle_events (
    event_id    text PRIMARY KEY,
    event_type  text NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now()
);

-- Redeem codes: staff make a batch; each code is stored only as its sha256.
CREATE TABLE code_batches (
    id            uuid PRIMARY KEY,
    name          text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    prefix        text NOT NULL CHECK (prefix ~ '^[A-Z0-9]{1,12}$'),
    -- What a code gives: the item's cosmetics (as they are when redeemed), and/or the cosmetics
    -- in code_batch_cosmetics.
    item_id       uuid REFERENCES shop_items (id),
    code_count    integer NOT NULL CHECK (code_count BETWEEN 1 AND 1000),
    uses_per_code integer NOT NULL CHECK (uses_per_code BETWEEN 1 AND 10000),
    expires_at    timestamptz,
    disabled_at   timestamptz,
    disabled_by   uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_by    uuid REFERENCES accounts (id) ON DELETE SET NULL,
    created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE code_batch_cosmetics (
    batch_id uuid NOT NULL REFERENCES code_batches (id) ON DELETE CASCADE,
    badge_id uuid REFERENCES badges (id) ON DELETE CASCADE,
    paint_id uuid REFERENCES paints (id) ON DELETE CASCADE,
    CHECK ((badge_id IS NULL) <> (paint_id IS NULL)),
    UNIQUE (batch_id, badge_id),
    UNIQUE (batch_id, paint_id)
);
CREATE INDEX code_batch_cosmetics_batch ON code_batch_cosmetics (batch_id);

CREATE TABLE redeem_codes (
    id         uuid PRIMARY KEY,
    batch_id   uuid NOT NULL REFERENCES code_batches (id) ON DELETE CASCADE,
    -- sha256 of the normalized code. The plain code is shown once, when the batch is made.
    code_hash  bytea NOT NULL UNIQUE CHECK (length(code_hash) = 32),
    uses       integer NOT NULL DEFAULT 0 CHECK (uses >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX redeem_codes_batch ON redeem_codes (batch_id);

-- One account redeems one code once.
CREATE TABLE code_redemptions (
    code_id     uuid NOT NULL REFERENCES redeem_codes (id) ON DELETE CASCADE,
    account_id  uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    redeemed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (code_id, account_id)
);

-- Entitlements: a new source, and where a purchased or redeemed one came from. A refund revokes
-- exactly the entitlements its order made.
ALTER TABLE badge_entitlements
    DROP CONSTRAINT badge_entitlements_source_check,
    ADD CONSTRAINT badge_entitlements_source_check
        CHECK (source IN ('grant', 'purchase', 'event', 'code')),
    ADD COLUMN order_id uuid REFERENCES shop_orders (id) ON DELETE SET NULL,
    ADD COLUMN code_id uuid REFERENCES redeem_codes (id) ON DELETE SET NULL;
ALTER TABLE paint_entitlements
    DROP CONSTRAINT paint_entitlements_source_check,
    ADD CONSTRAINT paint_entitlements_source_check
        CHECK (source IN ('grant', 'purchase', 'event', 'code')),
    ADD COLUMN order_id uuid REFERENCES shop_orders (id) ON DELETE SET NULL,
    ADD COLUMN code_id uuid REFERENCES redeem_codes (id) ON DELETE SET NULL;
CREATE INDEX badge_entitlements_order ON badge_entitlements (order_id) WHERE order_id IS NOT NULL;
CREATE INDEX paint_entitlements_order ON paint_entitlements (order_id) WHERE order_id IS NOT NULL;
