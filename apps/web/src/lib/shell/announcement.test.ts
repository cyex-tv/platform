import { describe, expect, it } from 'vitest';
import { DISMISS_MAX, closedCookie, emoteIdFrom, mergeClosed, readClosedCookie, external, phase, readClosed, running, safeLink, withClosed } from './announcement';

describe('closing the announcement', () => {
  it('reads only a list of strings', () => {
    expect(readClosed(null)).toEqual([]);
    expect(readClosed('nope')).toEqual([]);
    expect(readClosed('{"a":1}')).toEqual([]);
    expect(readClosed('["a", 3, "b"]')).toEqual(['a', 'b']);
  });

  it('remembers each one once, newest last, capped', () => {
    expect(withClosed(['a', 'b'], 'a')).toEqual(['b', 'a']);
    const many = Array.from({ length: DISMISS_MAX }, (_, i) => `x${i}`);
    const next = withClosed(many, 'new');
    expect(next).toHaveLength(DISMISS_MAX);
    expect(next.at(-1)).toBe('new');
    expect(next).not.toContain('x0');
  });
});

describe('the closed-list cookie (the server reads it, so the bar never moves)', () => {
  it('keeps only IDs, capped, and reads back what it wrote', () => {
    expect(readClosedCookie(undefined)).toEqual([]);
    expect(readClosedCookie('A1.B2')).toEqual(['A1', 'B2']);
    expect(readClosedCookie('A1.<script>.B2')).toEqual(['A1', 'B2']);
    expect(closedCookie(['A1', 'x y', 'B2'])).toBe('A1.B2');
    const many = Array.from({ length: DISMISS_MAX + 5 }, (_, i) => `X${i}`);
    expect(readClosedCookie(closedCookie(many))).toEqual(many.slice(-DISMISS_MAX));
  });

  it('merges the cookie with an older localStorage list, each ID once', () => {
    expect(mergeClosed(['A', 'B'], ['B', 'C'])).toEqual(['A', 'B', 'C']);
  });
});

describe('links', () => {
  it('keeps paths here and https only', () => {
    expect(safeLink('/shop')).toBe('/shop');
    expect(safeLink(' https://example.com/a ')).toBe('https://example.com/a');
    for (const bad of ['//evil.example', '/\\evil', '/\t/evil.example', '/\n/x', ' /\t/evil.example', 'http://example.com', 'javascript:alert(1)', 'data:text/html,x', '', null]) expect(safeLink(bad)).toBeNull();
  });

  it('opens only other sites in a new tab', () => {
    expect(external('/shop')).toBe(false);
    expect(external('https://example.com/')).toBe(true);
  });
});

describe('time', () => {
  const now = Date.parse('2026-11-20T12:00:00Z');
  it('stops showing once over or ended', () => {
    expect(running({ endsAt: null }, now)).toBe(true);
    expect(running({ endsAt: '2026-11-20T11:00:00Z' }, now)).toBe(false);
    expect(running({ endsAt: '2026-11-21T00:00:00Z', endedAt: '2026-11-20T10:00:00Z' }, now)).toBe(false);
  });

  it('says where each stands on the admin page', () => {
    expect(phase({ live: true }, now)).toBe('live');
    expect(phase({ live: false, startsAt: '2026-12-01T00:00:00Z' }, now)).toBe('scheduled');
    expect(phase({ live: false, endedAt: '2026-11-20T10:00:00Z' }, now)).toBe('ended');
    expect(phase({ live: true, endsAt: '2026-11-20T11:00:00Z' }, now)).toBe('ended');
  });
});

describe('the emote field', () => {
  it('takes an ID or a link to the emote page', () => {
    expect(emoteIdFrom(' 01JD8XEMOTE ')).toBe('01JD8XEMOTE');
    expect(emoteIdFrom('https://cyex.tv/emotes/01JD8XEMOTE?x=1')).toBe('01JD8XEMOTE');
    expect(emoteIdFrom('/emotes/abc_-1')).toBe('abc_-1');
    expect(emoteIdFrom('not an id')).toBeNull();
    expect(emoteIdFrom('')).toBeNull();
  });
});
