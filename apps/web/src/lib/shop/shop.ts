// The shop's words and numbers, pure so they are tested: prices in the cyex number format, redeem
// codes as people type them, what each error code says to a person, what an order's state reads
// as. Nothing here talks to the network.
import { ApiError } from '$lib/api';
import { COPY, unreachableMessage } from '$lib/components/problem';
import { count, date } from '$lib/format';

const SYMBOL: Record<string, string> = { EUR: '€', USD: '$', GBP: '£' };

/** `499, 'EUR'` → `4,99 €`; `0` → `Free`; `123456, 'USD'` → `1.234,56 $`; other currencies keep their code. */
export function price(cents: number, currency: string): string {
  if (cents <= 0) return 'Free';
  const whole = Math.floor(cents / 100);
  const rest = String(cents % 100).padStart(2, '0');
  return `${count(whole)},${rest} ${SYMBOL[currency.toUpperCase()] ?? currency.toUpperCase()}`;
}

/** The price in cents from what someone typed (`4,99`, `4.99`, `5`), or null when it isn't a price. */
export function centsFrom(text: string): number | null {
  const t = text.trim().replace(/\s/g, '');
  if (!/^\d{1,6}([.,]\d{1,2})?$/.test(t)) return null;
  const [w, f = ''] = t.split(/[.,]/);
  return Number(w) * 100 + Number(f.padEnd(2, '0'));
}

/** `499` → `4,99`: a price as it goes into a field. */
export function centsText(cents: number): string {
  return `${Math.floor(cents / 100)},${String(cents % 100).padStart(2, '0')}`;
}

/** A code as the API wants it: upper case, no spaces, no dashes (case, spaces and dashes don't matter). */
export function normaliseCode(raw: string): string {
  return raw.toUpperCase().replace(/[\s\-‐-―−]/g, '');
}

/** Whether a normalised code can be a code at all: letters and digits, 6 to 64 of them. */
export function plausibleCode(normalised: string): boolean {
  return /^[A-Z0-9]{6,64}$/.test(normalised);
}

/** A code as shown while typing: upper case, spaces turned into dashes. Nothing is dropped. */
export function tidyCode(raw: string): string {
  return raw.toUpperCase().replace(/\s+/g, '-').replace(/-{2,}/g, '-');
}

// The site's one wording for a dropped network and an unplaceable failure (problem.ts), with the shop's own reassurance.
const SERVER = COPY.server.title;

/** What each error of the shop's operations says. Anything else falls back to the API's own sentence. */
const SHOP: Record<string, string> = {
  signed_out: 'Sign in to buy.',
  forbidden: 'Your account can’t do that.',
  item_unavailable: 'That item isn’t on sale right now.',
  sold_out: 'That item just sold out.',
  already_owned: 'You own everything in it already. You can still give it to someone.',
  recipient_not_found: 'No cyex.tv account has that login on that platform. They need to have signed in to cyex.tv once.',
  checkout_unavailable: 'Buying isn’t available yet. Try again later.',
  payment_provider_error: 'The payment provider didn’t answer as expected. Nothing was charged. Try again in a moment.',
  checkout_busy: 'A checkout for this is still starting. Try again in a minute.',
  checkout_failed: 'The checkout couldn’t be started. Nothing was charged. Try again in a moment.',
  unknown_client: 'This browser isn’t one cyex.tv can sell to. Reload the page and try again.',
  rate_limited: 'That’s a lot of tries. Wait a while, then try again.',
  not_found: 'That doesn’t exist any more.',
};

/** The sentence for an error from startCheckout (and the other shop reads). `gift` tunes already_owned. */
export function shopMessage(err: unknown, gift = false): string {
  if (!(err instanceof ApiError)) return `${SERVER} Nothing was charged.`;
  if (err.code === 'offline') return unreachableMessage();
  if (err.code === 'already_owned' && gift) return 'They own everything in it already.';
  return SHOP[err.code] ?? err.message;
}

const REDEEM: Record<string, string> = {
  bad_code: 'That code doesn’t work. Check it for typos.',
  code_disabled: 'That code was turned off.',
  code_expired: 'That code has expired.',
  already_redeemed: 'You used that code already.',
  code_used_up: 'That code has been used up.',
  signed_out: 'Sign in to redeem a code.',
  rate_limited: 'That’s a lot of tries. Wait a while, then try again.',
};

/** Minutes out of the API's "Try again in 7 min." for a lockout, or null. */
export function lockoutMinutes(message: string): number | null {
  const m = /in (\d{1,4}) min/.exec(message);
  return m ? Number(m[1]) : null;
}

/** The sentence for a failed redeem; a lockout says when to try again. */
export function redeemMessage(err: unknown): string {
  if (!(err instanceof ApiError)) return `${SERVER} Try again in a moment.`;
  if (err.code === 'locked_out') {
    const n = lockoutMinutes(err.message);
    return n === null
      ? 'Too many wrong codes. Try again in a while.'
      : `Too many wrong codes. You can try again in ${count(n)} ${n === 1 ? 'minute' : 'minutes'}.`;
  }
  if (err.code === 'offline') return unreachableMessage();
  return REDEEM[err.code] ?? err.message;
}

