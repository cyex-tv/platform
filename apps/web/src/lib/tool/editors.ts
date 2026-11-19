// Account editors (ADR 0014): what each permission allows, in the words the Editors page and its
// permissions dialog use, and what the signed-in account may do on someone else's account.
import type { EditorPermission } from '$lib/gql/graphql';

export type PermissionInfo = { key: EditorPermission; label: string; line: string; reserved?: boolean };
export type PermissionGroup = {
  label: string;
  note?: string;
  /** Its permissions count only on a grant covering every set (the API's rule for emotes). */
  needsAllSets?: boolean;
  items: PermissionInfo[];
};

/** The dialog's groups, in order. `reserved` flags are stored now and checked once their feature exists. */
export const GROUPS: PermissionGroup[] = [
  {
    label: 'Emote sets',
    items: [
      { key: 'EMOTES', label: 'Edit set emotes', line: 'Add, remove and rename emotes in sets.' },
      { key: 'SETS', label: 'Create and manage sets', line: 'Make, rename and delete sets.' },
    ],
  },
  {
    label: 'Emotes',
    needsAllSets: true,
    items: [
      { key: 'EMOTE_UPLOAD', label: 'Upload', line: 'Upload emotes the account owns.' },
      { key: 'EMOTE_MANAGE', label: 'Manage', line: 'Rename, retag, hide and delete its emotes.' },
      { key: 'EMOTE_TRANSFER', label: 'Transfer', line: 'Give its emotes to an account that trusts them.' },
    ],
  },
  {
    label: 'Profile',
    items: [{ key: 'PROFILE', label: 'Cosmetics and profile', line: 'Pick the badge and paint it shows.' }],
  },
  {
    label: 'Channels',
    items: [{ key: 'CHANNELS', label: 'Channel settings', line: 'The set each channel shows.' }],
  },
  {
    label: 'Billing and subscription',
    note: 'Saved now; used once subscriptions arrive.',
    items: [
      { key: 'BILLING_PAYMENT', label: 'Manage payment', line: 'Change payment methods.', reserved: true },
      { key: 'BILLING_SUBSCRIPTION', label: 'Manage subscription', line: 'Start, change or cancel it.', reserved: true },
    ],
  },
  {
    label: 'Editors',
    items: [{ key: 'EDITORS', label: 'Manage editors', line: 'Add and remove editors, up to their own.' }],
  },
];

/** Every permission, in the dialog's order. */
export const PERMISSIONS: PermissionInfo[] = GROUPS.flatMap((g) => g.items);

const ORDER = PERMISSIONS.map((p) => p.key);

/** Short names for lists ("Set emotes, Sets and Editors"). */
const SHORT: Record<EditorPermission, string> = {
  EMOTES: 'Set emotes',
  SETS: 'Sets',
  CHANNELS: 'Channels',
  EMOTE_UPLOAD: 'Uploads',
  EMOTE_MANAGE: 'Emotes',
  EMOTE_TRANSFER: 'Transfers',
  PROFILE: 'Profile',
  BILLING_PAYMENT: 'Payment',
  BILLING_SUBSCRIPTION: 'Subscription',
  EDITORS: 'Editors',
};

/** Permissions in the dialog's order, without repeats. */
export function ordered(perms: readonly EditorPermission[]): EditorPermission[] {
  return ORDER.filter((k) => perms.includes(k));
}

/** "Set emotes and Sets" for a list of permissions. Channels is left out when Sets brings it. */
export function permissionLabels(perms: readonly EditorPermission[]): string {
  const list = ordered(perms).filter((k) => !(k === 'CHANNELS' && perms.includes('SETS')));
  if (list.length === ORDER.length - 1 && perms.includes('SETS')) return 'Everything';
  const names = list.map((k) => SHORT[k]);
  if (names.length < 2) return names[0] ?? 'Nothing';
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}

/** `SETS` comes with `CHANNELS` (the API grants both). */
export function implied(perms: readonly EditorPermission[]): EditorPermission[] {
  return ordered(perms.includes('SETS') ? [...perms, 'CHANNELS'] : perms);
}

/** A box that can't be unticked on its own: Channel settings while Sets is on. */
export function locked(perms: readonly EditorPermission[], key: EditorPermission): boolean {
  return key === 'CHANNELS' && perms.includes('SETS');
}

/** Ticks or unticks one permission, keeping what it brings along. */
export function toggle(perms: readonly EditorPermission[], key: EditorPermission, on: boolean): EditorPermission[] {
  if (!on && locked(perms, key)) return ordered(perms);
  return implied(on ? [...perms, key] : perms.filter((k) => k !== key));
}

/** Whether every permission you may grant is ticked. */
export function allOn(perms: readonly EditorPermission[], offer: readonly PermissionInfo[]): boolean {
  return offer.length > 0 && offer.every((p) => perms.includes(p.key));
}

/** "Everything": every permission you may grant, or none. */
export function setAll(on: boolean, offer: readonly PermissionInfo[]): EditorPermission[] {
  return on ? implied(offer.map((p) => p.key)) : [];
}

type Grant = { account: { id: string }; permissions: readonly EditorPermission[]; allSets?: boolean; setIds?: readonly string[] };

/** What you may do on `ownerId`'s account: every permission when it's yours. */
export function grantOn(grants: readonly Grant[] | null | undefined, ownerId: string | null | undefined, me: string | null | undefined) {
  if (!ownerId) return new Set<EditorPermission>();
  if (ownerId === me) return new Set<EditorPermission>(ORDER);
  return new Set<EditorPermission>(implied(grants?.find((g) => g.account.id === ownerId)?.permissions ?? []));
}

