import { describe, expect, it } from 'vitest';
import * as visibility from './visibility';
import { setMark } from './visibility';

describe('set visibility', () => {
  it('offers no choice any more: every set is public', () => {
    expect(Object.keys(visibility)).toEqual(['setMark']);
  });

  it('marks only sets staff took off the listings', () => {
    expect(setMark('LISTED')).toBeNull();
    expect(setMark(undefined)).toBeNull();
    expect(setMark('UNLISTED')).toEqual({ icon: 'link', label: 'Unlisted by staff' });
    expect(setMark('PRIVATE')).toEqual({ icon: 'lock', label: 'Hidden by staff' });
  });
});