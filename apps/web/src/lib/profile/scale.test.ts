import { describe, expect, it } from 'vitest';
import { byName, EDITORS_STEP, fold, moreText, shown, showing } from './scale';

describe('bounded profile lists', () => {
  it('show one step, then grow a step at a time, never past the total', () => {
    expect(shown(50, 1, EDITORS_STEP)).toBe(10);
    expect(shown(50, 3, EDITORS_STEP)).toBe(30);
    expect(shown(50, 9, EDITORS_STEP)).toBe(50);
    expect(shown(4, 1, EDITORS_STEP)).toBe(4);
    expect(shown(0, 0, EDITORS_STEP)).toBe(0);
  });
  it('say how many show of how many, in cyex numbers', () => {
    expect(showing(24, 1200)).toBe('Showing 24 of 1.200');
    expect(showing(50, 50)).toBe('');
    expect(moreText(40, 50, 10)).toBe('Show 10 more');
    expect(moreText(45, 50, 10)).toBe('Show 5 more');
  });
  it('fold role flags past the first few', () => {
    expect(fold(['a', 'b', 'c', 'd', 'e'], 3)).toEqual({ head: ['a', 'b', 'c'], rest: 2 });
    expect(fold(['a'], 3)).toEqual({ head: ['a'], rest: 0 });
  });
  it('filter sets by name, any case', () => {
    const sets = [{ name: 'Main' }, { name: 'Subs only' }, { name: 'mainline' }];
    expect(byName(sets, ' MAIN ').map((s) => s.name)).toEqual(['Main', 'mainline']);
    expect(byName(sets, '')).toHaveLength(3);
  });
});
