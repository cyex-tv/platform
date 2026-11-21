// A spring in Apple's terms (response and damping ratio, not stiffness and mass), stepped by its
// exact solution, so it lands in the same place whatever the frame rate, starts from wherever the
// value is now and keeps its velocity when the target moves (interruptible, never a jump).

export type Spring = {
  value: number;
  velocity: number;
  target: number;
  /** Seconds to (most of the way to) the target: lower is snappier. */
  response: number;
  /** 1 settles without overshoot; below 1 overshoots a little. */
  damping: number;
};

export function spring(value: number, response: number, damping: number): Spring {
  return { value, velocity: 0, target: value, response, damping };
}

/** Moves `s` on by `dt` seconds (in place) and returns its value. */
export function step(s: Spring, dt: number): number {
  if (!(dt > 0) || !Number.isFinite(dt)) return s.value;
  const w = (2 * Math.PI) / Math.max(1e-3, s.response);
  const z = Math.min(1, Math.max(0.05, s.damping));
  const x0 = s.value - s.target;
  const v0 = s.velocity;
  let x: number;
  let v: number;
  if (z >= 1 - 1e-6) {
    // Critically damped.
    const e = Math.exp(-w * dt);
    x = (x0 + (v0 + w * x0) * dt) * e;
    v = (v0 - w * (v0 + w * x0) * dt) * e;
  } else {
    const wd = w * Math.sqrt(1 - z * z);
    const e = Math.exp(-z * w * dt);
    const c = Math.cos(wd * dt);
    const sn = Math.sin(wd * dt);
    x = e * (x0 * c + ((v0 + z * w * x0) / wd) * sn);
    v = e * (v0 * c - ((z * w * v0 + w * w * x0) / wd) * sn);
  }
  // Close enough and slow enough: rest exactly, so it never drifts by a hair forever.
  if (Math.abs(x) < 1e-6 && Math.abs(v) < 1e-5) {
    x = 0;
    v = 0;
  }
  s.value = s.target + x;
  s.velocity = v;
  return s.value;
}
