import { describe, expect, it } from 'vitest';
import { age, flagWords, groupReports, newestReport, nextKey, sizesOf } from './reports';

const r = (id: string, emote: string | null, createdAt = '2026-11-20T10:00:00Z') => ({ id, createdAt, emote: emote ? { id: emote } : null });

describe('report groups', () => {
  it('groups by emote with the most reported first, and keeps a deleted emote’s reports apart', () => {
    const g = groupReports([r('1', 'a'), r('2', 'b'), r('3', 'b'), r('4', null), r('5', null)]);
    expect(g.map((x) => [x.key, x.reports.length])).toEqual([['b', 2], ['a', 1], ['gone-4', 1], ['gone-5', 1]]);
  });

  it('finds the newest report', () => {
    expect(newestReport([r('1', 'a', '2026-11-19T10:00:00Z'), r('2', 'a', '2026-11-20T10:00:00Z')])?.id).toBe('2');
    expect(newestReport([])).toBeUndefined();
  });

  it('says how old in a short form', () => {
    const now = Date.parse('2026-11-21T12:00:00Z');
    expect(age('2026-11-21T11:59:40Z', now)).toBe('now');
    expect(age('2026-11-21T11:30:00Z', now)).toBe('30m');
    expect(age('2026-11-21T09:00:00Z', now)).toBe('3h');
    expect(age('2026-11-19T12:00:00Z', now)).toBe('2d');
    expect(age('2026-10-01T12:00:00Z', now)).toMatch(/^2026\/10\/0[12]$/);
    expect(age('nope', now)).toBe('');
  });

  it('picks the row that took the place of a resolved one', () => {
    expect(nextKey(['a', 'b', 'c'], 1)).toBe('b');
    expect(nextKey(['a', 'b'], 5)).toBe('b');
    expect(nextKey([], 0)).toBeNull();
  });
});

describe('the inspector helpers', () => {
  const f = (name: string, scale: number, isStatic = false) => ({ name, scale, width: 32 * scale, height: 32 * scale, isStatic });
  it('lists one picture per scale, smallest first, without stills', () => {
    const e = { files: [f('2x.webp', 2), f('1x_static.webp', 1, true), f('1x.webp', 1), f('1x.avif', 1)] };
    expect(sizesOf(e).map((x) => x.name)).toEqual(['1x.webp', '2x.webp']);
  });
  it('words the flags', () => {
    expect(flagWords({ animated: true, flags: { zeroWidth: false, nsfw: true, private: false, wide: false } })).toEqual(['Animated', 'NSFW']);
    expect(flagWords({ animated: false, flags: { zeroWidth: false, nsfw: false, private: false, wide: false } })).toEqual([]);
  });
});
