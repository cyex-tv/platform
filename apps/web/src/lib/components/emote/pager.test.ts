import { describe, expect, it } from 'vitest';
import { CursorTrail } from './pager';

describe('cursor trail', () => {
  it('starts at the newest rows with no next page known', () => {
    const t = new CursorTrail();
    expect(t.start(0)).toBeNull();
    expect(t.start(1)).toBeUndefined();
    expect(t.hasNext(0)).toBe(false);
  });
  it('remembers where each page starts, so Previous goes back without asking', () => {
    const t = new CursorTrail();
    t.after(0, 'c1');
    t.after(1, 'c2');
    expect([t.start(0), t.start(1), t.start(2)]).toEqual([null, 'c1', 'c2']);
    expect(t.hasNext(1)).toBe(true);
  });
  it('ends where a page says there is no more', () => {
    const t = new CursorTrail();
    t.after(0, 'c1');
    t.after(1, null);
    expect(t.hasNext(1)).toBe(false);
    expect(t.start(2)).toBeUndefined();
  });
  it('forgets later pages when an earlier one is loaded again with another cursor', () => {
    const t = new CursorTrail();
    t.after(0, 'c1');
    t.after(1, 'c2');
    t.after(0, 'x1');
    expect([t.start(1), t.start(2)]).toEqual(['x1', undefined]);
  });
});
