import { describe, expect, it } from 'vitest';
import type { EditorPermission } from '$lib/gql/graphql';
import {
  ALL_SETS,
  EMOTE_PERMISSIONS,
  GROUPS,
  PERMISSIONS,
  allOn,
  canManage,
  covers,
  effective,
  emoteCan,
  emotePermissionsIdle,
  transferTargets,
  uploadTargets,
  grantOn,
  grantable,
  ordered,
  ownerSetsHref,
  permissionLabels,
  permissionPhrase,
  setAccess,
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
    expect(keys).toHaveLength(9); // EMOTE_TRANSFER is retired
    for (const p of PERMISSIONS) expect(p.line.length).toBeLessThan(80);
    // The ones nothing checks yet say so in their group.
    for (const g of GROUPS) if (g.items.some((p) => p.reserved)) expect(g.note).toBeTruthy();
  });

  it('checks the emote permissions now: no "saved now" note, and they need every set', () => {
    const emotes = GROUPS.find((g) => g.label === 'Emotes')!;
    expect(emotes.note).toBeUndefined();
    expect(emotes.needsAllSets).toBe(true);
    expect(emotes.items.map((p) => p.key)).toEqual(['EMOTE_UPLOAD', 'EMOTE_MANAGE']);
    expect(emotes.items.some((p) => p.reserved)).toBe(false);
    // Billing still waits for its feature.
    const billing = GROUPS.find((g) => g.label === 'Billing and subscription')!;
    expect(billing.note).toBeTruthy();
    expect(billing.items.every((p) => p.reserved)).toBe(true);
  });

  it('names permissions in order, channel settings on their own', () => {
    expect(ordered(['EDITORS', 'EMOTES', 'EMOTES'])).toEqual(['EMOTES', 'EDITORS']);
    expect(permissionLabels(['SETS', 'EMOTES'])).toBe('Edit set emotes and manage sets');
    expect(permissionLabels(['SETS', 'EMOTES', 'CHANNELS'])).toBe('Edit set emotes, manage sets and change channel settings');
    expect(permissionLabels(['CHANNELS'])).toBe('Change channel settings');
    expect(permissionLabels([])).toBe('Nothing');
    expect(permissionLabels(PERMISSIONS.map((p) => p.key))).toBe('Everything');
    // "Everything" means every listed permission: Sets doesn't stand in for Channel settings.
    expect(permissionLabels(PERMISSIONS.map((p) => p.key).filter((k) => k !== 'CHANNELS'))).not.toBe('Everything');
  });

  it('ticks sets without channel settings, and each comes off on its own', () => {
    expect(toggle([], 'SETS', true)).toEqual(['SETS']);
    expect(toggle(['SETS', 'CHANNELS'], 'CHANNELS', false)).toEqual(['SETS']);
    expect(toggle(['SETS', 'CHANNELS'], 'SETS', false)).toEqual(['CHANNELS']);
    expect(toggle(['CHANNELS'], 'CHANNELS', false)).toEqual([]);
    expect(toggle(['SETS'], 'CHANNELS', true)).toEqual(['SETS', 'CHANNELS']);
  });

  it('ticks everything you can grant, and nothing more', () => {
    const offer = grantable(new Set(perms('EMOTES', 'EDITORS')));
    expect(setAll(true, offer)).toEqual(['EMOTES', 'EDITORS']);
    expect(setAll(false, offer)).toEqual([]);
    expect(allOn(['EMOTES', 'EDITORS'], offer)).toBe(true);
    expect(allOn(['EMOTES'], offer)).toBe(false);
    expect(allOn([], [])).toBe(false);
    const owner = grantable(grantOn([], 'ME', 'ME'));
    expect(setAll(true, owner)).toHaveLength(9);
  });

  it('gives the owner everything and an editor their grant', () => {
    const grants = [{ account: { id: 'A' }, permissions: perms('EMOTES', 'SETS') }];
    expect(grantOn(grants, 'ME', 'ME').size).toBe(9);
    expect([...grantOn(grants, 'A', 'ME')]).toEqual(['EMOTES', 'SETS']);
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

  it('lets a manager with Sets but not Channel settings manage an editor holding only Sets', () => {
    const held = grantOn([{ account: { id: 'A' }, permissions: perms('SETS', 'EDITORS') }], 'A', 'ME');
    expect(held.has('CHANNELS')).toBe(false);
    expect(canManage(held, ['SETS'])).toBe(true);
    expect(canManage(held, ['SETS', 'CHANNELS'])).toBe(false);
  });
});

describe('the retired transfer permission (owner decision 2026/11/22)', () => {
  it('is not in the picker, nor phrased, nor grantable, nor counted as an emote permission', () => {
    expect(PERMISSIONS.map((p) => p.key)).not.toContain('EMOTE_TRANSFER');
    expect(PERMISSIONS.some((p) => /transfer/i.test(`${p.label} ${p.line}`))).toBe(false);
    expect(grantable(new Set(['EMOTE_TRANSFER', 'EMOTE_MANAGE'] as EditorPermission[])).map((p) => p.key)).toEqual(['EMOTE_MANAGE']);
    expect(permissionPhrase(['EMOTE_TRANSFER'])).toBe('do nothing');
    expect(permissionPhrase(['EMOTE_MANAGE', 'EMOTE_TRANSFER'])).toBe('manage emotes');
    expect(EMOTE_PERMISSIONS).not.toContain('EMOTE_TRANSFER');
  });
});

