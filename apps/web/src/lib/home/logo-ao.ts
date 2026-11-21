// Ambient occlusion for the unlit bars, worked out from the shape instead of a screen-space pass:
// a frame's only concave places are inside its opening, where the inner walls meet (the four
// inside corners) and deep between its front and back edges. Light from the room reaches those
// less, so they darken softly; the outer faces stay open. The same sum runs in the shader (GLSL
// below) and here, where it is tested.

export const AO = {
  /** How dark an inside corner gets, and how far (grid units) the darkening reaches from it. */
  corner: 0.6,
  reach: 0.9,
  /** How much darker the middle of the opening's walls is than their front and back edges. */
  recess: 0.32,
};

/**
 * Occlusion (1 open, toward 0 occluded) at a point on a bar, in the bar's own space: `hole` is the
 * opening's half size, `halfDepth` half the bar's depth.
 */
export function cavity(x: number, y: number, z: number, hole: { x: number; y: number }, halfDepth: number): number {
  const px = Math.abs(x);
  const py = Math.abs(y);
  const inner = px <= hole.x + 0.05 && py <= hole.y + 0.05 ? 1 : 0;
  const corner = Math.hypot(px - hole.x, py - hole.y);
  const middle = 1 - Math.min(1, Math.abs(z) / halfDepth);
  return (1 - inner * AO.corner * Math.exp(-corner / AO.reach)) * (1 - inner * AO.recess * middle);
}

const f = (n: number) => n.toFixed(4);

/** The same, in GLSL: `heroCavity(vec3 p)`, with uniforms `uHole` (vec2) and `uHalfDepth`. */
export const cavityGlsl = /* glsl */ `
  uniform vec2 uHole;
  uniform float uHalfDepth;
  float heroCavity(vec3 p) {
    vec2 a = abs(p.xy);
    float inner = step(a.x, uHole.x + 0.05) * step(a.y, uHole.y + 0.05);
    float corner = length(a - uHole);
    float middle = 1.0 - min(1.0, abs(p.z) / uHalfDepth);
    return (1.0 - inner * ${f(AO.corner)} * exp(-corner / ${f(AO.reach)})) * (1.0 - inner * ${f(AO.recess)} * middle);
  }
`;
