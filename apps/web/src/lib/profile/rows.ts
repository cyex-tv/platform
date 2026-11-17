// The profile's rail (your profile is your dashboard, the 7TV model): which rows a viewer gets on
// an account's page, where each goes, and which one is current. Pure, so it is tested.
//   - Everyone: Uploaded emotes.
//   - The owner, and editors of the account: Active emotes and Emote sets; the owner also Channels.
//   - The owner, and editors with PROFILE: Cosmetics.
//   - Activity: when the API has it, for the owner and the account's editors.
// Sections are query-param tabs on /users/<id> (?s=sets); Uploaded emotes is the default (no
// parameter), so the server-rendered public page and the owner's page share one address.
// The /me management pages keep their routes and mark the row they belong to.
import type { EditorPermission } from '$lib/gql/graphql';
import type { RailGroup, RailItem } from '$lib/shell/rail';

export const SECTIONS = ['uploads', 'active', 'sets', 'cosmetics', 'activity'] as const;
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
export function sectionsFor(r: Relation, activity: boolean): SectionKey[] {
  const out: SectionKey[] = [];
  if (manages(r)) out.push('active');
  out.push('uploads');
  if (manages(r)) out.push('sets');
  if (canProfile(r)) out.push('cosmetics');
  if (activity && manages(r)) out.push('activity');
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
  if (path === '/me' || path.startsWith('/me/emotes') || path.startsWith('/me/upload')) return 'uploads';
  if (path.startsWith('/me/channels')) return 'channels';
  if (path.startsWith('/me/cosmetics')) return 'cosmetics';
  if (path.startsWith('/me/account')) return 'connections';
  if (path.startsWith('/me/editors')) return 'editors';
  return null;
}

const LABEL: Record<SectionKey, string> = {
  active: 'Active emotes',
  uploads: 'Uploaded emotes',
  sets: 'Emote sets',
  cosmetics: 'Cosmetics',
  activity: 'Activity',
};
const ICON: Record<SectionKey, RailItem['icon']> = {
  active: 'zap',
  uploads: 'upload',
  sets: 'folder',
  cosmetics: 'brush',
  activity: 'activity',
};

/** The profile's rail rows: emotes first, then cosmetics and activity, the current one marked. */
export function profileRail(accountId: string, r: Relation, current: RailKey, activity: boolean): RailGroup[] {
  const keys = sectionsFor(r, activity);
  const item = (k: SectionKey): RailItem => ({ href: sectionHref(accountId, k), label: LABEL[k], icon: ICON[k], current: current === k });
  const emotes: RailItem[] = keys.filter((k) => k === 'active' || k === 'uploads' || k === 'sets').map(item);
  if (r.mine) emotes.push({ href: '/me/channels', label: 'Channels', icon: 'tv', current: current === 'channels' });
  const more = keys.filter((k) => k === 'cosmetics' || k === 'activity').map(item);
  return [{ items: emotes }, ...(more.length ? [{ items: more }] : [])];
}
