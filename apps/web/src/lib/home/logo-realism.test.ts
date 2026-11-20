import { describe, expect, it } from 'vitest';
import { AO, cavity } from './logo-ao';
import { frameBudget, startQuality } from './logo-quality';
import { grain, normalBytes, roughnessBytes } from './logo-surface';

describe('3D logo cavity occlusion', () => {
  const hole = { x: 6, y: 1.7 };
  const half = 2.1;

  it('leaves the outer faces open', () => {
    expect(cavity(7.4, 0, 0, hole, half)).toBe(1); // outer side
    expect(cavity(3, 2.9, 0, hole, half)).toBe(1); // top
  });

  it('darkens the opening’s inside corners most, its walls’ middle more than their edges', () => {
    const corner = cavity(6, 1.7, 0, hole, half);
    const wall = cavity(0, 1.7, 0, hole, half);
    const wallAtFront = cavity(0, 1.7, half, hole, half);
    expect(corner).toBeLessThan(wall);
    expect(wall).toBeLessThan(wallAtFront);
    expect(wallAtFront).toBeCloseTo(1, 2);
    expect(corner).toBeGreaterThan((1 - AO.corner) * (1 - AO.recess) - 1e-9); // never black
  });
});

describe('3D logo surface grain', () => {
  const n = 32;
  const h = grain(n);

  it('is deterministic, in range, and not flat', () => {
    expect(grain(n)).toEqual(h);
    expect(Math.min(...h)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...h)).toBeLessThanOrEqual(1);
    expect(Math.max(...h) - Math.min(...h)).toBeGreaterThan(0.2);
  });

  it('tiles: the last column runs on into the first', () => {
    let seam = 0;
    let inside = 0;
    for (let y = 0; y < n; y++) {
      seam += Math.abs(h[y * n + n - 1] - h[y * n]);
      inside += Math.abs(h[y * n + 1] - h[y * n]);
    }
    expect(seam).toBeLessThan(inside * 2.5);
  });

  it('makes normals that lean only a little, and roughness within its spread', () => {
    const nb = normalBytes(h, n);
    for (let i = 0; i < n * n; i++) expect(nb[i * 4 + 2]).toBeGreaterThan(200); // mostly facing out
    const rb = roughnessBytes(h, n, 0.5, 0.1);
    for (let i = 0; i < n * n; i++) {
      expect(rb[i * 4 + 1]).toBeGreaterThanOrEqual(Math.floor(0.4 * 255));
      expect(rb[i * 4 + 1]).toBeLessThanOrEqual(Math.ceil(0.6 * 255));
    }
  });
});

describe('3D logo quality', () => {
  it('starts low-power devices, and anyone who asks, on the light step', () => {
    expect(startQuality({ memory: 8, cores: 8 })).toBe('high');
    expect(startQuality({ memory: undefined, cores: undefined })).toBe('high');
    expect(startQuality({ memory: 4, cores: 8 })).toBe('low');
    expect(startQuality({ memory: 8, cores: 4 })).toBe('low');
    expect(startQuality({ memory: 16, cores: 16, forceLow: true })).toBe('low');
  });

  it('steps down once frames run long, after the warm-up, and only once', () => {
    const b = frameBudget({ limit: 24, window: 10, warmup: 5 });
    for (let i = 0; i < 5; i++) expect(b.add(100)).toBe(false); // warm-up ignored
    for (let i = 0; i < 30; i++) expect(b.add(16.7)).toBe(false);
    let tripped = 0;
    for (let i = 0; i < 30; i++) tripped += b.add(40) ? 1 : 0;
    expect(tripped).toBe(1);
  });

  it('holds steady at 60 fps with the odd long frame', () => {
    const b = frameBudget({ limit: 24, window: 60, warmup: 0 });
    let tripped = false;
    for (let i = 0; i < 600; i++) tripped ||= b.add(i % 30 === 0 ? 50 : 16.7);
    expect(tripped).toBe(false);
  });
});
