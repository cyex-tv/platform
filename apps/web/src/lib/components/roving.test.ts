import { describe, expect, it } from 'vitest';
import { nextIndex } from './roving';

describe('nextIndex', () => {
  it('steps down and stops at the end', () => {
    expect(nextIndex('ArrowDown', 0, 3)).toBe(1);
    expect(nextIndex('ArrowDown', 2, 3)).toBe(2);
    expect(nextIndex('ArrowDown', -1, 3)).toBe(0);
  });
  it('steps up and stops at the start', () => {
    expect(nextIndex('ArrowUp', 2, 3)).toBe(1);
    expect(nextIndex('ArrowUp', 0, 3)).toBe(0);
    expect(nextIndex('ArrowUp', -1, 3)).toBe(2);
  });
  it('jumps with Home and End', () => {
    expect(nextIndex('Home', 2, 3)).toBe(0);
    expect(nextIndex('End', 0, 3)).toBe(2);
  });
  it('leaves other keys and empty lists alone', () => {
    expect(nextIndex('Tab', 0, 3)).toBeNull();
    expect(nextIndex('Escape', 0, 3)).toBeNull();
    expect(nextIndex('ArrowDown', 0, 0)).toBeNull();
  });
});
