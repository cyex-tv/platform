// The hero logo as neon: what the glow around the lit bar follows and the look per theme, kept
// apart from the WebGL scene so both are tested without a GPU.
import { CurvePath, LineCurve3, QuadraticBezierCurve3, Vector3 } from 'three';
import { STROKE, type Bar } from './logo';

const v = (x: number, y: number) => new Vector3(x, y, 0);

/** Half the size of a bar's wall's middle line: the bar's outline less half its wall. */
export function halfSize(b: Bar): { hw: number; hh: number } {
  return { hw: b.w / 2 - STROKE / 2, hh: b.h / 2 - STROKE / 2 };
}

/**
 * A closed loop along the middle of a bar's wall, centred on the bar, with corners round enough
 * (nearly half the bar's height) that a glow shell wider than the wall never folds into itself.
 */
export function loopPath(bar: Bar): CurvePath<Vector3> {
  const { hw, hh } = halfSize(bar);
  const r = hh * 0.92;
  const p = new CurvePath<Vector3>();
  const line = (from: Vector3, to: Vector3) => p.add(new LineCurve3(from, to));
  const turn = (from: Vector3, via: Vector3, to: Vector3) => p.add(new QuadraticBezierCurve3(from, via, to));
  line(v(0, -hh), v(hw - r, -hh));
  turn(v(hw - r, -hh), v(hw, -hh), v(hw, -hh + r));
  line(v(hw, -hh + r), v(hw, hh - r));
  turn(v(hw, hh - r), v(hw, hh), v(hw - r, hh));
  line(v(hw - r, hh), v(-hw + r, hh));
  turn(v(-hw + r, hh), v(-hw, hh), v(-hw, hh - r));
  line(v(-hw, hh - r), v(-hw, -hh + r));
  turn(v(-hw, -hh + r), v(-hw, -hh), v(-hw + r, -hh));
  line(v(-hw + r, -hh), v(0, -hh));
  return p;
}

/** The widest glow shell that still fits the loop's corners (radius as a share of the corner). */
export const shellLimit = (bar: Bar) => halfSize(bar).hh * 0.92;

/** Segments along a loop: enough that its bends stay round (about five per grid unit). */
export const segmentsFor = (path: CurvePath<Vector3>) => Math.ceil(path.getLength() * 5);

export type Look = {
  /** The lit rod: its brightness, how white its core runs, and how far toward the edges the core reaches (0-1). */
  level: number;
  hot: number;
  coreWidth: number;
  /** The glow around it: a halo on the rod and a wide, faint light in the air. */
  glow: number;
  air: number;
  /** On dark the glow adds light; on white it lays colour down. */
  additive: boolean;
  /** The lit rod's light falling on the other two. */
  spill: number;
  /** The unlit bars: their own faint light in the mark's ink colour. */
  inkGlow: number;
  /** The unlit bars' finish: how much of the mark's ink colour their caps and sides show (1 = all). */
  inkCap: number;
  inkSide: number;
  /** How strongly they reflect the studio room (their own, apart from the rest of the scene). */
  inkReflect: number;
  /** Their own white glow: how bright they are in the bloom's render (0 = black, no glow), and the light step's halo. */
  inkBloom: number;
  inkHalo: number;
};

/** The look for each theme: dark lets the light carry it; on white the rod's colour has to. */
export function look(light: boolean): Look {
  return light
    ? { level: 1, hot: 0.5, coreWidth: 0.45, glow: 0.4, air: 0.14, additive: false, spill: 30, inkGlow: 0, inkCap: 1, inkSide: 1, inkReflect: 0.8, inkBloom: 0, inkHalo: 0 }
    : // Dark: the unlit bars are a bright, near-white satin with a soft white glow of their own,
      // weaker than the lit bar's, which stays the one that's lit.
      { level: 1.15, hot: 0.6, coreWidth: 0.35, glow: 0.7, air: 0.24, additive: true, spill: 90, inkGlow: 0.03, inkCap: 0.6, inkSide: 0.45, inkReflect: 1.2, inkBloom: 0.25, inkHalo: 0.3 };
}
