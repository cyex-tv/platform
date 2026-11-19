// What a profile needs beyond the public page, loaded in the browser (only it has the session) and
// shared between the profile and the /me pages in its rail, so moving between them asks nothing
// twice. Every request is made once per account and kept until `forgetProfile()` (after a change
// the page already knows about, or signing out). A failed one isn't kept.
import { gql } from '$lib/api';
import type {
  AccountEditorsQuery,
  AccountProfileQuery,
  AccountRolesQuery,
  EditorOfQuery,
  EditorPermission,
  ProfileEditedQuery,
  ProfileMineQuery,
} from '$lib/gql/graphql';
import { ACCOUNT_EDITORS, ACCOUNT_PROFILE, ACCOUNT_ROLES, ACCOUNT_SETS, EDITOR_OF, MY_ROLES, PROFILE_EDITED, PROFILE_MINE } from '$lib/queries';
import { session, type Me } from '$lib/tool/session.svelte';
import { effective } from '$lib/tool/editors';
import { NOBODY, type Relation } from './rows';

const asks = new Map<string, Promise<unknown>>();

function once<T>(key: string, fn: () => Promise<T>): Promise<T> {
  const known = asks.get(key) as Promise<T> | undefined;
  if (known) return known;
  const p = fn();
  asks.set(key, p);
  p.catch(() => {
    if (asks.get(key) === p) asks.delete(key);
  });
  return p;
}

/** Forget cached answers: all, or those whose key starts with one of `prefixes`. */
export function forgetProfile(...prefixes: string[]) {
  if (!prefixes.length) return asks.clear();
  for (const k of [...asks.keys()]) if (prefixes.some((p) => k.startsWith(p))) asks.delete(k);
}

export type Grant = EditorOfQuery['editorOf'][number];
export type Role = AccountRolesQuery['accountRoles'][number];
export type Editor = AccountEditorsQuery['accountEditors'][number];
export type Channel = ProfileMineQuery['myChannels'][number];
export type ProfileSet = ProfileMineQuery['mySets'][number];

/** The accounts you edit, with what you may do on each. */
export const editorOf = () => once('grants', () => gql(EDITOR_OF, {}).then((r) => r.editorOf));

/** What `me` may do on `accountId`: the grant as the API checks it (emote permissions need every set). */
export function relationTo(accountId: string, me: Me | null, grants: readonly Grant[] | null): Relation {
  if (!me) return NOBODY;
  if (me.id === accountId) return { mine: true, perms: new Set<EditorPermission>(['EMOTES', 'SETS', 'EDITORS', 'PROFILE']) };
  const g = grants?.find((x) => x.account.id === accountId);
  return { mine: false, perms: new Set(g ? effective(g) : []) };
}

/** An account's roles: yours, or anyone's for staff with users.view or roles.manage. Highest first; [] when not allowed. */
export const rolesOf = (accountId: string) =>
  once(`roles:${accountId}`, () =>
    gql(ACCOUNT_ROLES, { accountId })
      .then((r) => [...r.accountRoles].sort((a, b) => b.position - a.position))
      .catch(() => [] as Role[]),
  );

/** Your own roles, hidden ones too (`me.roles`): only your own profile shows the hidden ones, marked. */
export const myRoles = () =>
  once('roles:mine', () => gql(MY_ROLES, {}).then((r) => r.me.roles));

export type PublicProfile = NonNullable<AccountProfileQuery['accountProfile']>;

/** An account's public profile (logins, public roles, active sets, member since); null when there is none. Anyone may ask. */
export const profileOf = (accountId: string) =>
  once(`profile:${accountId}`, () => gql(ACCOUNT_PROFILE, { accountId }).then((r) => r.accountProfile ?? null));

/** An account's editors: yours, or those of an account you edit with EDITORS. */
export const editorsOf = (accountId: string, mine: boolean) =>
  once(`editors:${accountId}`, () => gql(ACCOUNT_EDITORS, { accountId: mine ? null : accountId }).then((r) => r.accountEditors));

/** An account's listed sets as cards: what anyone sees on its profile. */
export const publicSetsOf = (accountId: string) =>
  once(`library:public:${accountId}`, () => gql(ACCOUNT_SETS, { accountId }).then((r) => r.accountSets));

/** The account's channels and sets: your own, or (from what you edit) the account's. */
export function libraryOf(accountId: string, mine: boolean): Promise<{ channels: Channel[]; sets: ProfileSet[] }> {
  if (mine) return once('library:mine', () => gql(PROFILE_MINE, {}).then((r) => ({ channels: r.myChannels, sets: r.mySets })));
  return once('library:edited', () => gql(PROFILE_EDITED, {})).then((r: ProfileEditedQuery) => ({
    channels: r.editorChannels.filter((c) => c.account.id === accountId),
    sets: r.editorSets.filter((s) => s.owner?.id === accountId),
  }));
}

/**
 * What the signed-in viewer may do on an account, kept current: everything on their own, their
 * editor permissions on one they edit, nothing otherwise (or while signed out).
 */
export function relationState(accountId: () => string) {
  let grants = $state<readonly Grant[] | null>(null);
  $effect(() => {
    const me = session.me;
    if (!me || me.id === accountId()) return;
    editorOf()
      .then((g) => (grants = g))
      .catch(() => {});
  });
  return {
    get value(): Relation {
      return relationTo(accountId(), session.me, grants);
    },
  };
}
