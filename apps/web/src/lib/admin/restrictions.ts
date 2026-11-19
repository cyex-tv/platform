// Restrictions on an account (uploads, reports and tickets, the whole account), as the staff
// account page shows and places them. The API ends a timed one on its own.
import type { RestrictionKind } from '$lib/gql/graphql';

export type HoldKind = `${RestrictionKind}`;

export const HOLDS: { kind: HoldKind; label: string; stopped: string; hint: string; permission: string }[] = [
  { kind: 'UPLOADS', label: 'Uploads', stopped: 'Uploads stopped', hint: 'They can still sign in and use sets.', permission: 'users.restrict_uploads' },
  { kind: 'TICKETS', label: 'Reports and tickets', stopped: 'Reports stopped', hint: 'They can’t report emotes or open tickets.', permission: 'users.restrict_tickets' },
  { kind: 'ACCOUNT', label: 'Whole account', stopped: 'Blocked', hint: 'Signs them out everywhere and refuses sign-in.', permission: 'users.block' },
];

export type Duration = '1h' | '1d' | '7d' | '30d' | 'custom' | 'permanent';

export const DURATIONS: { value: Duration; label: string }[] = [
  { value: '1h', label: '1 hour' },
  { value: '1d', label: '1 day' },
  { value: '7d', label: '7 days' },
  { value: '30d', label: '30 days' },
  { value: 'custom', label: 'Until…' },
  { value: 'permanent', label: 'For good' },
];

const HOUR = 3_600_000;
const SPANS: Partial<Record<Duration, number>> = { '1h': HOUR, '1d': 24 * HOUR, '7d': 7 * 24 * HOUR, '30d': 30 * 24 * HOUR };

/**
 * When a restriction picked as `d` ends, as an RFC 3339 time for the API; `null` for good.
 * `custom` is a `datetime-local` value (the viewer's time zone). Throws a sentence for the form
 * when a custom end is missing or not in the future.
 */
export function endFor(d: Duration, now: Date, custom = ''): string | null {
  if (d === 'permanent') return null;
  const span = SPANS[d];
  if (span) return new Date(now.getTime() + span).toISOString();
  const at = custom ? new Date(custom) : null;
  if (!at || Number.isNaN(at.getTime())) throw new Error('Pick the day and time it ends.');
  if (at.getTime() <= now.getTime()) throw new Error('The end has to be in the future.');
  if (at.getTime() > now.getTime() + 3650 * 24 * HOUR) throw new Error('At most ten years. Pick “For good” instead.');
  return at.toISOString();
}

/** A `datetime-local` value for `d` (min attribute, default custom end). */
export function localInput(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
