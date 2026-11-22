import { describe, expect, it } from 'vitest';
import { PERMISSION_GROUPS } from './permissions';
import { decisionGroups, decisionState, decisionText, effectOf, groupCounts, setEffect, setGroup, summarize } from './role-effects';

const moderation = PERMISSION_GROUPS.find((g) => g.id === 'moderation')!;
const none = { allows: [], denies: [] };

describe('a role’s effect on a permission', () => {
  it('is allow, deny or inherit (not set)', () => {
    const sets = { allows: ['emotes.review'], denies: ['emotes.upload'] };
    expect(effectOf(sets, 'emotes.review')).toBe('allow');
    expect(effectOf(sets, 'emotes.upload')).toBe('deny');
    expect(effectOf(sets, 'users.view')).toBe('inherit');
  });

  it('moves a name between the lists and never keeps it in both', () => {
    let s = setEffect(none, 'emotes.upload', 'deny');
    expect(s).toEqual({ allows: [], denies: ['emotes.upload'] });
    s = setEffect(s, 'emotes.upload', 'allow');
    expect(s).toEqual({ allows: ['emotes.upload'], denies: [] });
    s = setEffect(s, 'emotes.upload', 'inherit');
    expect(s).toEqual(none);
  });

  it('keeps the lists in the editor’s order', () => {
    const s = setEffect(setEffect(none, 'users.block', 'deny'), 'emotes.upload', 'deny');
    expect(s.denies).toEqual(['emotes.upload', 'users.block']);
  });

  it('sets a whole group, leaving what you may not change as it is', () => {
    const mayChange = (k: string) => k !== 'users.block';
    const start = { allows: ['users.block'], denies: [] };
    const denied = setGroup(start, moderation, 'deny', mayChange);
    expect(denied.allows).toEqual(['users.block']);
    expect(denied.denies).toEqual(['users.restrict_uploads', 'users.restrict_tickets', 'users.restrict']);
    expect(groupCounts(denied, moderation)).toEqual({ allow: 1, deny: 3, of: 4 });
    expect(setGroup(denied, moderation, 'inherit', mayChange)).toEqual({ allows: ['users.block'], denies: [] });
    expect(setGroup(none, moderation, 'allow').allows).toHaveLength(moderation.items.length);
  });

  it('summarizes keys by group', () => {
    const out = summarize(['users.block', 'emotes.upload'], PERMISSION_GROUPS);
    expect(out.map((g) => g.label)).toEqual(['Emotes', 'Moderation']);
    expect(out[1]).toMatchObject({ n: 1, of: 4, names: 'Block accounts' });
  });
});

describe('what decided an account’s permission', () => {
  const d = (o: Partial<Parameters<typeof decisionText>[0]>) => ({ permission: 'emotes.upload', granted: false, owner: false, ...o });

  it('says it in a line', () => {
    expect(decisionText(d({ granted: true, effect: 'ALLOW', roleName: 'Moderator' }))).toBe('Allowed by Moderator');
    expect(decisionText(d({ effect: 'DENY', roleName: 'Muted' }))).toBe('Denied by Muted');
    expect(decisionText(d({ granted: true, owner: true }))).toBe('Owner');
    expect(decisionText(d({}))).toBe('Not granted');
  });

  it('has three states', () => {
    expect(decisionState(d({ granted: true, effect: 'ALLOW' }))).toBe('allowed');
    expect(decisionState(d({ effect: 'DENY' }))).toBe('denied');
    expect(decisionState(d({}))).toBe('none');
  });

  it('groups the decisions like the editor', () => {
    const g = decisionGroups([d({}), d({ permission: 'users.block' })], PERMISSION_GROUPS);
    expect(g.map((x) => x.label)).toEqual(['Emotes', 'Moderation']);
    expect(g[0].items[0].label).toBe('Upload emotes');
  });
});
