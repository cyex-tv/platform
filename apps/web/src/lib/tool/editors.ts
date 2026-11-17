// Account editors (ADR 0014): what each permission allows, in the words the Editors page uses,
// and what the signed-in account may do on someone else's account.
import type { EditorPermission } from '$lib/gql/graphql';

export type PermissionInfo = { key: EditorPermission; label: string; line: string };

/** Every permission, in the order the page lists them. */
export const PERMISSIONS: PermissionInfo[] = [
  { key: 'EMOTES', label: 'Emotes', line: 'Add, remove and rename emotes in every set, the ones chat shows too.' },
  { key: 'SETS', label: 'Sets', line: 'Make, rename and delete sets, and pick what each channel shows.' },
  { key: 'EDITORS', label: 'Editors', line: 'Add and remove editors, never with more than they hold themselves.' },
  { key: 'PROFILE', label: 'Profile', line: 'Badges and paints, once cosmetics arrive. Nothing to change yet.' },
];

const ORDER = PERMISSIONS.map((p) => p.key);

/** Permissions in the page's order, without repeats. */
export function ordered(perms: readonly EditorPermission[]): EditorPermission[] {
  return ORDER.filter((k) => perms.includes(k));
}

/** "Emotes and Sets" for a list of permissions. */
export function permissionLabels(perms: readonly EditorPermission[]): string {
  const names = ordered(perms).map((k) => PERMISSIONS.find((p) => p.key === k)!.label);
  if (names.length < 2) return names[0] ?? 'Nothing';
  return `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
}

type Grant = { account: { id: string }; permissions: readonly EditorPermission[] };

/** What you may do on `ownerId`'s account: every permission when it's yours. */
export function grantOn(grants: readonly Grant[] | null | undefined, ownerId: string | null | undefined, me: string | null | undefined) {
  const all = new Set<EditorPermission>(ORDER);
  if (!ownerId) return new Set<EditorPermission>();
  if (ownerId === me) return all;
  return new Set<EditorPermission>(grants?.find((g) => g.account.id === ownerId)?.permissions ?? []);
}

/** The flags you may hand out on an account: the ones you hold there. */
export function grantable(held: ReadonlySet<EditorPermission>): PermissionInfo[] {
  return PERMISSIONS.filter((p) => held.has(p.key));
}

/** Whether you may change or remove an editor holding `theirs`: only within your own flags. */
export function canManage(held: ReadonlySet<EditorPermission>, theirs: readonly EditorPermission[]): boolean {
  return held.has('EDITORS') && theirs.every((p) => held.has(p));
}
