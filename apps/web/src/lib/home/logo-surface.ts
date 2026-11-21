// The unlit bars' surface, made in code (no downloaded textures): a fine, tileable grain that
// varies the roughness a little and tilts the normal a little, so reflections break up the way
// they do on a real moulded or anodised part instead of sliding across a perfect mirror.

/** A deterministic hash in 0-1 for a lattice point (wrapped to `period`, so the noise tiles). */
function lattice(x: number, y: number, period: number, seed: number): number {
  const xi = ((x % period) + period) % period;
  const yi = ((y % period) + period) % period;
  let h = (xi * 374761393 + yi * 668265263 + seed * 2147483647) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
}

const fade = (t: number) => t * t * (3 - 2 * t);

/** Smooth value noise at (x, y) on a lattice of `period` cells, tiling every `period`. */
function noise(x: number, y: number, period: number, seed: number): number {
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  const fx = fade(x - x0);
  const fy = fade(y - y0);
  const a = lattice(x0, y0, period, seed);
  const b = lattice(x0 + 1, y0, period, seed);
  const c = lattice(x0, y0 + 1, period, seed);
  const d = lattice(x0 + 1, y0 + 1, period, seed);
  return a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy;
}

/** A tileable height field, `n` × `n`, values 0-1: a few octaves of value noise. */
export function grain(n: number, seed = 7): Float32Array {
  const out = new Float32Array(n * n);
  const octaves = [
    [8, 0.5],
    [16, 0.3],
    [32, 0.2],
  ] as const;
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++) {
      let v = 0;
      for (const [cells, w] of octaves) v += w * noise((x / n) * cells, (y / n) * cells, cells, seed + cells);
      out[y * n + x] = v;
    }
  return out;
}

/** RGBA bytes of a tangent-space normal map from a height field (wrapping at the edges). */
export function normalBytes(h: Float32Array, n: number, strength = 2.5): Uint8Array {
  const out = new Uint8Array(n * n * 4);
  const at = (x: number, y: number) => h[((y + n) % n) * n + ((x + n) % n)];
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++) {
      const dx = (at(x + 1, y) - at(x - 1, y)) * strength;
      const dy = (at(x, y + 1) - at(x, y - 1)) * strength;
      const len = Math.hypot(dx, dy, 1);
      const i = (y * n + x) * 4;
      out[i] = Math.round(((-dx / len) * 0.5 + 0.5) * 255);
      out[i + 1] = Math.round(((-dy / len) * 0.5 + 0.5) * 255);
      out[i + 2] = Math.round(((1 / len) * 0.5 + 0.5) * 255);
      out[i + 3] = 255;
    }
  return out;
}

/** RGBA bytes of a roughness map (three reads green): `base` give or take `spread`. */
export function roughnessBytes(h: Float32Array, n: number, base: number, spread: number): Uint8Array {
  const out = new Uint8Array(n * n * 4);
  for (let i = 0; i < n * n; i++) {
    const v = Math.min(1, Math.max(0, base + (h[i] - 0.5) * 2 * spread));
    out[i * 4] = out[i * 4 + 1] = out[i * 4 + 2] = Math.round(v * 255);
    out[i * 4 + 3] = 255;
  }
  return out;
}
