// The profile's Activity section, behind a small adapter: the feed component only knows
// `ActivityItem`, so wiring the API's `accountActivity` query (being built) means writing `load`
// here and turning `ACTIVITY_READY` on. Until then the section stays hidden.
import type { Picture } from '$lib/emote';

/** Whether the API has `accountActivity` yet. */
export const ACTIVITY_READY = false;

export type ActivityKind = 'added' | 'removed' | 'renamed' | 'paint';

export type ActivityItem = {
  id: string;
  kind: ActivityKind;
  /** RFC 3339. */
  at: string;
  /** Who did it (the owner or one of their editors). */
  actor: { id: string; displayName: string } | null;
  /** The set it happened in (added, removed, renamed). */
  set?: { id: string; name: string } | null;
  /** The emote (added, removed, renamed): its name in the set and its picture. */
  emote?: { id: string; name: string; picture?: Picture | null } | null;
  /** Renamed: the name before. */
  from?: string | null;
  /** Paint changed: the paint ids before and after (null for none). */
  paintFrom?: string | null;
  paintTo?: string | null;
};

/** The account's latest activity, newest first. */
export async function loadActivity(_accountId: string, _fetch: typeof fetch = fetch): Promise<ActivityItem[]> {
  // Note: waits for `accountActivity` in schema/schema.graphql (another branch builds it).
  return [];
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['week', 7 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
];

/** "3 minutes ago", "a month ago", "just now" (English, the site's language). */
export function relative(iso: string, now = Date.now()): string {
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return '';
  const secs = Math.max(0, Math.round((now - t) / 1000));
  if (secs < 45) return 'just now';
  for (const [unit, size] of UNITS) {
    const n = Math.round(secs / size);
    if (n >= 1 && secs >= size * 0.9) return n === 1 ? `${unit === 'hour' ? 'an' : 'a'} ${unit} ago` : `${n} ${unit}s ago`;
  }
  return 'a minute ago';
}
