import { describe, expect, it } from 'vitest';
import { BARS } from './logo';
import { STRIKE, strike } from './logo-motion';
import { TUBE, ends, halfSize, look, loopPath, segmentsFor, tubePath } from './logo-tube';

describe('3D logo tubes', () => {
  it('bends each bar into one tube along the middle of its wall, inside its outline', () => {
    for (const b of BARS) {
      const pts = tubePath(b).getSpacedPoints(400);
      for (const p of pts) {
        expect(Math.abs(p.x) + TUBE.radius).toBeLessThanOrEqual(b.w / 2 + 1e-9);
        expect(Math.abs(p.y) + TUBE.radius).toBeLessThanOrEqual(b.h / 2 + 1e-9);
        expect(p.z).toBeLessThanOrEqual(1e-9); // nothing comes forward of the sign's plane
      }
      // Its four sides reach the bar's middle line: the outline reads as the bar.
      const { hw, hh } = halfSize(b);
      expect(Math.max(...pts.map((p) => p.x))).toBeCloseTo(hw, 6);
      expect(Math.min(...pts.map((p) => p.x))).toBeCloseTo(-hw, 6);
      expect(Math.max(...pts.map((p) => p.y))).toBeCloseTo(hh, 6);
    }
  });

  it('turns both ends back into the electrodes behind the bottom edge, apart from each other', () => {
    for (const b of BARS) {
      const path = tubePath(b);
      const start = path.getPoint(0);
      const end = path.getPoint(1);
      const { hh } = halfSize(b);
      const e = ends(b);
      expect(start.toArray()).toEqual([e.a, -hh, -TUBE.back]);
      expect(end.toArray()).toEqual([e.b, -hh, -TUBE.back]);
      // The two glass ends (and their caps) never touch.
      expect(e.a - e.b).toBeGreaterThan(TUBE.cap * 2);
      expect(e.a - e.b).toBeGreaterThan(TUBE.radius * 2);
    }
  });

  it('keeps every bend rounder than the glass, so the tube never folds into itself', () => {
    expect(TUBE.corner).toBeGreaterThan(TUBE.radius);
    expect(TUBE.bend).toBeLessThanOrEqual(TUBE.back);
    expect(TUBE.core).toBeLessThan(TUBE.radius);
    expect(TUBE.coreOff).toBeLessThan(TUBE.radius);
  });

  it('gives the glow a closed loop around the lit bar, and enough segments for round bends', () => {
    const lit = BARS.find((b) => b.lit)!;
    const loop = loopPath(lit);
    expect(loop.getPoint(0).distanceTo(loop.getPoint(1))).toBeLessThan(1e-9);
    expect(segmentsFor(tubePath(lit))).toBeGreaterThanOrEqual(Math.ceil(tubePath(lit).getLength() * 5));
  });
});

describe('3D logo look per theme', () => {
  it('lets the light carry dark and the glass edges carry light', () => {
    const dark = look(false);
    const light = look(true);
    expect(dark.additive).toBe(true);
    expect(light.additive).toBe(false);
    expect(light.glow).toBeLessThan(dark.glow);
    expect(light.edgeDark).toBe(true);
    expect(light.edgeAlpha).toBeGreaterThan(dark.edgeAlpha);
  });

  it('keeps the glass mostly clear and the unlit tubes dimmer than the lit one', () => {
    for (const l of [look(false), look(true)]) {
      expect(l.glassOpacity).toBeLessThan(0.15);
      expect(l.edgeAlpha).toBeLessThanOrEqual(1);
      expect(l.inkGlow).toBeLessThan(l.coreLevel * 0.1);
      expect(l.bloom).toBeLessThan(l.glow);
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
    const dimmed = samples.filter((k) => k < 0.99).length / 1000;
    expect(dimmed).toBeLessThan(0.45);
  });
});