/** The flags you may hand out on an account: the ones you hold there. */
export function grantable(held: ReadonlySet<EditorPermission>): PermissionInfo[] {
  return PERMISSIONS.filter((p) => held.has(p.key));
}

// Which sets an editor works on: every set (new ones too), or a list.

export type Scope = { all: true } | { all: false; ids: readonly string[] };

export const ALL_SETS: Scope = { all: true };

export function scopeOf(g: { allSets?: boolean; setIds?: readonly string[] } | null | undefined): Scope {
  if (!g || g.allSets !== false) return ALL_SETS;
  return { all: false, ids: [...(g.setIds ?? [])] };
}

/** Ticks or unticks one set. Unticking the last one goes back to every set. */
export function toggleSet(scope: Scope, id: string, on: boolean): Scope {
  const now = scope.all ? [] : scope.ids;
  const ids = on ? [...new Set([...now, id])] : now.filter((x) => x !== id);
  return ids.length ? { all: false, ids } : ALL_SETS;
}

/** What the scope button reads: "All sets", the set's name, or "3 sets". */
export function scopeLabel(scope: Scope, sets: readonly { id: string; name: string }[]): string {
  if (scope.all) return 'All sets';
  if (scope.ids.length === 1) return sets.find((s) => s.id === scope.ids[0])?.name ?? '1 set';
  return `${scope.ids.length} sets`;
}

/** What the API takes: no IDs for every set. */
export function scopeInput(scope: Scope): string[] {
  return scope.all ? [] : [...scope.ids];
}

/** Whether `inner` covers nothing `outer` doesn't. */
export function within(inner: Scope, outer: Scope): boolean {
  if (outer.all) return true;
  if (inner.all) return false;
  return inner.ids.every((id) => outer.ids.includes(id));
}

/** Whether a grant lets someone work on one set. */
export function covers(scope: Scope, setId: string): boolean {
  return scope.all || scope.ids.includes(setId);
}

/** Whether you may change or remove an editor holding `theirs` on `theirScope`: only within your own. */
export function canManage(
  held: ReadonlySet<EditorPermission>,
  theirs: readonly EditorPermission[],
  mine: Scope = ALL_SETS,
  theirScope: Scope = ALL_SETS,
): boolean {
  return held.has('EDITORS') && implied(theirs).every((p) => held.has(p)) && within(theirScope, mine);
}

// Emotes belong to the account, not to a set: the emote permissions count only on a grant that
// covers every set (the API refuses them otherwise).

export const EMOTE_PERMISSIONS: readonly EditorPermission[] = ['EMOTE_UPLOAD', 'EMOTE_MANAGE', 'EMOTE_TRANSFER'];

type EmoteGrant = { account: { id: string }; permissions: readonly EditorPermission[]; allSets?: boolean };

/** What a grant lets you do, as the API checks it: the emote permissions drop out on a set-limited grant. */
export function effective(g: { permissions: readonly EditorPermission[]; allSets?: boolean }): EditorPermission[] {
  const perms = implied(g.permissions);
  return g.allSets === false ? perms.filter((p) => !EMOTE_PERMISSIONS.includes(p)) : perms;
}

/** Whether a grant holds emote permissions that its set limit switches off. */
export function emotePermissionsIdle(g: { permissions: readonly EditorPermission[]; allSets?: boolean }): boolean {
  return g.allSets === false && g.permissions.some((p) => EMOTE_PERMISSIONS.includes(p));
}

/** Whether you may use one emote permission on `ownerId`'s emotes: always on your own. */
export function emoteCan(
  grants: readonly EmoteGrant[] | null | undefined,
  ownerId: string | null | undefined,
  me: string | null | undefined,
  key: EditorPermission,
): boolean {
  if (!ownerId || !me) return false;
  if (ownerId === me) return true;
  const g = grants?.find((x) => x.account.id === ownerId);
  return !!g && effective(g).includes(key);
}

export type UploadTarget = { id: string; displayName: string; self: boolean };

/** Who you can upload for: yourself first, then the accounts where you hold Upload, by name. */
export function uploadTargets(
  me: { id: string; displayName: string } | null | undefined,
  grants: readonly (EmoteGrant & { account: { id: string; displayName: string } })[] | null | undefined,
): UploadTarget[] {
  if (!me) return [];
  const others = (grants ?? [])
    .filter((g) => g.account.id !== me.id && effective(g).includes('EMOTE_UPLOAD'))
    .map((g) => ({ id: g.account.id, displayName: g.account.displayName, self: false }))
    .sort((a, b) => a.displayName.localeCompare(b.displayName));
  return [{ id: me.id, displayName: me.displayName, self: true }, ...others];
}

/**
 * Where an emote can go: yourself (unless it's yours already), and the accounts where you hold
 * Upload, so nobody gets an emote from someone they never trusted.
 */
export function transferTargets(
  me: { id: string; displayName: string } | null | undefined,
  grants: readonly (EmoteGrant & { account: { id: string; displayName: string } })[] | null | undefined,
  ownerId: string | null | undefined,
): UploadTarget[] {
  return uploadTargets(me, grants).filter((t) => t.id !== ownerId);
}

/** The most editors an account can have (the API's `too_many_editors`). */
export const MAX_EDITORS = 50;
