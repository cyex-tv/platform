import { describe, expect, it } from 'vitest';
import { defaultVisibility, setMark, visibilityHint, visibilityOptions } from './visibility';

describe('set visibility', () => {
  it('offers listed, unlisted and private, with icons', () => {
    const o = visibilityOptions('CHANNEL');
    expect(o.map((x) => x.value)).toEqual(['LISTED', 'UNLISTED', 'PRIVATE']);
    expect(o.map((x) => x.label)).toEqual(['Listed', 'Unlisted', 'Private']);
    expect(o.every((x) => x.icon)).toBe(true);
    expect(o.every((x) => !x.disabled)).toBe(true);
  });
  it('switches private off for a personal set, which the API refuses', () => {
    const o = visibilityOptions('PERSONAL');
    expect(o.find((x) => x.value === 'PRIVATE')?.disabled).toBe(true);
    expect(o.find((x) => x.value === 'LISTED')?.disabled).toBeFalsy();
  });
  it('defaults like the API: channel sets listed, personal unlisted', () => {
    expect(defaultVisibility('CHANNEL')).toBe('LISTED');
    expect(defaultVisibility('PERSONAL')).toBe('UNLISTED');
  });
  it('says each state in one plain line', () => {
    expect(visibilityHint('LISTED')).toMatch(/profile/i);
    expect(visibilityHint('UNLISTED')).toMatch(/link/i);
    expect(visibilityHint('PRIVATE')).toMatch(/only/i);
    expect(new Set(['LISTED', 'UNLISTED', 'PRIVATE'].map((v) => visibilityHint(v as never))).size).toBe(3);
  });
  it('marks only the sets that are not listed', () => {
    expect(setMark('LISTED')).toBeNull();
    expect(setMark(undefined)).toBeNull();
    expect(setMark('UNLISTED')).toEqual({ icon: 'link', label: 'Unlisted' });
    expect(setMark('PRIVATE')).toEqual({ icon: 'lock', label: 'Private' });
  });
});
