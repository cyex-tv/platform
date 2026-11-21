// The hero logo leaning toward the pointer: where it leans (pure, so it's tested), and who gets it.
// Only a mouse or pen on a device that hovers; never touch, never with reduced motion.

/** The most it leans, each way, on each axis (radians: 7°). */
export const TILT_MAX = (7 * Math.PI) / 180;
/** How far outside the hero the pointer still counts, as a share of its half size; past it, rest. */
export const TILT_REACH = 1.35;

export type Rect = { left: number; top: number; width: number; height: number };

/**
 * How far to lean for a pointer at (x, y) over the hero's box: `yaw` turns its face toward the
 * pointer's side, `pitch` toward its height. Zero when the pointer is gone or far from the hero.
 */
export function tiltFor(pointer: { x: number; y: number } | null, r: Rect): { pitch: number; yaw: number } {
  if (!pointer || r.width <= 0 || r.height <= 0) return { pitch: 0, yaw: 0 };
  const nx = (pointer.x - (r.left + r.width / 2)) / (r.width / 2);
  const ny = (pointer.y - (r.top + r.height / 2)) / (r.height / 2);
  if (!Number.isFinite(nx) || !Number.isFinite(ny) || Math.abs(nx) > TILT_REACH || Math.abs(ny) > TILT_REACH) return { pitch: 0, yaw: 0 };
  const c = (k: number) => Math.max(-1, Math.min(1, k));
  return { pitch: c(ny) * TILT_MAX, yaw: c(nx) * TILT_MAX };
}

export type TiltEnv = { hover: boolean; fine: boolean; reducedMotion: boolean };

/** Whether the logo leans at all: a hovering, fine pointer, and motion allowed. */
export const tiltAllowed = (e: TiltEnv) => e.hover && e.fine && !e.reducedMotion;