describe('emote permissions on another account', () => {
  const me = { id: 'ME', displayName: 'Me' };
  const grants = [
    { account: { id: 'A', displayName: 'Zed' }, permissions: perms('EMOTE_UPLOAD', 'EMOTE_MANAGE'), allSets: true },
    { account: { id: 'B', displayName: 'Ann' }, permissions: perms('EMOTE_UPLOAD', 'EMOTE_TRANSFER', 'EMOTES'), allSets: false },
    { account: { id: 'C', displayName: 'Bo' }, permissions: perms('EMOTE_UPLOAD', 'EMOTE_TRANSFER') },
    { account: { id: 'D', displayName: 'Cy' }, permissions: perms('SETS') },
  ];

  it('drops them on a grant limited to some sets', () => {
    expect(effective(grants[1])).toEqual(['EMOTES']);
    expect(effective(grants[0])).toEqual(['EMOTE_UPLOAD', 'EMOTE_MANAGE']);
    expect(effective(grants[3])).toEqual(['SETS']);
    expect(emotePermissionsIdle(grants[1])).toBe(true);
    expect(emotePermissionsIdle(grants[0])).toBe(false);
    expect(emotePermissionsIdle({ permissions: ['EMOTES'], allSets: false })).toBe(false);
  });

  it('allows editing your own emotes and those of accounts where you manage emotes', () => {
    expect(emoteCan(grants, 'ME', 'ME', 'EMOTE_MANAGE')).toBe(true);
    expect(emoteCan(grants, 'A', 'ME', 'EMOTE_MANAGE')).toBe(true);
    expect(emoteCan(grants, 'B', 'ME', 'EMOTE_MANAGE')).toBe(false);
    expect(emoteCan(grants, 'B', 'ME', 'EMOTE_TRANSFER')).toBe(false);
    // Retired (ADR 0014, amendment 2026/11/22): a grant that still lists it lets nobody transfer.
    expect(emoteCan(grants, 'C', 'ME', 'EMOTE_TRANSFER')).toBe(false);
    expect(emoteCan(grants, 'Z', 'ME', 'EMOTE_MANAGE')).toBe(false);
    expect(emoteCan(grants, 'A', null, 'EMOTE_MANAGE')).toBe(false);
    expect(emoteCan(null, 'A', 'ME', 'EMOTE_MANAGE')).toBe(false);
  });

  it('offers yourself first for uploads, then accounts where you hold Upload on every set', () => {
    expect(uploadTargets(me, grants)).toEqual([
      { id: 'ME', displayName: 'Me', self: true },
      { id: 'C', displayName: 'Bo', self: false },
      { id: 'A', displayName: 'Zed', self: false },
    ]);
    expect(uploadTargets(me, null)).toEqual([{ id: 'ME', displayName: 'Me', self: true }]);
    expect(uploadTargets(null, grants)).toEqual([]);
  });

  it('transfers only to yourself or accounts that trust you with uploads, never the owner', () => {
    expect(transferTargets(me, grants, 'A').map((t) => t.id)).toEqual(['ME', 'C']);
    expect(transferTargets(me, grants, 'ME').map((t) => t.id)).toEqual(['C', 'A']);
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

describe('wording of what an editor can do', () => {
  it('reads as actions, not bare nouns', () => {
    expect(permissionLabels(['EMOTES', 'SETS', 'EDITORS'])).toBe('Edit set emotes, manage sets and manage editors');
    expect(permissionPhrase(['EMOTES', 'PROFILE'])).toBe('edit set emotes and change the profile');
    expect(permissionPhrase([])).toBe('do nothing');
  });
});

describe('the Sets link of an account you edit', () => {
  it('uses the query the sets page reads, not a hash it ignores', () => {
    expect(ownerSetsHref('01ABC')).toBe('/me/sets?owner=01ABC');
    expect(ownerSetsHref('a b')).toBe('/me/sets?owner=a%20b');
  });
});

describe('what you may do with a set', () => {
  const none = new Set<EditorPermission>();
  it('gives a stranger nothing', () => {
    expect(setAccess({ owner: false, setEditor: false, staff: false, held: none })).toEqual({ emotes: false, rename: false, manage: false });
  });
  it('gives the owner and staff everything', () => {
    expect(setAccess({ owner: true, setEditor: false, staff: false, held: none })).toEqual({ emotes: true, rename: true, manage: true });
    expect(setAccess({ owner: false, setEditor: false, staff: true, held: none })).toEqual({ emotes: true, rename: true, manage: true });
  });
  it('lets a per-set editor change the emotes and name but not who sees it', () => {
    expect(setAccess({ owner: false, setEditor: true, staff: false, held: none })).toEqual({ emotes: true, rename: true, manage: false });
  });
  it('follows the account grant: Emotes edits emotes only, Sets manages', () => {
    expect(setAccess({ owner: false, setEditor: false, staff: false, held: new Set<EditorPermission>(['EMOTES']) })).toEqual({ emotes: true, rename: false, manage: false });
    expect(setAccess({ owner: false, setEditor: false, staff: false, held: new Set<EditorPermission>(['SETS']) })).toEqual({ emotes: true, rename: true, manage: true });
    expect(setAccess({ owner: false, setEditor: false, staff: false, held: new Set<EditorPermission>(['PROFILE', 'CHANNELS']) }).emotes).toBe(false);
  });
});

describe('what a grant really allows', () => {
  it('leaves the emote permissions out of the sentence on a set-limited grant', async () => {
    const { effectivePhrase, effectiveLabels } = await import('./editors');
    const limited = { permissions: ['EMOTE_UPLOAD', 'EMOTE_TRANSFER', 'EMOTES'] as EditorPermission[], allSets: false };
    expect(effectivePhrase(limited)).toBe('edit set emotes');
    expect(effectiveLabels(limited)).toBe('Edit set emotes');
    expect(effectivePhrase({ ...limited, allSets: true })).toContain('upload emotes');
    expect(effectivePhrase({ permissions: ['EMOTE_UPLOAD'], allSets: false })).toBe('do nothing');
  });
});
