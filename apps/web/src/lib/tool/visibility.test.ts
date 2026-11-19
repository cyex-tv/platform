import { describe, expect, it } from 'vitest';
import { VISIBILITY_HINT, VISIBILITY_OPTIONS, visibilityOf } from './visibility';

describe('emote visibility', () => {
  it('offers public, unlisted and private, in that order, each with a hint', () => {
    expect(VISIBILITY_OPTIONS.map((o) => o.value)).toEqual(['PUBLIC', 'UNLISTED', 'PRIVATE']);
    for (const o of VISIBILITY_OPTIONS) expect(VISIBILITY_HINT[o.value]).toBeTruthy();
    expect(VISIBILITY_HINT.UNLISTED).toMatch(/link/);
  });

  it('reads the field, falling back to the older private flag', () => {
    expect(visibilityOf({ visibility: 'UNLISTED', flags: { private: false } })).toBe('UNLISTED');
    expect(visibilityOf({ flags: { private: true } })).toBe('PRIVATE');
    expect(visibilityOf({ visibility: null, flags: { private: false } })).toBe('PUBLIC');
  });
});
