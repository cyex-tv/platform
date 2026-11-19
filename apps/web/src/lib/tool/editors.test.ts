import { describe, expect, it } from 'vitest';
import type { EditorPermission } from '$lib/gql/graphql';
import {
  ALL_SETS,
  GROUPS,
  PERMISSIONS,
  allOn,
  canManage,
  covers,
  grantOn,
  grantable,
  locked,
  ordered,
  permissionLabels,
  scopeInput,
  scopeLabel,
  scopeOf,
  setAll,
  toggle,
  toggleSet,
  within,
} from './editors';

const perms = (...p: EditorPermission[]) => p;

describe('account editor permissions', () => {
  it('groups every permission once, each with a short line', () => {
    expect(GROUPS.map((g) => g.label)).toEqual(['Emote sets', 'Emotes', 'Profile', 'Channels', 'Billing and subscription', 'Editors']);
    const keys = PERMISSIONS.map((p) => p.key);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys).toHaveLength(10);
    for (const p of PERMISSIONS) expect(p.line.length).toBeLessThan(80);
    // The ones nothing checks yet say so in their group.
    for (const g of GROUPS) if (g.items.some((p) => p.reserved)) expect(g.note).toBeTruthy();
  });

  it('names permissions in order, folding channels into sets', () => {
    expect(ordered(['EDITORS', 'EMOTES', 'EMOTES'])).toEqual(['EMOTES', 'EDITORS']);
    expect(permissionLabels(['SETS', 'EMOTES', 'CHANNELS'])).toBe('Set emotes and Sets');
    expect(permissionLabels(['CHANNELS'])).toBe('Channels');
    expect(permissionLabels([])).toBe('Nothing');
    expect(permissionLabels(PERMISSIONS.map((p) => p.key))).toBe('Everything');
  });

  it('brings channel settings along with sets and keeps them while sets is on', () => {
    expect(toggle([], 'SETS', true)).toEqual(['SETS', 'CHANNELS']);
    expect(locked(['SETS', 'CHANNELS'], 'CHANNELS')).toBe(true);
    expect(toggle(['SETS', 'CHANNELS'], 'CHANNELS', false)).toEqual(['SETS', 'CHANNELS']);
    expect(toggle(['SETS', 'CHANNELS'], 'SETS', false)).toEqual(['CHANNELS']);
    expect(toggle(['CHANNELS'], 'CHANNELS', false)).toEqual([]);
    expect(locked(['CHANNELS'], 'CHANNELS')).toBe(false);
  });

  it('ticks everything you can grant, and nothing more', () => {
    const offer = grantable(new Set(perms('EMOTES', 'EDITORS')));
    expect(setAll(true, offer)).toEqual(['EMOTES', 'EDITORS']);
    expect(setAll(false, offer)).toEqual([]);
    expect(allOn(['EMOTES', 'EDITORS'], offer)).toBe(true);
    expect(allOn(['EMOTES'], offer)).toBe(false);
    expect(allOn([], [])).toBe(false);
    const owner = grantable(grantOn([], 'ME', 'ME'));
    expect(setAll(true, owner)).toHaveLength(10);
  });

  it('gives the owner everything and an editor their grant', () => {
    const grants = [{ account: { id: 'A' }, permissions: perms('EMOTES', 'SETS') }];
    expect(grantOn(grants, 'ME', 'ME').size).toBe(10);
    expect([...grantOn(grants, 'A', 'ME')]).toEqual(['EMOTES', 'SETS', 'CHANNELS']);
    expect(grantOn(grants, 'B', 'ME').size).toBe(0);
    expect(grantOn(grants, null, 'ME').size).toBe(0);
  });

  it('never offers more than you hold', () => {
    const held = new Set(perms('EMOTES', 'EDITORS'));
    expect(grantable(held).map((p) => p.key)).toEqual(['EMOTES', 'EDITORS']);
    expect(canManage(held, ['EMOTES'])).toBe(true);
    expect(canManage(held, ['EMOTES', 'SETS'])).toBe(false);
    expect(canManage(new Set(perms('EMOTES')), ['EMOTES'])).toBe(false);
  });
});

describe('sets an editor may edit', () => {
  const sets = [
    { id: 's1', name: 'Main' },
    { id: 's2', name: 'Subs' },
  ];

  it('defaults to every set, new ones included', () => {
    expect(scopeOf(null)).toEqual(ALL_SETS);
    expect(scopeOf({ allSets: true, setIds: [] })).toEqual(ALL_SETS);
    expect(scopeOf({ allSets: false, setIds: ['s1'] })).toEqual({ all: false, ids: ['s1'] });
    expect(scopeInput(ALL_SETS)).toEqual([]);
    expect(covers(ALL_SETS, 'anything')).toBe(true);
  });

  it('picks sets one by one and goes back to all when none is left', () => {
    let s = toggleSet(ALL_SETS, 's1', true);
    expect(s).toEqual({ all: false, ids: ['s1'] });
    s = toggleSet(s, 's2', true);
    expect(scopeInput(s)).toEqual(['s1', 's2']);
    expect(covers(s, 's2')).toBe(true);
    expect(covers(s, 's3')).toBe(false);
    s = toggleSet(toggleSet(s, 's1', false), 's2', false);
    expect(s).toEqual(ALL_SETS);
  });

  it('labels the choice', () => {
    expect(scopeLabel(ALL_SETS, sets)).toBe('All sets');
    expect(scopeLabel({ all: false, ids: ['s2'] }, sets)).toBe('Subs');
    expect(scopeLabel({ all: false, ids: ['s1', 's2'] }, sets)).toBe('2 sets');
  });

  it('never lets a limited manager hand out or change more sets than theirs', () => {
    const mine = { all: false as const, ids: ['s1'] };
    expect(within({ all: false, ids: ['s1'] }, mine)).toBe(true);
    expect(within(ALL_SETS, mine)).toBe(false);
    expect(within({ all: false, ids: ['s1', 's2'] }, mine)).toBe(false);
    const held = new Set(perms('EMOTES', 'EDITORS'));
    expect(canManage(held, ['EMOTES'], mine, { all: false, ids: ['s1'] })).toBe(true);
    expect(canManage(held, ['EMOTES'], mine, ALL_SETS)).toBe(false);
  });
});
