// The profile's Activity section (ADR 0016), behind a small adapter: the feed component only knows
// `ActivityItem`, and `loadActivity` maps the API's `accountActivity` into it. The feed is public.
import { gql } from '$lib/api';
import type { Picture } from '$lib/emote';
import type { AccountActivityQuery, ActivityGroup, ActivityKind as ApiKind } from '$lib/gql/graphql';
import { ACCOUNT_ACTIVITY, EMOTE_ACTIVITY } from '$lib/queries';

/** Whether the API has `accountActivity` (it does since PR 52). */
export const ACTIVITY_READY = true;

export type ActivityKind =
  | 'added'
  | 'removed'
  | 'renamed'
  | 'active'
  | 'cosmetics'
  | 'uploaded'
  | 'created'
  | 'setRenamed'
  | 'editorAdded'
  | 'editorRemoved';

type Change = { fromId?: string | null; fromName?: string | null; toId?: string | null; toName?: string | null };

export type ActivityItem = {
  id: string;
  kind: ActivityKind;
  /** RFC 3339. */
  at: string;
  /** Who did it (the owner or one of their editors). */
  actor: { id: string; displayName: string } | null;
  set: { id: string; name: string } | null;
  /** The emote, with the name it had in the set then, and its picture. */
  emote: { id: string; name: string; picture: Picture } | null;
  from: string | null;
  to: string | null;
  badge: Change | null;
  paint: Change | null;
  /** The editor added or removed (editor lines only); never their permissions. */
  editor: { id: string; displayName: string; avatar?: string | null } | null;
};

const KIND: Record<ApiKind, ActivityKind> = {
  SET_EMOTE_ADDED: 'added',
  SET_EMOTE_REMOVED: 'removed',
  SET_EMOTE_RENAMED: 'renamed',
  ACTIVE_SET_CHANGED: 'active',
  COSMETICS_SELECTED: 'cosmetics',
  EMOTE_UPLOADED: 'uploaded',
  SET_CREATED: 'created',
  SET_RENAMED: 'setRenamed',
  EDITOR_ADDED: 'editorAdded',
  EDITOR_REMOVED: 'editorRemoved',
};

type Entry = AccountActivityQuery['accountActivityPage']['entries'][number];

/** One API entry as a feed item; unknown kinds are dropped. */
export function toItem(e: Entry): ActivityItem | null {
  const kind = KIND[e.kind];
  if (!kind) return null;
  return {
    id: e.id,
    kind,
    at: e.createdAt,
    actor: e.actor ?? null,
    set: e.set ?? null,
    emote: e.emote ? { id: e.emote.id, name: e.emote.name, picture: { host: e.emote.host, animated: false, files: e.emote.files } } : null,
    from: e.fromName ?? null,
    to: e.toName ?? null,
    badge: e.badge ?? null,
    paint: e.paint ?? null,
    editor: e.editor ?? null,
  };
}

export const PAGE = 30;

/** A page of activity: its lines, and where to carry on from (`null`: that was the end). */
export type ActivityPage = { items: ActivityItem[]; next: string | null };

/** Rounds one "page" may take when the API answers empty-handed but says there is more. */
const SKIPS = 5;

type Answer = { entries: Entry[]; next?: string | null };

/**
 * Asks until a page has something to show. The API drops what isn't public, so a page can come
 * back short, even empty, while `next` says the feed goes on; follow it a few times, then hand the
 * cursor back so "Load more" carries on.
 */
async function fill(ask: (before: string | null) => Promise<Answer>, before: string | null): Promise<ActivityPage> {
  const items: ActivityItem[] = [];
  let next: string | null = before;
  for (let n = 0; n < SKIPS; n++) {
    const r = await ask(next);
    items.push(...r.entries.map(toItem).filter((i): i is ActivityItem => i !== null));
    next = r.next ?? null;
    if (items.length || next === null) break;
  }
  return { items, next };
}

/** What the profile's Activity tab narrows to (the feed's filter bar). */
export type ActivityQuery = {
  /** One group, or every group (editor lines included). */
  group?: Lowercase<ActivityGroup> | 'all';
  /** Only lines by this account (the owner, or one of its editors). */
  actor?: string | null;
  since?: string | null;
  until?: string | null;
};

const GROUPS: ActivityGroup[] = ['EMOTES', 'SETS', 'COSMETICS', 'EDITORS'];

/** The API's filter for a query; "all" asks for every group, so editor lines come too. */
export function filterOf(q: ActivityQuery) {
  const group = q.group ?? 'all';
  return {
    groups: group === 'all' ? GROUPS : [group.toUpperCase() as ActivityGroup],
    actorId: q.actor || null,
    since: q.since || null,
    until: q.until || null,
  };
}

/** A page of the account's activity, newest first; `before` is the previous page's `next`. */
export function loadActivity(
  accountId: string,
  before: string | null = null,
  fetcher: typeof fetch = fetch,
  query: ActivityQuery = {},
): Promise<ActivityPage> {
  const filter = filterOf(query);
  return fill((b) => gql(ACCOUNT_ACTIVITY, { accountId, before: b, limit: PAGE, filter }, fetcher).then((r) => r.accountActivityPage), before);
}

/** A page of the lines about one emote (the emote page's Activity tab); empty unless it is public. */
export function loadEmoteActivity(emoteId: string, before: string | null = null, fetcher: typeof fetch = fetch): Promise<ActivityPage> {
  return fill((b) => gql(EMOTE_ACTIVITY, { emoteId, before: b, limit: PAGE }, fetcher).then((r) => r.emoteActivityPage), before);
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
