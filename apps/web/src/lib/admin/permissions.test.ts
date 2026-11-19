import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { groupState, groupsFor, PERMISSION_GROUPS, permissionInfo, permissionLabel, toggleGroup } from './permissions';

// The API's permission names, read from its source so a new one can't go unlabelled.
const rust = readFileSync(new URL('../../../../../crates/shared/src/permissions.rs', import.meta.url), 'utf8');
const API = [...rust.matchAll(/pub const [A-Z_]+: &str = "([a-z_.]+)";/g)].map((m) => m[1]);

describe('permission labels', () => {
  it('names every permission the API has, once, in one group', () => {
    expect(API.length).toBeGreaterThan(20);
    const keys = PERMISSION_GROUPS.flatMap((g) => g.items.map((i) => i.key));
    expect(new Set(keys).size).toBe(keys.length);
    expect([...keys].sort()).toEqual([...API].sort());
  });

  it('gives each a short label and a sentence for the tooltip', () => {
    for (const g of PERMISSION_GROUPS)
      for (const i of g.items) {
        expect(i.label.length, i.key).toBeLessThanOrEqual(28);
        expect(i.label).not.toContain('.');
        expect(i.hint, i.key).toMatch(/^[A-Z].*\.$/);
        expect(i.hint).not.toBe(i.label);
      }
  });

  it('makes unknown names readable instead of failing', () => {
    expect(permissionLabel('emotes.review')).toBe('Review uploads');
    expect(permissionInfo('payouts.send_money')).toEqual({ key: 'payouts.send_money', label: 'Send money', hint: 'payouts.send_money' });
    expect(permissionLabel('weird')).toBe('Weird');
  });

  it('groups what the API lists, unknown names last', () => {
    const groups = groupsFor(['audit.view', 'emotes.review', 'later.thing']);
    expect(groups.map((g) => g.label)).toEqual(['Emotes', 'System', 'Other']);
    expect(groups[2].items[0].label).toBe('Thing');
  });
});

describe('group checkbox', () => {
  const mod = PERMISSION_GROUPS.find((g) => g.id === 'moderation')!;
  const keys = mod.items.map((i) => i.key);
  const all = () => true;

  it('is empty, partial (indeterminate) or full', () => {
    expect(groupState(mod, [])).toBe('none');
    expect(groupState(mod, ['users.block', 'emotes.review'])).toBe('some');
    expect(groupState(mod, keys)).toBe('all');
  });

  it('picks the whole group, then clears it', () => {
    const on = toggleGroup(mod, ['emotes.review', 'users.block'], all);
    expect(groupState(mod, on)).toBe('all');
    expect(on).toContain('emotes.review');
    // In the editor's order: group by group.
    expect(on[0]).toBe('emotes.review');
    const off = toggleGroup(mod, on, all);
    expect(off).toEqual(['emotes.review']);
  });

  it('leaves what you may not grant or take away alone', () => {
    const mayChange = (k: string) => k !== 'users.restrict';
    // You lack users.restrict: picking the group picks the rest, and stays partial.
    const on = toggleGroup(mod, [], mayChange);
    expect(on).not.toContain('users.restrict');
    expect(groupState(mod, on)).toBe('some');
    // Clicking again clears what you may change, not what someone above you set.
    const off = toggleGroup(mod, [...on, 'users.restrict'], mayChange);
    expect(off).toEqual(['users.restrict']);
  });
});
