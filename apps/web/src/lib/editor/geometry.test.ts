import { describe, expect, it } from 'vitest';
import {
  FLIP_H,
  IDENTITY,
  ROT_CCW,
  ROT_CW,
  apply,
  clampRect,
  compose,
  cropToRatio,
  dragCrop,
  fitScale,
  fitView,
  placed,
  scaled,
  segmentDistance,
  turnedSize,
  zoomAt,
} from './geometry';

describe('orientation', () => {
  it('turns clockwise: right goes to the bottom, the picture stays at the origin', () => {
    const t = placed(ROT_CW, 40, 10);
    expect(turnedSize(ROT_CW, 40, 10)).toEqual({ w: 10, h: 40 });
    // the source's top right corner (40, 0) lands at the canvas's bottom right (10, 40)
    expect(apply(t, 40, 0)).toEqual([10, 40]);
    expect(apply(t, 0, 0)).toEqual([10, 0]);
    expect(apply(t, 0, 10)).toEqual([0, 0]);
  });

  it('four quarter turns and two flips are the identity', () => {
    let m = IDENTITY;
    for (let i = 0; i < 4; i++) m = compose(ROT_CW, m);
    expect(m).toEqual(IDENTITY);
    expect(compose(FLIP_H, FLIP_H)).toEqual(IDENTITY);
    expect(compose(ROT_CCW, ROT_CW)).toEqual(IDENTITY);
  });

  it('mirrors along the width', () => {
    expect(apply(placed(FLIP_H, 30, 20), 0, 5)).toEqual([30, 5]);
  });
});

describe('crop', () => {
  const start = { x: 10, y: 10, w: 40, h: 20 };

  it('drags an edge and keeps the opposite one', () => {
    expect(dragCrop(start, 'e', 15, 99, 100, 100)).toEqual({ x: 10, y: 10, w: 55, h: 20 });
    expect(dragCrop(start, 'nw', -5, -5, 100, 100)).toEqual({ x: 5, y: 5, w: 45, h: 25 });
  });

  it('never goes past the canvas or under 1 px', () => {
    expect(dragCrop(start, 'w', -50, 0, 100, 100)).toEqual({ x: 0, y: 10, w: 50, h: 20 });
    expect(dragCrop(start, 'e', -500, 0, 100, 100).w).toBe(1);
    expect(dragCrop(start, 'move', 500, 500, 100, 100)).toEqual({ x: 60, y: 80, w: 40, h: 20 });
  });

  it('keeps a locked ratio from a corner and from an edge', () => {
    const r = dragCrop(start, 'se', 20, 0, 200, 200, 2);
    expect(r.w / r.h).toBeCloseTo(2, 1);
    expect(r).toMatchObject({ x: 10, y: 10, w: 60, h: 30 });
    const e = dragCrop({ x: 50, y: 50, w: 40, h: 40 }, 'e', 40, 0, 200, 200, 1);
    expect(e).toEqual({ x: 50, y: 30, w: 80, h: 80 });
  });

  it('shrinks a locked box to fit instead of breaking the ratio', () => {
    const r = dragCrop({ x: 0, y: 0, w: 20, h: 10 }, 'se', 500, 0, 100, 40, 2);
    expect(r).toEqual({ x: 0, y: 0, w: 80, h: 40 });
  });

  it('cuts the largest centred box of a ratio', () => {
    expect(cropToRatio(1000, 100, 4)).toEqual({ x: 300, y: 0, w: 400, h: 100 });
    expect(cropToRatio(100, 100, 2)).toEqual({ x: 0, y: 25, w: 100, h: 50 });
  });

  it('clamps to whole pixels inside the canvas', () => {
    expect(clampRect({ x: -3.4, y: 2.6, w: 500, h: 4.2 }, 100, 50)).toEqual({ x: 0, y: 3, w: 100, h: 4 });
  });
});

describe('size and view', () => {
  it('fits the longer side', () => {
    expect(fitScale(2000, 500, 1000)).toBe(0.5);
    expect(fitScale(200, 50, 1000)).toBe(1);
    expect(scaled({ w: 333, h: 111 }, 0.5, 0.5)).toEqual({ w: 167, h: 56 });
    expect(scaled({ w: 1, h: 1 }, 0.01, 0.01)).toEqual({ w: 1, h: 1 });
  });

  it('fits a picture in the stage and zooms around a point', () => {
    const v = fitView(100, 50, 448, 448, 24, 8);
    expect(v.zoom).toBe(4);
    expect(v.x).toBe(24);
    const z = zoomAt({ zoom: 1, x: 0, y: 0 }, 2, 100, 100);
    expect(z).toEqual({ zoom: 2, x: -100, y: -100 });
  });

  it('measures the distance to a segment', () => {
    expect(segmentDistance(5, 5, 0, 0, 10, 0)).toBe(5);
    expect(segmentDistance(-3, 4, 0, 0, 10, 0)).toBe(5);
  });
});

describe('wheel zoom and stretching', () => {
  it('zooms in on wheel up and out on wheel down, inside the range', async () => {
    const { MAX_ZOOM, minZoom, wheelZoom } = await import('./geometry');
    expect(wheelZoom(1, -100, 0, false, 0.1)).toBeGreaterThan(1.1);
    expect(wheelZoom(1, 100, 0, false, 0.1)).toBeLessThan(0.9);
    // Lines (Firefox) count like pixels, a pinch (Ctrl + wheel) follows small deltas.
    expect(wheelZoom(1, -3, 1, false, 0.1)).toBeCloseTo(wheelZoom(1, -48, 0, false, 0.1));
    expect(wheelZoom(1, -10, 0, true, 0.1)).toBeCloseTo(Math.exp(0.1));
    // One event at most doubles or halves, and the ends hold.
    expect(wheelZoom(1, -100_000, 0, false, 0.1)).toBe(2);
    expect(wheelZoom(30, -1000, 0, false, 0.1)).toBe(MAX_ZOOM);
    expect(wheelZoom(0.2, 10_000, 0, false, 0.1)).toBe(0.1);
    expect(minZoom(8)).toBe(0.25);
    expect(minZoom(0.4)).toBeCloseTo(0.1);
  });

  it('stretches freely by default: an edge changes one side, a corner both', async () => {
    const { stretchBy } = await import('./geometry');
    expect(stretchBy('e', 100, 100, 150, 0, false)).toEqual({ w: 250, h: 100 });
    expect(stretchBy('s', 100, 100, 0, 200, false)).toEqual({ w: 100, h: 300 });
    expect(stretchBy('nw', 100, 100, 30, -40, false)).toEqual({ w: 70, h: 140 });
    expect(stretchBy('w', 100, 100, 500, 0, false)).toEqual({ w: 1, h: 100 });
  });

  it('keeps the shape when locked', async () => {
    const { stretchBy } = await import('./geometry');
    expect(stretchBy('e', 100, 50, 100, 0, true)).toEqual({ w: 200, h: 100 });
    expect(stretchBy('se', 100, 50, 10, 50, true)).toEqual({ w: 200, h: 100 });
  });
});
