// The profile's sections, each its own chunk. Uploaded emotes (the default tab) is part of the
// page; every other section loads when its tab is first asked for, and is fetched ahead when the
// tab is hovered or focused. The page's load awaits the section the address asks for, so the
// server renders it and a visit or navigation never shows a gap.
import type { Component } from 'svelte';
import type { SectionKey } from '../rows';

const loaders = {
  overview: () => import('./Overview.svelte'),
  active: () => import('./ActiveEmotes.svelte'),
  sets: () => import('./SetCards.svelte'),
  favourites: () => import('./Favourites.svelte'),
  picker: () => import('./CosmeticsPicker.svelte'),
  shown: () => import('./ShownCosmetics.svelte'),
  account: () => import('./AccountSection.svelte'),
  editors: () => import('./EditorsSection.svelte'),
  notifications: () => import('./NotificationsSection.svelte'),
  notificationsPanel: () => import('../NotificationsPanel.svelte'),
  activity: () => import('./ActivityFeed.svelte'),
  myUploads: () => import('./MyUploads.svelte'),
};
export type LazyKey = keyof typeof loaders;

let loaded = $state.raw<Partial<Record<LazyKey, Component<any>>>>({});
let failed = $state.raw<Partial<Record<LazyKey, true>>>({});
const pending = new Map<LazyKey, Promise<void>>();

/** The loaded section, or undefined while its chunk is on the way. */
export function lazySection(key: LazyKey): Component<any> | undefined {
  return loaded[key];
}
export function lazyFailed(key: LazyKey): boolean {
  return !!failed[key];
}

/** Starts (once) and awaits a section's chunk; never rejects, a failure shows as lazyFailed. */
export function loadSection(key: LazyKey): Promise<void> {
  if (loaded[key]) return Promise.resolve();
  let p = pending.get(key);
  if (!p) {
    p = loaders[key]().then(
      (m) => {
        loaded = { ...loaded, [key]: m.default };
        if (failed[key]) failed = { ...failed, [key]: undefined };
      },
      () => {
        failed = { ...failed, [key]: true };
        pending.delete(key);
      },
    );
    pending.set(key, p);
  }
  return p;
}

/** What shows for a section: the key of its chunk, or null for the page's own Uploaded emotes. */
export function lazyKeyFor(section: SectionKey, view: { mine: boolean; canProfile: boolean; manage: boolean; signedIn: boolean }): LazyKey | null {
  switch (section) {
    case 'overview': return 'overview';
    case 'active': return 'active';
    case 'sets': return 'sets';
    case 'favourites': return 'favourites';
    case 'cosmetics': return view.canProfile ? 'picker' : 'shown';
    case 'account': return view.mine && view.signedIn ? 'account' : fallback(view);
    case 'editors': return view.mine ? 'editors' : fallback(view);
    case 'notifications': return view.mine ? 'notifications' : fallback(view);
    case 'activity': return 'activity';
    default: return fallback(view);
  }
}
// Uploaded emotes: the owner's and a manager's own list, or the page's public grid.
function fallback(view: { mine: boolean; manage: boolean }): LazyKey | null {
  return view.mine || view.manage ? 'myUploads' : null;
}

const CANDIDATES: Record<string, LazyKey[]> = {
  overview: ['overview'], active: ['active'], sets: ['sets'], favourites: ['favourites'], cosmetics: ['shown', 'picker'],
  account: ['account'], editors: ['editors'], notifications: ['notifications', 'notificationsPanel'], activity: ['activity'],
};
const PUBLIC = new Set(['sets', 'cosmetics', 'activity', 'favourites']);

/** Fetches what the address's ?s= may show: on the server only the public sections (the owner's are the browser's). */
export function preloadFor(section: string | null, server: boolean): Promise<unknown> {
  if (!section || (server && !PUBLIC.has(section))) return Promise.resolve();
  return Promise.all((CANDIDATES[section] ?? []).filter((k) => !(server && (k === 'picker'))).map(loadSection));
}
