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
