import { describe, expect, it } from 'vitest';
import { boomerang, changeSpeed, delaysAtFps, delaysFromTimes, dropEvery, gifDelays, limitFps, moveFrames, shownDelay, total } from './timing';

const f = (...delays: number[]) => delays.map((delay, i) => ({ i, delay }));

describe('frame timing', () => {
  it('times a 30 fps clip so the loop lasts exactly as long, with no stall at the loop point', () => {
    // 30 frames of a 1 s clip at 33 333 µs each; the last frame keeps its own duration.
    const times = Array.from({ length: 30 }, (_, i) => Math.round((i * 1_000_000) / 30));
    const d = delaysFromTimes(times, 1_000_000);
    expect(d).toHaveLength(30);
    expect(d.reduce((a, b) => a + b)).toBe(1000);
    expect(Math.max(...d) - Math.min(...d)).toBeLessThanOrEqual(1);
    expect(d.at(-1)).toBeGreaterThanOrEqual(33);
  });

  it('sorts presentation times (B-frames arrive out of order)', () => {
    expect(delaysFromTimes([40_000, 0, 20_000], 60_000)).toEqual([20, 20, 20]);
  });

  it('spreads fps delays and ends at the duration', () => {
    expect(delaysAtFps(3, 30)).toEqual([33, 34, 33]);
    expect(delaysAtFps(3, 30, 90)).toEqual([33, 34, 23]);
  });

  it('shows short imported delays as browsers do', () => {
    expect(shownDelay(0, 5)).toBe(100);
    expect(shownDelay(10, 5)).toBe(100);
    expect(shownDelay(20, 5)).toBe(20);
    expect(shownDelay(70, 1)).toBe(0);
  });

  it('changes speed and keeps the proportion', () => {
    const s = changeSpeed(f(100, 50, 50), 2);
    expect(s.map((x) => x.delay)).toEqual([50, 25, 25]);
    expect(total(changeSpeed(f(33, 34, 33), 0.5))).toBe(200);
  });

  it('drops every 2nd frame and keeps the length', () => {
    const d = dropEvery(f(10, 20, 30, 40, 50));
    expect(d.map((x) => x.i)).toEqual([0, 2, 4]);
    expect(d.map((x) => x.delay)).toEqual([30, 70, 50]);
  });

  it('limits to 50 fps by sampling every 20 ms, same length', () => {
    const sixty = f(...Array.from({ length: 60 }, (_, i) => (i % 3 === 2 ? 16 : 17)));
    const l = limitFps(sixty, 20);
    expect(total(l)).toBe(total(sixty));
    expect(Math.min(...l.map((x) => x.delay))).toBeGreaterThanOrEqual(20);
    expect(l.length).toBe(50);
  });

  it('leaves an animation within the limit alone and merges a long frame once', () => {
    expect(limitFps(f(20, 30), 20).map((x) => x.delay)).toEqual([20, 30]);
    const l = limitFps(f(10, 10, 500), 20);
    expect(l.map((x) => [x.i, x.delay])).toEqual([[0, 20], [2, 500]]);
    expect(total(l)).toBe(520);
  });

  it('goes forward then back without showing an end twice', () => {
    const b = boomerang([0, 1, 2, 3], (x) => x + 10);
    expect(b).toEqual([0, 1, 2, 3, 12, 11]);
    expect(boomerang([0, 1], (x) => x)).toEqual([0, 1]);
  });

  it('rounds GIF delays to hundredths, at least 2', () => {
    expect(gifDelays([33, 34, 33])).toEqual([3, 4, 3]);
    expect(gifDelays([10, 10])).toEqual([2, 2]);
  });

  it('moves picked frames before a position', () => {
    expect(moveFrames(['a', 'b', 'c', 'd'], new Set([0]), 3)).toEqual(['b', 'c', 'a', 'd']);
    expect(moveFrames(['a', 'b', 'c', 'd'], new Set([2, 3]), 0)).toEqual(['c', 'd', 'a', 'b']);
    expect(moveFrames(['a', 'b', 'c', 'd'], new Set([1]), 4)).toEqual(['a', 'c', 'd', 'b']);
  });
});
