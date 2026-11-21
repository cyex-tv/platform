import { describe, expect, it } from 'vitest';
import { BARS, STROKE } from './logo';
import { STRIKE, strike } from './logo-motion';
import { halfSize, look, loopPath, segmentsFor, shellLimit } from './logo-glow';

describe('3D logo glow', () => {
  const lit = BARS.find((b) => b.lit)!;

  it('follows the middle of the lit bar’s wall, closed, inside its outline', () => {
    const loop = loopPath(lit);
    expect(loop.getPoint(0).distanceTo(loop.getPoint(1))).toBeLessThan(1e-9);
    const { hw, hh } = halfSize(lit);
    const pts = loop.getSpacedPoints(400);
    expect(Math.max(...pts.map((p) => p.x))).toBeCloseTo(hw, 6);
    expect(Math.max(...pts.map((p) => p.y))).toBeCloseTo(hh, 6);
    for (const p of pts) expect(p.z).toBe(0);
  });

  it('leaves room for a glow wider than the rod, and enough segments for round corners', () => {
    // The rod's half thickness across its wall is STROKE / 2: the shell must reach past it.
    expect(shellLimit(lit)).toBeGreaterThan(STROKE);
    const loop = loopPath(lit);
    expect(segmentsFor(loop)).toBeGreaterThanOrEqual(Math.ceil(loop.getLength() * 5));
  });
});

describe('3D logo look per theme', () => {
  it('adds light on dark and lays colour down on white, softer there', () => {
    const dark = look(false);
    const light = look(true);
    expect(dark.additive).toBe(true);
    expect(light.additive).toBe(false);
    expect(light.glow).toBeLessThan(dark.glow);
    expect(light.hot).toBeLessThan(dark.hot); // a white core would vanish on white
  });

  it('keeps one bar lit: the others never glow near the lit one', () => {
    for (const l of [look(false), look(true)]) {
      expect(l.inkGlow).toBeLessThan(l.level * 0.1);
      expect(l.air).toBeLessThan(l.glow);
      expect(l.coreWidth).toBeGreaterThan(0);
      expect(l.coreWidth).toBeLessThan(1);
    }
  });
});

describe('3D logo strike', () => {
  it('is fully lit before, after, and at the still image', () => {
    expect(strike(0)).toBe(1);
    expect(strike(-1)).toBe(1);
    expect(strike(STRIKE[STRIKE.length - 1][0])).toBe(1);
    for (let t = 1; t < 60; t += 0.5) expect(strike(t)).toBe(1); // once, never looping
  });

  it('dips briefly, never goes dark, and is over within a second', () => {
    const samples = Array.from({ length: 1000 }, (_, i) => strike(i / 1000));
    expect(Math.min(...samples)).toBeGreaterThanOrEqual(0.4);
    expect(Math.min(...samples)).toBeLessThan(0.8);
    expect(samples.filter((k) => k < 0.99).length / 1000).toBeLessThan(0.45);
  });
});

describe('3D logo unlit bars per theme', () => {
  const dark = look(false);
  const light = look(true);

  it('makes them a bright satin on dark that still shades (sides darker than caps, not flat white)', () => {
    expect(dark.inkCap).toBeGreaterThan(0.5); // lighter than the old 0.42
    expect(dark.inkCap).toBeLessThan(1);
    expect(dark.inkSide).toBeLessThan(dark.inkCap);
    expect(dark.inkReflect).toBeGreaterThan(light.inkReflect);
  });

  it('gives them a white glow on dark, weaker than the lit bar’s, and none on light', () => {
    expect(dark.inkBloom).toBeGreaterThan(0);
    expect(dark.inkBloom).toBeLessThan(dark.level * 0.5);
    expect(dark.inkHalo).toBeGreaterThan(0);
    expect(dark.inkHalo).toBeLessThan(dark.glow);
    expect(light.inkBloom).toBe(0);
    expect(light.inkHalo).toBe(0);
  });

  it('leaves the light theme’s dark bars as they were', () => {
    expect(light).toMatchObject({ inkCap: 1, inkSide: 1, inkGlow: 0, inkReflect: 0.8 });
  });
});
