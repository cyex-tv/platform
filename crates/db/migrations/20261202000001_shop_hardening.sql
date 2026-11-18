-- Shop hardening (ADR 0016). Additive: a new order status, four nullable order columns, a link
-- table between orders and the entitlements they cover, a partial unique index and an index for
-- the webhook-event retention.

-- A staff refund is first recorded as requested, then Paddle is asked outside the order's lock,
-- then the order is finished as refunded (or put back to paid when Paddle says no).
ALTER TABLE shop_orders
    DROP CONSTRAINT shop_orders_status_check,
    ADD CONSTRAINT shop_orders_status_check
        CHECK (status IN ('pending', 'paid', 'refund_requested', 'refunded', 'failed')),
    -- Paddle's hosted checkout link, so a repeated checkout can hand out the same transaction.
    ADD COLUMN checkout_url text,
    -- Why a failed order failed. A late payment is still taken for `replaced` and `expired`
    -- (the buyer may pay an old checkout tab); `canceled` came from Paddle.
    ADD COLUMN failed_reason text CHECK (failed_reason IN
        ('canceled', 'provider_error', 'transaction_not_saved', 'replaced', 'expired')),
    ADD COLUMN refund_requested_at timestamptz,
    -- Paddle rejected the refund after the cosmetics were taken back: staff look at it by hand.
    ADD COLUMN refund_rejected_at timestamptz;

-- At most one pending order per buyer, item and recipient, so pending checkouts can't hoard stock.
-- Older duplicates (none exist: nothing is deployed) are failed first.
UPDATE shop_orders o
   SET status = 'failed', failed_at = now(), failed_reason = 'replaced', updated_at = now()
 WHERE o.status = 'pending'
   AND EXISTS (SELECT 1 FROM shop_orders n
                WHERE n.status = 'pending' AND n.account_id = o.account_id
                  AND n.item_id = o.item_id AND n.recipient_id = o.recipient_id AND n.id > o.id);
CREATE UNIQUE INDEX shop_orders_one_pending ON shop_orders (account_id, item_id, recipient_id)
    WHERE status = 'pending';

-- Every cosmetic an order covers, whether the order made the entitlement or found it owned
-- already. A refund takes a cosmetic back only when no other paid order (and no code or staff
-- grant) still covers it.
CREATE TABLE shop_order_entitlements (
    order_id    uuid NOT NULL REFERENCES shop_orders (id) ON DELETE CASCADE,
    account_id  uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    kind        text NOT NULL CHECK (kind IN ('badge', 'paint')),
    cosmetic_id uuid NOT NULL,
    PRIMARY KEY (order_id, kind, cosmetic_id)
);
CREATE INDEX shop_order_entitlements_owner
    ON shop_order_entitlements (account_id, kind, cosmetic_id);

INSERT INTO shop_order_entitlements (order_id, account_id, kind, cosmetic_id)
SELECT order_id, account_id, 'badge', badge_id FROM badge_entitlements WHERE order_id IS NOT NULL
UNION ALL
SELECT order_id, account_id, 'paint', paint_id FROM paint_entitlements WHERE order_id IS NOT NULL
ON CONFLICT DO NOTHING;

-- The worker deletes handled webhook events after 90 days.
CREATE INDEX paddle_events_received ON paddle_events (received_at);
