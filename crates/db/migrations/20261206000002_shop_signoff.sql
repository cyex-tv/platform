-- Shop sign-off fixes (ADR 0016). Additive: five nullable or defaulted columns on shop_orders.

ALTER TABLE shop_orders
    -- What the order sells, as it was at checkout: fulfilment grants these, not whatever the
    -- item holds when the payment lands. NULL on orders made before this column (they fall back
    -- to the item's current cosmetics).
    ADD COLUMN badge_ids uuid[],
    ADD COLUMN paint_ids uuid[],
    -- Bought for another account. Stored at checkout, so it stays right when the buyer or the
    -- recipient is deleted later.
    ADD COLUMN is_gift boolean NOT NULL DEFAULT false,
    -- An order failed by us (replaced, expired) has its Paddle transaction cancelled, so the old
    -- checkout can't be paid. When the cancel fails, the time is kept and a late payment is
    -- still taken.
    ADD COLUMN provider_canceled_at timestamptz,
    ADD COLUMN cancel_failed_at timestamptz;

UPDATE shop_orders SET is_gift = (account_id IS DISTINCT FROM recipient_id);

-- The worker's sweep for transactions still to cancel.
CREATE INDEX shop_orders_to_cancel ON shop_orders (id)
    WHERE status = 'failed' AND failed_reason IN ('replaced', 'expired')
      AND provider_transaction_id IS NOT NULL
      AND provider_canceled_at IS NULL AND cancel_failed_at IS NULL;
