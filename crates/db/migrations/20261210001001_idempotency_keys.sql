-- Idempotency keys (additive): a client may send an `idempotencyKey` (a UUID it makes per user
-- action) with startCheckout, redeemCode, approveEmote and reviewEmotes. The first request with a
-- key claims the row (`response` NULL while it runs) and stores its answer; a repeat with the same
-- key gets that answer back instead of acting again. `request_hash` is a sha256 of what was asked
-- (never the request itself: a redeem code is never stored in plain text), so the same key with
-- another payload is refused. Rows live 24 hours; the worker's clean-up removes older ones.
CREATE TABLE idempotency_keys (
    account_id   uuid NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    operation    text NOT NULL CHECK (char_length(operation) <= 64),
    key          uuid NOT NULL,
    request_hash bytea NOT NULL CHECK (length(request_hash) = 32),
    response     jsonb,
    created_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, operation, key)
);

CREATE INDEX idempotency_keys_created ON idempotency_keys (created_at);
