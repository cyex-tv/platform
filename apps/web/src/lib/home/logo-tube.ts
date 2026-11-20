// The hero logo as light tubes: the shape of each tube and the look per theme, kept apart from
// the WebGL scene so both are tested without a GPU.
//
// Each open bar of logo.ts becomes one glass tube bent into a rounded rectangle along the middle of
// the bar's wall, the way a neon sign is bent. Its two ends meet on the bottom edge, near the left
// corner, and turn back into metal electrode caps behind it, as a real tube does.
import { CurvePath, LineCurve3, QuadraticBezierCurve3, Vector3 } from 'three';
import { STROKE, type Bar } from './logo';

export const TUBE = {
  /** Outer radius of the glass, grid units: the tube is about as thick as the mark's wall. */
  radius: STROKE * 0.48,
  /** The glowing core inside (lit) and the dim one (unlit). */
  core: STROKE * 0.2,
  coreOff: STROKE * 0.17,
  /** How round the bent corners are (more than the radius, so the tube never folds into itself). */
  corner: STROKE * 0.75,
  /** The ends' bend back, and how far behind the tube the electrodes sit. */
  bend: STROKE * 0.6,
  back: STROKE * 1.3,
  /** Centre-to-centre distance of the two ends, and where they meet, from the left corner's start. */
  gap: STROKE * 1.35,
  gapAt: STROKE * 2.2,
  /** The electrode caps: a little wider than the glass. */
  cap: STROKE * 0.54,
  capLength: STROKE * 0.6,
};

const v = (x: number, y: number, z = 0) => new Vector3(x, y, z);

/** Half the size of the tube's middle line: the bar's outline less half its wall. */
export function halfSize(b: Bar): { hw: number; hh: number } {
  return { hw: b.w / 2 - STROKE / 2, hh: b.h / 2 - STROKE / 2 };
}

/** Where the two ends sit (x, centred on the bar), on the bottom edge. */
export function ends(b: Bar): { a: number; b: number } {
  const { hw } = halfSize(b);
  const mid = -hw + TUBE.corner + TUBE.gapAt;
  return { a: mid + TUBE.gap / 2, b: mid - TUBE.gap / 2 };
}

/**
 * The tube's middle line for one bar, centred on the bar: from one electrode (behind the bottom
 * edge), forward and round the rectangle anticlockwise, back into the other electrode.
 */
export function tubePath(bar: Bar): CurvePath<Vector3> {
  const { hw, hh } = halfSize(bar);
  const { corner: r, bend, back } = TUBE;
  const { a, b } = ends(bar);
  const p = new CurvePath<Vector3>();
  const line = (from: Vector3, to: Vector3) => p.add(new LineCurve3(from, to));
  const turn = (from: Vector3, via: Vector3, to: Vector3) => p.add(new QuadraticBezierCurve3(from, via, to));
  // Out of the first electrode and forward into the sign's plane.
  line(v(a, -hh, -back), v(a, -hh, -bend));
  turn(v(a, -hh, -bend), v(a, -hh), v(a + bend, -hh));
  // Round the rectangle.
  line(v(a + bend, -hh), v(hw - r, -hh));
  turn(v(hw - r, -hh), v(hw, -hh), v(hw, -hh + r));
  line(v(hw, -hh + r), v(hw, hh - r));
  turn(v(hw, hh - r), v(hw, hh), v(hw - r, hh));
  line(v(hw - r, hh), v(-hw + r, hh));
  turn(v(-hw + r, hh), v(-hw, hh), v(-hw, hh - r));
  line(v(-hw, hh - r), v(-hw, -hh + r));
  turn(v(-hw, -hh + r), v(-hw, -hh), v(-hw + r, -hh));
  line(v(-hw + r, -hh), v(b - bend, -hh));
  // And back into the second electrode.
  turn(v(b - bend, -hh), v(b, -hh), v(b, -hh, -bend));
  line(v(b, -hh, -bend), v(b, -hh, -back));
  return p;
}

/**
 * The same rectangle closed, without the ends: what the glow around the lit tube follows (the
 * light comes from the part that faces out; a halo bent through the ends' tight turns would fold).
 */
export function loopPath(bar: Bar): CurvePath<Vector3> {
  const { hw, hh } = halfSize(bar);
  // Rounder than the glass, so a halo wider than the corner never folds into itself there.
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

/** Segments along a tube: enough that the bends stay round (about five per grid unit). */
export const segmentsFor = (path: CurvePath<Vector3>) => Math.ceil(path.getLength() * 5);

export type Look = {
  /** Glass: base opacity, how opaque and bright its edges get (fresnel), and its reflections. */
  glassOpacity: number;
  edgeAlpha: number;
  edgeLight: number;
  edgePower: number;
  reflect: number;
  /** Glass edges darken on white (light) or catch light (dark). */
  edgeDark: boolean;
  /** The lit core's brightness and how white its middle runs. */
  coreLevel: number;
  coreHot: number;
  /** The glow around the lit tube: a tight halo and a wide bloom. */
  glow: number;
  bloom: number;
  /** The wide, faint light in the air around it. */
  air: number;
  /** On dark the glow adds light; on white it lays colour down. */
  additive: boolean;
  /** The lit tube's light falling on the other two. */
  spill: number;
  /** The unlit tubes' dim core: how much of the mark's ink colour it shows, and its own faint light. */
  inkCore: number;
  inkGlow: number;
  ambient: number;
  key: number;
};

/** The look for each theme: dark lets the light carry it, light lets the glass edges do. */
export function look(light: boolean): Look {
  return light
    ? { glassOpacity: 0.08, edgeAlpha: 0.75, edgeLight: 0, edgePower: 2.2, reflect: 1.4, edgeDark: true, coreLevel: 1, coreHot: 0.25, glow: 0.38, bloom: 0.1, air: 0.12, additive: false, spill: 45, inkCore: 1, inkGlow: 0, ambient: 0.6, key: 2.2 }
    : { glassOpacity: 0.05, edgeAlpha: 0.6, edgeLight: 0.55, edgePower: 2.6, reflect: 1.1, edgeDark: false, coreLevel: 1.25, coreHot: 0.6, glow: 0.75, bloom: 0.28, air: 0.22, additive: true, spill: 170, inkCore: 0.2, inkGlow: 0.03, ambient: 0.3, key: 1.6 };
}