const STAFF: Record<string, string> = {
  not_refundable: 'Only paid orders can be refunded, and only once.',
  already_refunded: 'That order was refunded already.',
  refund_in_progress: 'A refund for that order is already in progress. Check again in a minute.',
  refund_failed: 'The payment provider refused the refund. Nothing changed here. Try again, or refund it in Paddle.',
  bad_reason: 'Give a reason of 1 to 500 characters.',
  bad_name: 'Names are 1 to 64 characters.',
  bad_description: 'The description is up to 500 characters.',
  bad_price: 'The price is 0 to 1.000,00.',
  bad_currency: 'Use a three-letter currency code like EUR.',
  bad_price_id: 'Use a Paddle price ID like pri_01abc.',
  bad_time: 'Times need a date and time, and the sale has to end after it starts.',
  bad_stock: 'The stock is 1 or more, or empty for no limit.',
  bad_cosmetics: 'Pick 1 to 10 badges and paints in all.',
  forbidden: 'Your account can’t do that.',
  rate_limited: 'That’s a lot of changes. Wait a while, then try again.',
};

/** The sentence for a failed staff write (items, refunds, code batches). */
export function staffMessage(err: unknown): string {
  if (!(err instanceof ApiError)) return 'That didn’t work. Nothing was changed.';
  return STAFF[err.code] ?? err.message;
}

export type OrderStatus = 'PENDING' | 'PAID' | 'REFUNDED' | 'FAILED';

/** What an order's state reads as, to the person it is shown to. Refund reasons are never part of it. */
export function orderStatusText(status: OrderStatus): string {
  switch (status) {
    case 'PENDING':
      return 'Waiting for payment';
    case 'PAID':
      return 'Paid';
    case 'REFUNDED':
      return 'Refunded';
    case 'FAILED':
      return 'Not completed';
  }
}

/** Whether polling an order can stop. */
export const settled = (status: OrderStatus) => status !== 'PENDING';

type Timed = {
  activeFrom?: string | null;
  activeTo?: string | null;
  stockLeft?: number | null;
  visible: boolean;
  deleted?: boolean;
};

export type ItemState = 'deleted' | 'hidden' | 'scheduled' | 'ended' | 'sold_out' | 'on_sale';

/** What a staff list calls an item: deleted, hidden, scheduled, ended, sold out or on sale. */
export function itemState(i: Timed, now = Date.now()): ItemState {
  if (i.deleted) return 'deleted';
  if (!i.visible) return 'hidden';
  if (i.activeFrom && Date.parse(i.activeFrom) > now) return 'scheduled';
  if (i.activeTo && Date.parse(i.activeTo) <= now) return 'ended';
  if (i.stockLeft === 0) return 'sold_out';
  return 'on_sale';
}

export const ITEM_STATE_TEXT: Record<ItemState, string> = {
  deleted: 'Deleted',
  hidden: 'Hidden',
  scheduled: 'Scheduled',
  ended: 'Ended',
  sold_out: 'Sold out',
  on_sale: 'On sale',
};

/** `12 left`, `Sold out`, or null without a limit. */
export function stockText(i: Pick<Timed, 'stockLeft'>): string | null {
  if (i.stockLeft === null || i.stockLeft === undefined) return null;
  return i.stockLeft === 0 ? 'Sold out' : `${count(i.stockLeft)} left`;
}

/** `On sale until 2026/12/01`, or null without an end. */
export function saleEndText(i: Pick<Timed, 'activeTo'>, now = Date.now()): string | null {
  if (!i.activeTo) return null;
  return Date.parse(i.activeTo) <= now ? `Ended ${date(i.activeTo)}` : `On sale until ${date(i.activeTo)}`;
}

/** The text file of a new batch's codes: one per line. */
export function codesText(codes: readonly string[]): string {
  return `${codes.join('\n')}\n`;
}

/** `cyex-tv-codes-<name>.txt`, with the name reduced to letters, digits and dashes. */
export function codesFilename(name: string): string {
  const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 40);
  return `cyex-tv-codes${slug ? `-${slug}` : ''}.txt`;
}

/** A local `datetime-local` value (`2026-12-01T18:30`) from an RFC 3339 time, or ''. */
export function toLocalInput(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  const p = (v: number) => String(v).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** An RFC 3339 time from a `datetime-local` value (read in the reader's time zone), or null. */
export function fromLocalInput(value: string): string | null {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

const CHECKOUT_HOSTS = new Set(['cyex.tv', 'buy.paddle.com', 'sandbox-buy.paddle.com']);

/** The hosted checkout page, only when it is https on cyex.tv or Paddle's checkout hosts; else null. */
export function safeCheckoutUrl(u: string | null | undefined): string | null {
  if (!u) return null;
  try {
    const url = new URL(u);
    return url.protocol === 'https:' && !url.username && !url.password && CHECKOUT_HOSTS.has(url.hostname) ? url.href : null;
  } catch {
    return null;
  }
}
