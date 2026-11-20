# 0023: Idempotency keys for requests that must not run twice

Status: accepted (owner, 2026/11/20)

## Context
A checkout start, a code redeem or a review verdict that times out on the client may still have
been done. The website could only say "check before trying again". Retrying blindly could start a
second order (after the first was paid), or answer `already_redeemed` / `not_pending` for something
that worked.

## Decision
- `startCheckout`, `redeemCode`, `approveEmote` and `reviewEmotes` take an optional
  `idempotencyKey` (a UUID). Additive: without it nothing changes.
- The server keeps `(account, operation, key) → (sha256 of the request, answer)` in
  `idempotency_keys` for 24 hours (the worker's clean-up removes older rows; erased with the
  account). Only the hash of the request is stored, never the request (a redeem code stays
  unstored).
- The first request claims the key (a row with no answer), runs, and stores its answer. A repeat:
  - same request, answer stored: gets the stored answer, nothing runs again;
  - same request, still running: `idempotency_in_progress` (409), try again in a moment;
  - another request: `idempotency_key_reused` (409).
- A request that fails gives its key back, so retrying it acts again.
- A request that worked but whose answer couldn't be stored keeps its key claimed: repeats are told
  it is in progress until the key expires. Never twice beats a clean retry.
- The stored answer is the answer as it was (a checkout's order is `PENDING` there); clients poll
  `order(id)` as before.
- The website makes one key per user action (`$lib/idempotency.ts`): the same purchase (item and
  recipient) or the same code tried again sends the same key until a request answers.

## Consequences
- A retry after a timeout is safe, and the timeout sentences say "try again" instead of "check
  first".
- One extra small write per keyed request. The table is small (24 hours of keyed actions).
