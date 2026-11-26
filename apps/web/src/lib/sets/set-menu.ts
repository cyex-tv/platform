// What a set's right-click menu offers you (ADR 0014 and 0017, amended 2026/11/22), as the API
// checks it: renaming and the capacity for whoever setCapacityRange answers (the owner, the set's
// editors, account editors with Sets covering it, staff); deleting for the owner, account editors
// with Sets covering it and staff (not a per-set editor); "Set active" on each of the owner's
// channels you change, as the owner or an account editor with Sets or Channel settings.
import type { EditorPermission, SetMenuQuery } from '$lib/gql/graphql';
import { covers, grantOn, scopeOf } from '$lib/tool/editors';

export type MenuSet = { id: string; name: string; kind: string; ownerId: string | null; capacity?: number };

export type ChannelChoice = { platform: string; platformChannelId: string; login: string; active: boolean };

export type MenuRights = {
  rename: boolean;
  capacity: boolean;
  remove: boolean;
  /** Open the set's editor (you may change something in it). */
  edit: boolean;
  channels: ChannelChoice[];
};

export type SetAction =
  | { kind: 'rename' }
  | { kind: 'copy' }
  | { kind: 'active'; platform: string; platformChannelId: string; login: string }
  | { kind: 'capacity' }
  | { kind: 'page' }
  | { kind: 'editor' }
  | { kind: 'delete' };

export const NO_RIGHTS: MenuRights = { rename: false, capacity: false, remove: false, edit: false, channels: [] };

export function menuRights(set: MenuSet, data: SetMenuQuery | null | undefined, me: string | null | undefined, staff: boolean): MenuRights {
  if (!data || !me) return NO_RIGHTS;
  const owner = !!set.ownerId && set.ownerId === me;
  const grant = data.editorOf.find((g) => g.account.id === set.ownerId);
  const covered = !!grant && covers(scopeOf(grant), set.id);
  const held: ReadonlySet<EditorPermission> = owner ? grantOn([], me, me) : covered ? grantOn(data.editorOf, set.ownerId, me) : new Set();
  const range = !!data.setCapacityRange;
  const remove = set.kind !== 'GLOBAL' && (owner || staff || held.has('SETS'));
  const canPick = set.kind === 'CHANNEL' && (owner || held.has('SETS') || held.has('CHANNELS'));
  const seen = new Set<string>();
  const channels: ChannelChoice[] = [];
  if (canPick) {
    for (const c of [...data.myChannels, ...data.editorChannels]) {
      const key = `${c.platform}:${c.platformChannelId}`;
      if (seen.has(key) || c.account.id !== set.ownerId) continue;
      seen.add(key);
      channels.push({ platform: c.platform, platformChannelId: c.platformChannelId, login: c.login, active: c.activeSetId === set.id });
    }
  }
  return { rename: range, capacity: range, remove, edit: range || remove, channels };
}

export const platformName = (p: string) => (p === 'KICK' ? 'Kick' : 'Twitch');

/** The set's public address. */
export const setLink = (origin: string, id: string) => `${origin}/sets/${encodeURIComponent(id)}`;

/** Shift+F10 or the context-menu key: open the menu from the keyboard (a browser may not turn them into a contextmenu event). */
export function isMenuKey(e: KeyboardEvent): boolean {
  return e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey);
}