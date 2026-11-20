// Idempotency keys: one per user action (a purchase, a redeem), sent again when that action is
// retried, so a request that may have gone through (a timeout, a dropped connection) is answered
// from the first try instead of being done twice. The API keeps a key for 24 hours.

/** A fresh key (a random UUID). */
export function newIdempotencyKey(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  // Outside a secure context randomUUID is missing; getRandomValues is still there.
  const b = crypto.getRandomValues(new Uint8Array(16));
  b[6] = (b[6] & 0x0f) | 0x40;
  b[8] = (b[8] & 0x3f) | 0x80;
  const h = Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

/**
 * Keeps one key per action: the same `what` (an item and recipient, a code) gets the same key
 * until `done()` (it worked, so the next try is a new action) or until `what` changes.
 */
export function keyKeeper() {
  let held: { what: string; key: string } | null = null;
  return {
    key(what: string): string {
      if (held?.what !== what) held = { what, key: newIdempotencyKey() };
      return held.key;
    },
    done() {
      held = null;
    },
  };
}
