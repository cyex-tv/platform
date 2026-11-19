// Small pure parts of the public set page (/sets/<id>): the in-set search, the capacity line,
// and the words for what cloning did or why it didn't.
import { count } from '$lib/format';

type Named = { name: string; emote: { name: string } };

/** The members whose name in the set, or the emote's own name, contains `needle` (any case). */
export function filterMembers<T extends Named>(items: readonly T[], needle: string): T[] {
  const n = needle.trim().toLowerCase();
  if (!n) return [...items];
  return items.filter((m) => m.name.toLowerCase().includes(n) || m.emote.name.toLowerCase().includes(n));
}

/** `12 / 1.000`. */
export function capacityText(used: number, capacity: number): string {
  return `${count(used)} / ${count(capacity)}`;
}

/** `12 of 1.000 emotes used`: the same figure for a screen reader. */
export function capacityLabel(used: number, capacity: number): string {
  return `${count(used)} of ${count(capacity)} emotes used`;
}

/** How full, 0 to 100 (whole numbers; 0 when there is no capacity). */
export function fillPercent(used: number, capacity: number): number {
  if (!(capacity > 0) || !(used > 0)) return 0;
  return Math.min(100, Math.round((used / capacity) * 100));
}

/** What a finished clone says: how many came along and why the rest didn't. */
export function cloneOutcome(copied: number, skipped: number): string {
  const c = `${count(copied)} ${copied === 1 ? 'emote' : 'emotes'} copied.`;
  if (skipped <= 0) return c;
  return `${c} ${count(skipped)} left out: over the size of a set, not approved yet, or someone else’s private emote.`;
}

/** The sentence for a failed clone, from the API's error code. */
export function cloneError(code: string | undefined, fallback: string): string {
  switch (code) {
    case 'signed_out':
      return 'Sign in to copy this set.';
    case 'not_found':
      return 'This set is gone, or it is private now.';
    case 'rate_limited':
      return 'That’s a lot of copied sets. Try again in a while.';
    case 'bad_name':
      return 'Set names are 1 to 100 characters.';
    case 'forbidden':
      return 'This account can’t make sets.';
    case 'offline':
      return fallback;
    default:
      return fallback || 'That didn’t work. Try again.';
  }
}
