import { describe, expect, it } from 'vitest';
import { nextSegment } from './segmented';

describe('nextSegment', () => {
  const free = [false, false, false];
  it('moves right and left, wrapping at the ends', () => {
    expect(nextSegment('ArrowRight', 0, free)).toBe(1);
    expect(nextSegment('ArrowDown', 2, free)).toBe(0);
    expect(nextSegment('ArrowLeft', 0, free)).toBe(2);
    expect(nextSegment('ArrowUp', 2, free)).toBe(1);
  });
  it('jumps with Home and End', () => {
    expect(nextSegment('Home', 2, free)).toBe(0);
    expect(nextSegment('End', 0, free)).toBe(2);
  });
  it('skips disabled options', () => {
    const d = [false, true, false];
    expect(nextSegment('ArrowRight', 0, d)).toBe(2);
    expect(nextSegment('ArrowLeft', 2, d)).toBe(0);
    expect(nextSegment('Home', 1, [true, false, false])).toBe(1);
    expect(nextSegment('End', 0, [false, false, true])).toBe(1);
  });
  it('ignores other keys and a group with nothing enabled', () => {
    expect(nextSegment('Enter', 0, free)).toBe(-1);
    expect(nextSegment('ArrowRight', 0, [true, true])).toBe(-1);
    expect(nextSegment('Home', 0, [])).toBe(-1);
  });
});
