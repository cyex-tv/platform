import { describe, expect, it } from 'vitest';
import { relative, toItem } from './activity';
import { profileRail, railKeyForPath, stripExtra, stripTabs, sectionFrom, sectionHref, sectionsFor, NOBODY, type Relation } from './rows';

const now = Date.UTC(2026, 10, 17, 12, 0, 0);
const ago = (s: number) => new Date(now - s * 1000).toISOString();

describe('relative', () => {
  it('reads like a sentence', () => {
    expect(relative(ago(10), now)).toBe('just now');
    expect(relative(ago(60), now)).toBe('a minute ago');
    expect(relative(ago(5 * 60), now)).toBe('5 minutes ago');
    expect(relative(ago(3600), now)).toBe('an hour ago');
    expect(relative(ago(3 * 86400), now)).toBe('3 days ago');
    expect(relative(ago(32 * 86400), now)).toBe('a month ago');
    expect(relative(ago(70 * 86400), now)).toBe('2 months ago');
    expect(relative('nope', now)).toBe('');
  });
});

const mine: Relation = { mine: true, perms: new Set(['EMOTES', 'SETS', 'EDITORS', 'PROFILE']) };
const editor: Relation = { mine: false, perms: new Set(['EMOTES']) };
const profiler: Relation = { mine: false, perms: new Set(['PROFILE']) };

describe('toItem', () => {
  const base = { id: 'a1', actor: { id: 'u', displayName: 'U', avatar: null }, set: { id: 's', name: 'Main' }, emote: null,
    fromName: null, toName: null, badge: null, paint: null, createdAt: '2026-11-17T12:00:00Z' };
  it('maps the API kinds', () => {
    expect(toItem({ ...base, kind: 'SET_EMOTE_ADDED' })?.kind).toBe('added');
    expect(toItem({ ...base, kind: 'COSMETICS_SELECTED' })?.kind).toBe('cosmetics');
    expect(toItem({ ...base, kind: 'SET_RENAMED', fromName: 'a', toName: 'b' })).toMatchObject({ kind: 'setRenamed', from: 'a', to: 'b' });
  });
  it('drops kinds it doesn’t know', () => {
    expect(toItem({ ...base, kind: 'SOMETHING_NEW' as never })).toBeNull();
  });
});

describe('profile rows', () => {
  it('gives the public the uploads, the listed sets and the activity', () => {
    expect(sectionsFor(NOBODY, false)).toEqual(['uploads', 'sets']);
    expect(sectionsFor(NOBODY, true)).toEqual(['uploads', 'sets', 'activity']);
  });
  it('gives the owner everything, activity only when the API has it', () => {
    expect(sectionsFor(mine, false)).toEqual(['active', 'uploads', 'sets', 'cosmetics']);
    expect(sectionsFor(mine, true)).toContain('activity');
  });
  it('gives editors cosmetics only with PROFILE', () => {
    expect(sectionsFor(editor, false)).not.toContain('cosmetics');
    expect(sectionsFor(profiler, false)).toContain('cosmetics');
  });
  it('links the default section without a parameter', () => {
    expect(sectionHref('a1', 'uploads')).toBe('/users/a1');
    expect(sectionHref('a1', 'sets')).toBe('/users/a1?s=sets');
  });
  it('falls back to uploads for sections the viewer lacks', () => {
    expect(sectionFrom(new URLSearchParams('s=cosmetics'), ['uploads'])).toBe('uploads');
    expect(sectionFrom(new URLSearchParams('s=sets'), sectionsFor(mine, false))).toBe('sets');
  });
  it('puts the /me pages under their row', () => {
    expect(railKeyForPath('/me/sets/x')).toBe('sets');
    expect(railKeyForPath('/me/emotes/x')).toBe('uploads');
    expect(railKeyForPath('/me/upload')).toBe('uploads');
    expect(railKeyForPath('/me/account')).toBe('connections');
  });
  it('marks one current row, with Channels for the owner only', () => {
    const rows = profileRail('a1', mine, 'sets', false).flatMap((g) => g.items);
    expect(rows.filter((r) => r.current).map((r) => r.label)).toEqual(['Emote sets']);
    expect(rows.some((r) => r.href === '/me/channels')).toBe(true);
    expect(profileRail('a1', editor, null, false).flatMap((g) => g.items).some((r) => r.href === '/me/channels')).toBe(false);
  });
});

describe('the phone strip on /me pages without a row', () => {
  it('names Upload, Notifications, Account and Editors, nothing else', () => {
    expect(stripExtra('/me/upload')?.label).toBe('Upload');
    expect(stripExtra('/me/notifications')?.label).toBe('Notifications');
    expect(stripExtra('/me/account')?.label).toBe('Account');
    expect(stripExtra('/me/editors')?.label).toBe('Editors');
    expect(stripExtra('/me/sets')).toBeNull();
    expect(stripExtra('/me/uploads-old')).toBeNull();
    expect(stripExtra('/users/x')).toBeNull();
  });
  it('marks the extra tab and no other', () => {
    const groups = profileRail('a', { mine: true, perms: new Set() }, 'uploads', false);
    expect(stripTabs(groups, stripExtra('/me/upload')).filter((t) => t.current).map((t) => t.label)).toEqual(['Upload']);
    expect(stripTabs(groups, null).filter((t) => t.current).map((t) => t.label)).toEqual(['Uploaded emotes']);
  });
});
