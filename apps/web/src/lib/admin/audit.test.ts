import { describe, expect, it } from 'vitest';
import { actionText, changes, dayAfter, dayLabel, dayStart, looksLikeId, targetHref } from './audit';

describe('audit helpers', () => {
  it('shows a day in the cyex format', () => {
    expect(dayLabel('2026-11-20')).toBe('2026/11/20');
    expect(dayLabel('')).toBe('');
    expect(dayLabel('20.11.2026')).toBe('');
  });

  it('words an action', () => {
    expect(actionText('role.updated')).toBe('Role updated');
    expect(actionText('set.emote_added')).toBe('Set emote added');
    expect(actionText('')).toBe('');
  });

  it('links targets that have a page', () => {
    expect(targetHref('account', 'A1')).toBe('/admin/users/A1');
    expect(targetHref('emote', 'a b')).toBe('/me/emotes/a%20b');
    expect(targetHref('mystery', 'x')).toBeNull();
  });

  it('lists only what changed', () => {
    const out = changes('{"state":"pending","n":1,"gone":true}', '{"state":"approved","n":1,"new":{"a":2}}');
    expect(out).toEqual([
      { path: 'gone', before: 'true', kind: 'removed' },
      { path: 'new.a', after: '2', kind: 'added' },
      { path: 'state', before: 'pending', after: 'approved', kind: 'changed' },
    ]);
  });

  it('copes with empty and invalid sides', () => {
    expect(changes(null, '{"a":1}')).toEqual([{ path: 'a', after: '1', kind: 'added' }]);
    expect(changes('{"a":1}', null)).toEqual([{ path: 'a', before: '1', kind: 'removed' }]);
    expect(changes('nope', 'nope')).toEqual([]);
    expect(changes(null, null)).toEqual([]);
  });

  it('turns a day into a half-open window', () => {
    const a = dayStart('2026-11-17')!;
    const b = dayAfter('2026-11-17')!;
    expect(new Date(b).getTime() - new Date(a).getTime()).toBeGreaterThanOrEqual(23 * 3600_000);
    expect(dayStart('17/11/2026')).toBeNull();
  });

  it('recognises an account id', () => {
    expect(looksLikeId('01JD8X0000000000000000MEME')).toBe(true);
    expect(looksLikeId('lumen_fox')).toBe(false);
  });
});
