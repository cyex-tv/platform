import { describe, expect, it } from 'vitest';
import { filterOf, relative, toItem } from './activity';
import { profileRail, railKeyForPath, stripExtra, stripTabs, sectionFrom, sectionHref, sectionsFor, NOBODY, RAIL_SECTIONS, type Relation } from './rows';

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

describe('editor lines and the filter', () => {
  const base = { id: 'a1', actor: { id: 'u', displayName: 'U', avatar: null }, set: null, emote: null,
    fromName: null, toName: null, badge: null, paint: null, createdAt: '2026-11-17T12:00:00Z' };
  it('maps editor lines with the editor, names only', () => {
    const ed = { id: 'e', displayName: 'Ed', avatar: null };
    expect(toItem({ ...base, kind: 'EDITOR_ADDED', editor: ed })).toMatchObject({ kind: 'editorAdded', editor: ed });
    expect(toItem({ ...base, kind: 'EDITOR_REMOVED', editor: ed })?.kind).toBe('editorRemoved');
    expect(toItem({ ...base, kind: 'SET_CREATED' })?.editor).toBeNull();
  });
  it('asks for every group (editors too) unless one is picked', () => {
    expect(filterOf({})).toEqual({ groups: ['EMOTES', 'SETS', 'COSMETICS', 'EDITORS'], actorId: null, since: null, until: null });
    expect(filterOf({ group: 'cosmetics', actor: 'x', since: 'a', until: 'b' })).toEqual({ groups: ['COSMETICS'], actorId: 'x', since: 'a', until: 'b' });
  });
});

describe('profile rows', () => {
  it('gives the public the uploads, the listed sets, the cosmetics and the activity', () => {
    expect(sectionsFor(NOBODY, false)).toEqual(['uploads', 'sets', 'cosmetics']);
    expect(sectionsFor(NOBODY, true)).toEqual(['uploads', 'sets', 'cosmetics', 'activity']);
  });
  it('has no Collection any more, for anyone', () => {
    for (const r of [NOBODY, editor, profiler, mine]) expect(sectionsFor(r, true)).not.toContain('collection');
    expect(profileRail('a1', 'uploads', true).flatMap((g) => g.items).some((r) => r.label === 'Collection')).toBe(false);
  });
  it('lets the owner open every section, activity only when the API has it', () => {
    expect(sectionsFor(mine, false)).toEqual(['overview', 'active', 'uploads', 'sets', 'favourites', 'cosmetics']);
    expect(sectionsFor(mine, true)).toContain('activity');
  });
  it('gives the owner alone Favourites and Overview (private, never in the rail)', () => {
    for (const r of [NOBODY, editor, profiler]) {
      expect(sectionsFor(r, true)).not.toContain('favourites');
      expect(sectionsFor(r, true)).not.toContain('overview');
    }
    expect(sectionHref('a1', 'overview')).toBe('/users/a1?s=overview');
    expect(railKeyForPath('/me/favourites')).toBe('favourites');
  });
  it('the rail is the same public rows for every viewer: uploads, sets, cosmetics, activity', () => {
    const rows = profileRail('a1', null, true).flatMap((g) => g.items);
    expect(rows.map((r) => r.label)).toEqual(['Uploaded emotes', 'Emote sets', 'Cosmetics', 'Activity']);
    expect(rows.map((r) => r.href)).toEqual(['/users/a1', '/users/a1?s=sets', '/users/a1?s=cosmetics', '/users/a1?s=activity']);
    expect(RAIL_SECTIONS.every((k) => sectionsFor(NOBODY, true).includes(k))).toBe(true);
    // Nothing that only the owner may open: no /me page, no Overview, Active emotes, Favourites or Channels.
    expect(rows.some((r) => r.href.startsWith('/me'))).toBe(false);
    expect(profileRail('a1', null, false).flatMap((g) => g.items).map((r) => r.label)).toEqual(['Uploaded emotes', 'Emote sets', 'Cosmetics']);
  });
  it('links the default section without a parameter', () => {
    expect(sectionHref('a1', 'uploads')).toBe('/users/a1');
    expect(sectionHref('a1', 'sets')).toBe('/users/a1?s=sets');
  });
  it('falls back to uploads for sections the viewer lacks', () => {
    expect(sectionFrom(new URLSearchParams('s=overview'), sectionsFor(NOBODY, true))).toBe('uploads');
    expect(sectionFrom(new URLSearchParams('s=sets'), sectionsFor(mine, false))).toBe('sets');
    expect(sectionFrom(new URLSearchParams('s=cosmetics'), sectionsFor(NOBODY, false))).toBe('cosmetics');
  });
  it('puts the /me pages under their row', () => {
    expect(railKeyForPath('/me/sets/x')).toBe('sets');
    expect(railKeyForPath('/me/emotes/x')).toBe('uploads');
    expect(railKeyForPath('/me/upload')).toBe('uploads');
    expect(railKeyForPath('/me/cosmetics')).toBe('cosmetics');
    expect(railKeyForPath('/me/account')).toBe('connections');
  });
  it('marks one current row; an owner-only section marks none', () => {
    const rows = profileRail('a1', 'sets', false).flatMap((g) => g.items);
    expect(rows.filter((r) => r.current).map((r) => r.label)).toEqual(['Emote sets']);
    expect(profileRail('a1', 'overview', true).flatMap((g) => g.items).some((r) => r.current)).toBe(false);
  });
});

describe('the phone strip on pages without a rail row', () => {
  it('names Upload, Notifications, Account, Editors, Channels, Favourites, and the owner-only sections', () => {
    expect(stripExtra('/me/upload')?.label).toBe('Upload');
    expect(stripExtra('/me/notifications')?.label).toBe('Notifications');
    expect(stripExtra('/me/account')?.label).toBe('Account');
    expect(stripExtra('/me/editors')?.label).toBe('Editors');
    expect(stripExtra('/me/channels')?.label).toBe('Channels');
    expect(stripExtra('/me/favourites')?.label).toBe('Favourites');
    expect(stripExtra('/users/a1', 'overview')).toMatchObject({ label: 'Overview', href: '/users/a1?s=overview', current: true });
    expect(stripExtra('/users/a1', 'active')?.label).toBe('Active emotes');
    expect(stripExtra('/users/a1', 'sets')).toBeNull();
    expect(stripExtra('/me/sets')).toBeNull();
    expect(stripExtra('/me/uploads-old')).toBeNull();
    expect(stripExtra('/users/x')).toBeNull();
  });
  it('marks the extra tab and no other', () => {
    const groups = profileRail('a', 'uploads', false);
    expect(stripTabs(groups, stripExtra('/me/upload')).filter((t) => t.current).map((t) => t.label)).toEqual(['Upload']);
    expect(stripTabs(groups, null).filter((t) => t.current).map((t) => t.label)).toEqual(['Uploaded emotes']);
  });
});
