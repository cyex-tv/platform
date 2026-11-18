import { describe, expect, it } from 'vitest';
import { plan, summary } from './bulk';

describe('plan', () => {
  it('skips what the set holds and stops at the free places', () => {
    const p = plan(['a', 'b', 'c', 'd'], new Set(['b']), 2);
    expect(p).toEqual({ add: ['a', 'c'], already: ['b'], overflow: ['d'] });
  });
  it('a full set takes nothing', () => {
    expect(plan(['a'], new Set(), 0).overflow).toEqual(['a']);
    expect(plan(['a'], new Set(), -3).add).toEqual([]);
  });
});

describe('summary', () => {
  it('says what happened in one short paragraph', () => {
    expect(summary('Main', { added: 8, already: 1, overflow: 2, taken: 0, failed: 0 })).toBe(
      'Added 8 to Main. 1 was already there. 2 didn’t fit: the set is full.',
    );
    expect(summary('Main', { added: 0, already: 0, overflow: 0, taken: 1, failed: 2 })).toBe(
      'Nothing was added to Main. 1 has a name the set already uses. 2 couldn’t be added.',
    );
  });
});
