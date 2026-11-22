import { describe, expect, it } from 'vitest';
import { spring, step } from './logo-spring';

const run = (s: ReturnType<typeof spring>, seconds: number, hz: number) => {
  // Whole frames, then whatever is left, so every rate covers exactly the same time.
  let left = seconds;
  while (left > 1e-12) {
    const d = Math.min(1 / hz, left);
    step(s, d);
    left -= d;
  }
  return s;
};

describe('3D logo spring', () => {
  it('settles on its target, critically damped without overshoot', () => {
    const s = spring(0, 0.4, 1);
    s.target = 1;
    let max = 0;
    for (let i = 0; i < 240; i++) max = Math.max(max, step(s, 1 / 60));
    expect(s.value).toBe(1);
    expect(s.velocity).toBe(0);
    expect(max).toBeLessThanOrEqual(1 + 1e-9);
  });

  it('is most of the way there after one response, and overshoots only a hair when lightly damped', () => {
    const s = run(Object.assign(spring(0, 0.42, 0.9), { target: 1 }), 0.42, 120);
    expect(s.value).toBeGreaterThan(0.9);
    const t = Object.assign(spring(0, 0.42, 0.9), { target: 1 });
    let max = 0;
    for (let i = 0; i < 600; i++) max = Math.max(max, step(t, 1 / 120));
    expect(max).toBeGreaterThan(1);
    expect(max).toBeLessThan(1.01);
  });

  it('lands in the same place at any frame rate', () => {
    for (const damping of [1, 0.9]) {
      const at = (hz: number) => run(Object.assign(spring(0, 0.4, damping), { target: 1, velocity: 3 }), 0.3, hz);
      const a = at(30);
      const b = at(60);
      const c = at(144);
      expect(b.value).toBeCloseTo(a.value, 6);
      expect(c.value).toBeCloseTo(a.value, 6);
      expect(b.velocity).toBeCloseTo(a.velocity, 6);
    }
  });

  it('keeps its velocity when the target moves mid-flight (no jump), and never goes NaN', () => {
    const s = Object.assign(spring(0, 0.4, 1), { target: 1 });
    run(s, 0.1, 60);
    const before = { value: s.value, velocity: s.velocity };
    s.target = -1;
    step(s, 1 / 60);
    expect(Math.abs(s.value - before.value)).toBeLessThan(Math.abs(before.velocity) / 60 + 0.05);
    for (const dt of [0, -1, NaN, Infinity, 10]) {
      step(s, dt);
      expect(Number.isFinite(s.value)).toBe(true);
      expect(Number.isFinite(s.velocity)).toBe(true);
    }
    const odd = spring(0, 0, 0);
    odd.target = 1;
    step(odd, 1 / 60);
    expect(Number.isFinite(odd.value)).toBe(true);
  });
});
