// A set's capacity (ADR 0017, amendment 2026/11/22): what the capacity field takes and says. The
// API's setCapacityRange tells the range: from the emotes the set holds (at least 1) to the owner's
// role limit (1.000 unless a role raises it), or 1.000.000 for staff.
import { count } from '$lib/format';

export type CapacityRange = { min: number; max: number; ownerMax: number; used: number; staff: boolean };

/** The most any set can have (the API's STAFF_CAPACITY_MAX). */
export const CAPACITY_MAX = 1_000_000;

/** What is wrong with the typed capacity, or null. */
export function capacityProblem(raw: string, range: CapacityRange): string | null {
  const t = raw.trim().replaceAll('.', '');
  if (!t) return 'Type a number of slots.';
  if (!/^\d+$/.test(t)) return 'A whole number of slots.';
  const n = Number(t);
  if (n < range.min) return range.used > 0 && range.min === range.used ? `It holds ${count(range.used)} emotes: remove some first, or pick ${count(range.used)} or more.` : `At least ${count(range.min)}.`;
  if (n > range.max) return `Up to ${count(range.max)}.`;
  return null;
}

/** The typed capacity as a number (dots allowed as thousands marks), or null. */
export function capacityValue(raw: string): number | null {
  const t = raw.trim().replaceAll('.', '');
  return /^\d+$/.test(t) ? Number(t) : null;
}

/** The line under the field: "24 to 1.000 slots: it holds 24." */
export function rangeHint(range: CapacityRange): string {
  const span = `${count(Math.max(1, range.min))} to ${count(range.max)} slots`;
  const held = range.used ? `: it holds ${count(range.used)}` : '';
  const staff = range.staff && range.max > range.ownerMax ? ` Above ${count(range.ownerMax)} only as staff.` : '';
  return `${span}${held}.${staff}`;
}

// A personal set's slots (ADR 0038): 3, or 10 with a subscriber role, or what staff set by hand.

/** Slots of a personal set without a subscriber role, and with one. */
export const PERSONAL_SLOTS = 3;
export const SUBSCRIBER_SLOTS = 10;

/**
 * Whether cyex.tv sells a subscription that gives the subscriber slots. It does not yet (the shop
 * sells badges and paints only), so the upsell line stays hidden until it does.
 */
export const SUBSCRIPTION_OFFERED = false;

/** "2 of 3 slots", or "10 of 3 slots" when the set holds more than it may now. */
export function slotsLine(used: number, capacity: number): string {
  return `${count(used)} of ${count(capacity)} slots`;
}

/** The sentence under it: staff override, over the limit, or the subscriber upsell; null when there is nothing to add. */
export function slotsNote(p: { used: number; capacity: number; override: number | null; offered?: boolean }): string | null {
  if (p.used > p.capacity) return `It holds ${count(p.used)}, more than it may now. Nothing is removed; remove emotes to add new ones.`;
  if (p.override != null) return 'Staff set this number.';
  if ((p.offered ?? SUBSCRIPTION_OFFERED) && p.capacity < SUBSCRIBER_SLOTS) return `Subscribers get ${count(SUBSCRIBER_SLOTS)} slots.`;
  return null;
}
