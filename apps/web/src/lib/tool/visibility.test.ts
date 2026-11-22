import { describe, expect, it } from 'vitest';
import { FLAG_HINT, STAFF_VISIBILITY_HINT, VISIBILITY_OPTIONS, visibilityOf } from './visibility';

describe('emote visibility', () => {
  it('offers public, unlisted and private, in that order, each with a hint', () => {
    expect(VISIBILITY_OPTIONS.map((o) => o.value)).toEqual(['PUBLIC', 'UNLISTED', 'PRIVATE']);
    for (const o of VISIBILITY_OPTIONS) expect(STAFF_VISIBILITY_HINT[o.value]).toBeTruthy();
    expect(STAFF_VISIBILITY_HINT.UNLISTED).toMatch(/link/);
    // Creators get two flags only: overlaying and private (no unlisted, no NSFW).
    expect(Object.keys(FLAG_HINT)).toEqual(['overlaying', 'private']);
    expect(FLAG_HINT.private).toMatch(/own channel/);
  });

  it('reads the field, falling back to the older private flag', () => {
    expect(visibilityOf({ visibility: 'UNLISTED', flags: { private: false } })).toBe('UNLISTED');
    expect(visibilityOf({ flags: { private: true } })).toBe('PRIVATE');
    expect(visibilityOf({ visibility: null, flags: { private: false } })).toBe('PUBLIC');
  });
});
