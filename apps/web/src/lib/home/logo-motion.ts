// The 3D logo's motion as a pure function of time and scroll, so it is tested without WebGL.
//
// 0 → HOLD: the still pose (the static image's), while the canvas fades in over it.
// HOLD → HOLD + TURN: one full turn, eased in and out, ending in the still pose again.
// After that: a sway, a float and the lit bar breathing in and out, grown in over a few seconds
// so nothing jumps. Scrolling the hero away (0 → 1) turns it further and brings the lit bar forward.

export type Pose = { yaw: number; pitch: number; float: number; lift: number };

/** The still pose: turned to show the left sides and tops, the lit bar well in front. */
export const REST: Pose = { yaw: 0.5, pitch: 0.26, float: 0, lift: 2.4 };

export const HOLD = 0.7;
export const TURN = 3.2;
const GROW = 2.5;
const TAU = Math.PI * 2;

const inOut = (k: number) => (k < 0.5 ? 4 * k * k * k : 1 - (-2 * k + 2) ** 3 / 2);
const smooth = (k: number) => k * k * (3 - 2 * k);
const clamp01 = (k: number) => Math.min(1, Math.max(0, k));

/** Where the logo is `t` seconds after it started, with the hero scrolled `scroll` (0-1) away. */
export function pose(t: number, scroll = 0): Pose {
  const turn = inOut(clamp01((t - HOLD) / TURN)) * TAU;
  const s = Math.max(0, t - HOLD - TURN);
  const a = smooth(clamp01(s / GROW));
  const e = clamp01(scroll);
  return {
    yaw: REST.yaw + turn + a * 0.42 * Math.sin((TAU * s) / 12) + e * 0.8,
    pitch: REST.pitch + a * 0.12 * Math.sin((TAU * s) / 17) + e * 0.45,
    float: a * 0.45 * Math.sin((TAU * s) / 7.5),
    lift: REST.lift + a * 0.35 * Math.sin((TAU * s) / 5.5) + e * 2.4,
  };
}

/**
 * The lit tube striking when the scene starts: two short dips and a quick settle, once, while the
 * still image hands over. 1 is fully lit. It never goes dark and never repeats; with reduced motion
 * the scene doesn't run at all, and a still scene stays at 1.
 */
export const STRIKE: readonly (readonly [number, number])[] = [
  [0, 1],
  [0.55, 1],
  [0.56, 0.45],
  [0.61, 0.45],
  [0.62, 1],
  [0.69, 1],
  [0.7, 0.6],
  [0.74, 0.75],
  [0.92, 1],
];

export function strike(t: number): number {
  for (let i = 1; i < STRIKE.length; i++) {
    const [t1, v1] = STRIKE[i];
    if (t <= t1) {
      const [t0, v0] = STRIKE[i - 1];
      return v0 + ((v1 - v0) * (t - t0)) / (t1 - t0 || 1);
    }
  }
  return 1;
}
