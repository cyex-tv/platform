// The profile's sections and its rail. The rail is public: every viewer (signed out, an editor,
// the owner) gets the same rows, and each opens something anyone may see (owner rule, 2026/11/20):
//   Uploaded emotes, Emote sets (the listed ones, ADR 0017), Cosmetics (what the account shows in
//   chat) and Activity (public, ADR 0016, when the API has it).
// Sections that are not for everyone stay openable by address but live off the rail:
//   - Overview (?s=overview), the owner's dashboard: the account menu opens it, and the band has a
//     Dashboard button. It links on to Active emotes, Favourites and Channels.
//   - Active emotes (?s=active): the owner and editors of the account.
//   - Favourites (?s=favourites): the owner alone (private; nobody else, editors neither).
//   - Account (?s=account: display name, connections, sign out everywhere, your data, deleting the
//     account) and Editors (?s=editors: your editors and the accounts you edit), owner rule
//     2026/11/22: they moved here from Settings. Owner-only tabs after the public ones
//     (ownerTabs); nobody else ever sees them, so the public profile is unchanged.
//   - Notifications (?s=notifications: the filters, the list and the picked one in the panel),
//     owner rule 2026/11/22: it was /me/notifications (which now opens this tab). The first of
//     the owner-only tabs; private like the others.
// What a section shows can still depend on who looks (the owner's Uploaded emotes lists pending
// ones, the owner's Cosmetics tab is the picker), but the rows never do.
// Sections are query-param tabs on /users/<id> (?s=sets); Uploaded emotes is the default (no
// parameter), so the server-rendered public page and the owner's page share one address.
// The /me management pages keep their routes and mark the row they belong to. Pure, so it is tested.
import type { EditorPermission } from '$lib/gql/graphql';
import type { RailGroup, RailItem } from '$lib/shell/rail';

export const SECTIONS = ['overview', 'uploads', 'active', 'sets', 'favourites', 'cosmetics', 'activity', 'notifications', 'account', 'editors'] as const;
export type SectionKey = (typeof SECTIONS)[number];
/** The owner's own tabs, after the public ones (never shown to anyone else). */
export const OWNER_SECTIONS = ['notifications', 'account', 'editors'] as const satisfies readonly SectionKey[];
/** The sections in the rail, for everyone. */
export const RAIL_SECTIONS = ['uploads', 'sets', 'cosmetics', 'activity'] as const satisfies readonly SectionKey[];
/** A profile section, or a /me page that sits in the profile's rail. */
export type RailKey = SectionKey | 'channels' | null;

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

/** The sections this viewer may open on this account (the rail lists the public ones only). */
export function sectionsFor(r: Relation, activity: boolean): SectionKey[] {
  const out: SectionKey[] = [];
  if (r.mine) out.push('overview');
  if (manages(r)) out.push('active');
  out.push('uploads');
  out.push('sets');
  if (r.mine) out.push('favourites');
  out.push('cosmetics');
  if (activity) out.push('activity');
  if (r.mine) out.push(...OWNER_SECTIONS);
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
  if (path.startsWith('/me/account') || path.startsWith('/me/settings/account')) return 'account';
  if (path.startsWith('/me/editors')) return 'editors';
  if (path.startsWith('/me/notifications')) return 'notifications';
  return null;
}

const LABEL: Record<SectionKey, string> = {
  overview: 'Overview',
  active: 'Active emotes',
  uploads: 'Uploaded emotes',
  sets: 'Emote sets',
  favourites: 'Favourites',
  cosmetics: 'Cosmetics',
  activity: 'Activity',
  notifications: 'Notifications',
  account: 'Account',
  editors: 'Editors',
};
const ICON: Record<SectionKey, RailItem['icon']> = {
  overview: 'gauge',
  active: 'zap',
  uploads: 'upload',
  sets: 'folder',
  favourites: 'heart',
  cosmetics: 'brush',
  activity: 'activity',
  notifications: 'bell',
  account: 'user',
  editors: 'users',
};

/** The profile's rail: the same public rows for every viewer, emotes first, the current one marked. */
export function profileRail(accountId: string, current: RailKey, activity: boolean): RailGroup[] {
  const item = (k: SectionKey): RailItem => ({ href: sectionHref(accountId, k), label: LABEL[k], icon: ICON[k], current: current === k });
  const more: SectionKey[] = activity ? ['cosmetics', 'activity'] : ['cosmetics'];
  return [{ items: (['uploads', 'sets'] as const).map(item) }, { items: more.map(item) }];
}

/**
 * The pages that have no row in the profile rail (the /me pages Upload, Channels
 * and Favourites, and the off-rail sections Overview, Active emotes and
 * Favourites), as the extra tab the phone strip shows while you are on one, so the strip always
 * says where you are. Null on every other page.
 */
export function stripExtra(path: string, section?: SectionKey | null): RailItem | null {
  const at = (base: string) => path === base || path.startsWith(`${base}/`);
  if (at('/me/upload')) return { href: '/me/upload', label: 'Upload', icon: 'upload', current: true };
  if (at('/me/channels')) return { href: '/me/channels', label: 'Channels', icon: 'tv', current: true };
  if (at('/me/favourites')) return { href: '/me/favourites', label: 'Favourites', icon: 'heart', current: true };
  if (path.startsWith('/users/') && (section === 'overview' || section === 'active' || section === 'favourites'))
    return { href: `${path}?s=${section}`, label: LABEL[section], icon: ICON[section], current: true };
  return null;
}

/** The strip's tabs: the rail's rows, plus `extra` marked current (and then no other tab is). */
export function stripTabs(groups: RailGroup[], extra: RailItem | null): RailItem[] {
  const flat = groups.flatMap((g) => g.items);
  return extra ? [...flat.map((i) => ({ ...i, current: false })), extra] : flat;
}

/** The owner's tabs (Notifications, Account, Editors) on their own profile; none for anyone else. */
export function ownerTabs(accountId: string, r: Relation, current: SectionKey | null): RailItem[] {
  if (!r.mine) return [];
  return OWNER_SECTIONS.map((k) => ({ href: sectionHref(accountId, k), label: LABEL[k], icon: ICON[k], current: current === k }));
}

/**
 * The profile's tab row: the owner's Overview first (always), the public rows, then the off-row
 * section you are on (Active emotes, Favourites), then the owner's own tabs. Exactly one tab is current.
 */
export function profileTabs(accountId: string, r: Relation, section: SectionKey, path: string, activity: boolean): RailItem[] {
  // Overview is the owner's own dashboard: a tab of its own, first, wherever they came from.
  const overview: RailItem[] = r.mine ? [{ href: sectionHref(accountId, 'overview'), label: LABEL.overview, icon: ICON.overview, current: section === 'overview' }] : [];
  const extra = stripExtra(path, section === 'overview' ? null : section);
  const own = ownerTabs(accountId, r, section);
  const rest = stripTabs(profileRail(accountId, section, activity), extra);
  return [...overview, ...rest, ...own];
}

/** Your notifications: the owner-only tab on your profile (the old /me/notifications opens it). */
export const notificationsHref = (me: string | null | undefined) => (me ? sectionHref(me, 'notifications') : '/me/notifications');
