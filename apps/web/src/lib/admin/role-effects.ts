// What a role does with each permission: Allow, Deny or Inherit (not set, the roles below decide).
// A role's explicit allows and denies are two lists that never share a name. These helpers are the
// editor's maths, kept apart from the screen so they can be tested; the API decides for real.
import { PERMISSION_GROUPS, permissionLabel, type PermissionGroup } from './permissions';

export type Effect = 'allow' | 'inherit' | 'deny';
export type Sets = { allows: string[]; denies: string[] };

export const EFFECTS: { id: Effect; label: string }[] = [
  { id: 'allow', label: 'Allow' },
  { id: 'inherit', label: 'Inherit' },
  { id: 'deny', label: 'Deny' },
];

const ORDER = PERMISSION_GROUPS.flatMap((g) => g.items.map((i) => i.key));
const sortKeys = (keys: Iterable<string>) =>
  [...new Set(keys)].sort((a, b) => (ORDER.indexOf(a) + 1 || 1e9) - (ORDER.indexOf(b) + 1 || 1e9) || a.localeCompare(b));

/** What this role does with a permission. */
export function effectOf(sets: Sets, key: string): Effect {
  if (sets.denies.includes(key)) return 'deny';
  return sets.allows.includes(key) ? 'allow' : 'inherit';
}

/** The sets with one permission moved to an effect (a name is never in both lists). */
export function setEffect(sets: Sets, key: string, effect: Effect): Sets {
  const allows = new Set(sets.allows);
  const denies = new Set(sets.denies);
  allows.delete(key);
  denies.delete(key);
  if (effect === 'allow') allows.add(key);
  if (effect === 'deny') denies.add(key);
  return { allows: sortKeys(allows), denies: sortKeys(denies) };
}

/** A whole group moved to an effect. Permissions you may not change stay as they are. */
export function setGroup(sets: Sets, group: PermissionGroup, effect: Effect, canChange: (key: string) => boolean = () => true): Sets {
  let out = sets;
  for (const i of group.items) if (canChange(i.key)) out = setEffect(out, i.key, effect);
  return out;
}

/** How many of a group are allowed and denied. */
export function groupCounts(sets: Sets, group: PermissionGroup): { allow: number; deny: number; of: number } {
  const a = new Set(sets.allows);
  const d = new Set(sets.denies);
  return { allow: group.items.filter((i) => a.has(i.key)).length, deny: group.items.filter((i) => d.has(i.key)).length, of: group.items.length };
}

/** Allows or denies of a role as groups with the labels of the permissions in them (the inspector). */
export function summarize(keys: string[], groups: PermissionGroup[]): { label: string; n: number; of: number; names: string }[] {
  const have = new Set(keys);
  return groups
    .map((g) => {
      const items = g.items.filter((i) => have.has(i.key));
      return { label: g.label, n: items.length, of: g.items.length, names: items.map((i) => i.label).join(', ') };
    })
    .filter((g) => g.n > 0);
}

export type Decision = { permission: string; granted: boolean; effect?: 'ALLOW' | 'DENY' | null; roleId?: string | null; roleName?: string | null; owner: boolean };

/** One line for an account's permission: "Allowed by Moderator", "Denied by Muted", "Owner", "Not granted". */
export function decisionText(d: Decision): string {
  if (d.owner) return 'Owner';
  if (d.effect === 'ALLOW') return d.roleName ? `Allowed by ${d.roleName}` : 'Allowed';
  if (d.effect === 'DENY') return d.roleName ? `Denied by ${d.roleName}` : 'Denied';
  return 'Not granted';
}

export type DecisionState = 'allowed' | 'denied' | 'none';
export const decisionState = (d: Decision): DecisionState => (d.granted ? 'allowed' : d.effect === 'DENY' ? 'denied' : 'none');

/** The decisions grouped like the editor, each group's items in the editor's order. */
export function decisionGroups(decisions: Decision[], groups: PermissionGroup[]): { id: string; label: string; items: (Decision & { label: string })[] }[] {
  const by = new Map(decisions.map((d) => [d.permission, d]));
  return groups
    .map((g) => ({ id: g.id, label: g.label, items: g.items.filter((i) => by.has(i.key)).map((i) => ({ ...by.get(i.key)!, label: permissionLabel(i.key) })) }))
    .filter((g) => g.items.length);
}
