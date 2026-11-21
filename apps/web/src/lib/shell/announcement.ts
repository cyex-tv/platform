// The site announcement under the header: which one shows, what closing it remembers, and where
// its link may go. Closing is per device and per announcement (a new one shows again). Pure, so
// it is tested.
import type { IconName } from '$lib/components/Icon.svelte';
import type { AnnouncementKind } from '$lib/gql/graphql';

export const DISMISS_KEY = 'cyex.announcement.closed';
/** The live announcement's ID, for app.html's first-paint hint on prerendered pages. */
export const SEEN_KEY = 'cyexAnnounce';
/** Closed IDs kept at most; the oldest go first. */
export const DISMISS_MAX = 20;

/** The closed IDs from storage: a JSON list of strings, anything else counts as none. */
export function readClosed(raw: string | null | undefined): string[] {
  if (!raw) return [];
  try {
    const v: unknown = JSON.parse(raw);
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string').slice(-DISMISS_MAX) : [];
  } catch {
    return [];
  }
}

/**
 * The closed IDs also go in a cookie, so the server renders the page with or without the bar and
 * nothing moves once it is in the browser. IDs joined by dots: only letters and digits are kept.
 */
export const CLOSED_COOKIE = 'cyexAnnounceClosed';
const ID = /^[0-9A-Za-z_-]{1,64}$/;

/** The closed IDs from the cookie's value; anything that isn't an ID is dropped. */
export function readClosedCookie(raw: string | null | undefined): string[] {
  if (!raw) return [];
  return raw
    .split('.')
    .filter((x) => ID.test(x))
    .slice(-DISMISS_MAX);
}

/** The cookie's value for a closed list (the IDs that can't go in a cookie are left out). */
export function closedCookie(list: readonly string[]): string {
  return list
    .filter((x) => ID.test(x))
    .slice(-DISMISS_MAX)
    .join('.');
}

/** Both lists in one, each ID once, oldest first, capped. */
export function mergeClosed(a: readonly string[], b: readonly string[]): string[] {
  return [...new Set([...a, ...b])].slice(-DISMISS_MAX);
}

/** The list after closing `id`: at the end, once, capped. */
export function withClosed(list: readonly string[], id: string): string[] {
  return [...list.filter((x) => x !== id), id].slice(-DISMISS_MAX);
}

/** Whether a link goes somewhere safe: a path on cyex.tv or an https:// address. Otherwise null. */
export function safeLink(url: string | null | undefined): string | null {
  if (!url) return null;
  const u = url.trim();
  if (u.startsWith('/') && !u.startsWith('//') && !u.startsWith('/\\')) return u;
  try {
    const parsed = new URL(u);
    return parsed.protocol === 'https:' ? parsed.href : null;
  } catch {
    return null;
  }
}

/** Whether the link leaves cyex.tv (it then opens in a new tab). */
export const external = (href: string) => !href.startsWith('/');

export const KIND: Record<AnnouncementKind, { icon: IconName; label: string }> = {
  INFO: { icon: 'message-square', label: 'News' },
  MAINTENANCE: { icon: 'wrench', label: 'Maintenance' },
  CAMPAIGN: { icon: 'sparkles', label: 'Event' },
};

/** Whether an announcement is still on at `now` (the API's answer may be a few seconds old). */
export function running(a: { endsAt?: string | null; endedAt?: string | null }, now: number): boolean {
  if (a.endedAt) return false;
  if (!a.endsAt) return true;
  const end = Date.parse(a.endsAt);
  return !Number.isFinite(end) || end > now;
}

/** An emote's ID from what staff paste: the ID itself, or a link to its page. Null when neither. */
export function emoteIdFrom(raw: string): string | null {
  const t = raw.trim();
  if (!t) return null;
  const m = /\/emotes\/([A-Za-z0-9_-]{1,64})(?:[/?#]|$)/.exec(t);
  if (m) return m[1];
  return /^[A-Za-z0-9_-]{1,64}$/.test(t) ? t : null;
}

/** Where an announcement stands for the admin list. */
export function phase(a: { live: boolean; startsAt?: string | null; endsAt?: string | null; endedAt?: string | null }, now: number): 'live' | 'scheduled' | 'ended' {
  if (a.endedAt) return 'ended';
  if (a.endsAt && Date.parse(a.endsAt) <= now) return 'ended';
  if (a.live) return 'live';
  return a.startsAt && Date.parse(a.startsAt) > now ? 'scheduled' : 'ended';
}
