// The profile's rail (your profile is your dashboard, the 7TV model): which rows a viewer gets on
// an account's page, where each goes, and which one is current. Pure, so it is tested.
//   - The owner: Overview first (?s=overview, the dashboard the account menu opens).
//   - Everyone: Uploaded emotes and Emote sets (the account's listed sets, ADR 0017).
//   - The owner, and editors of the account: Active emotes, and all their sets; the owner also Channels.
//   - The owner alone: Favourites (private; nobody else, editors neither, ever sees it).
//   - The owner, and editors with PROFILE: Cosmetics.
//   - Everyone: Collection (owned badges and paints), when the account shows it; the owner always.
//   - Everyone: Activity (public, ADR 0016), when the API has it.
// Sections are query-param tabs on /users/<id> (?s=sets); Uploaded emotes is the default (no
// parameter), so the server-rendered public page and the owner's page share one address.
// The /me management pages keep their routes and mark the row they belong to.
import type { EditorPermission } from '$lib/gql/graphql';
import type { RailGroup, RailItem } from '$lib/shell/rail';

export const SECTIONS = ['overview', 'uploads', 'active', 'sets', 'favourites', 'cosmetics', 'collection', 'activity'] as const;
export type SectionKey = (typeof SECTIONS)[number];
/** A profile section, or a /me page that sits in the profile's rail. */
export type RailKey = SectionKey | 'channels' | 'connections' | 'editors' | null;

/** What the viewer may do on the account: everything when it is theirs. */
export type Relation = { mine: boolean; perms: ReadonlySet<EditorPermission> };

export const NOBODY: Relation = { mine: false, perms: new Set() };

/** Whether the viewer manages the account at all (owner or any editor permission). */
export const manages = (r: Relation) => r.mine || r.perms.size > 0;
export const canProfile = (r: Relation) => r.mine || r.perms.has('PROFILE');
export const canEditors = (r: Relation) => r.mine || r.perms.has('EDITORS');
export const canSets = (r: Relation) => r.mine || r.perms.has('SETS');

/** `/users/<id>` for the default section, `/users/<id>?s=<key>` for the rest. */
export function sectionHref(accountId: string, key: SectionKey): string {
  const base = `/users/${encodeURIComponent(accountId)}`;
  return key === 'uploads' ? base : `${base}?s=${key}`;
}

/** The sections this viewer has on this account, in rail order. */
export function sectionsFor(r: Relation, activity: boolean, collection = false): SectionKey[] {
  const out: SectionKey[] = [];
  if (r.mine) out.push('overview');
  if (manages(r)) out.push('active');
  out.push('uploads');
  out.push('sets');
  if (r.mine) out.push('favourites');
  if (canProfile(r)) out.push('cosmetics');
  if (r.mine || collection) out.push('collection');
  if (activity) out.push('activity');
  return out;
}

/** The section the address asks for, if the viewer has it; otherwise Uploaded emotes. */
export function sectionFrom(search: URLSearchParams, allowed: readonly SectionKey[]): SectionKey {
  const want = search.get('s') as SectionKey | null;
  return want && allowed.includes(want) ? want : 'uploads';
}

/** Which rail row a /me page belongs to. */
export function railKeyForPath(path: string): RailKey {
  if (path.startsWith('/me/sets')) return 'sets';
  if (path.startsWith('/me/favourites')) return 'favourites';
  if (path === '/me' || path.startsWith('/me/emotes') || path.startsWith('/me/upload')) return 'uploads';
  if (path.startsWith('/me/channels')) return 'channels';
  if (path.startsWith('/me/cosmetics')) return 'cosmetics';
  if (path.startsWith('/me/account')) return 'connections';
  if (path.startsWith('/me/editors')) return 'editors';
  return null;
}

const LABEL: Record<SectionKey, string> = {
  overview: 'Overview',
  active: 'Active emotes',
  uploads: 'Uploaded emotes',
  sets: 'Emote sets',
  favourites: 'Favourites',
  cosmetics: 'Cosmetics',
  collection: 'Collection',
  activity: 'Activity',
};
const ICON: Record<SectionKey, RailItem['icon']> = {
  overview: 'gauge',
  active: 'zap',
  uploads: 'upload',
  sets: 'folder',
  favourites: 'heart',
  cosmetics: 'brush',
  collection: 'badge',
  activity: 'activity',
};

/** The profile's rail rows: emotes first, then cosmetics and activity, the current one marked. */
export function profileRail(accountId: string, r: Relation, current: RailKey, activity: boolean, collection = false): RailGroup[] {
  const keys = sectionsFor(r, activity, collection);
  const item = (k: SectionKey): RailItem => ({ href: sectionHref(accountId, k), label: LABEL[k], icon: ICON[k], current: current === k });
  const emotes: RailItem[] = keys.filter((k) => k === 'active' || k === 'uploads' || k === 'sets' || k === 'favourites').map(item);
  if (r.mine) emotes.push({ href: '/me/channels', label: 'Channels', icon: 'tv', current: current === 'channels' });
  const more = keys.filter((k) => k === 'cosmetics' || k === 'collection' || k === 'activity').map(item);
  const head = keys.includes('overview') ? [{ items: [item('overview')] }] : [];
  return [...head, { items: emotes }, ...(more.length ? [{ items: more }] : [])];
}

/**
 * The /me pages that have no row of their own in the profile rail (Upload, Notifications,
 * Account, Editors), as the extra tab the phone strip shows while you are on one, so the strip
 * always says where you are. Null on every other page.
 */
export function stripExtra(path: string): RailItem | null {
  const at = (base: string) => path === base || path.startsWith(`${base}/`);
  if (at('/me/upload')) return { href: '/me/upload', label: 'Upload', icon: 'upload', current: true };
  if (at('/me/notifications')) return { href: '/me/notifications', label: 'Notifications', icon: 'bell', current: true };
  if (at('/me/account')) return { href: '/me/account', label: 'Account', icon: 'user', current: true };
  if (at('/me/editors')) return { href: '/me/editors', label: 'Editors', icon: 'circle-user', current: true };
  return null;
}

/** The strip's tabs: the rail's rows, plus `extra` marked current (and then no other tab is). */
export function stripTabs(groups: RailGroup[], extra: RailItem | null): RailItem[] {
  const flat = groups.flatMap((g) => g.items);
  return extra ? [...flat.map((i) => ({ ...i, current: false })), extra] : flat;
}
