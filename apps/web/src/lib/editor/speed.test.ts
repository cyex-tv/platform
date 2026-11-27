// Playback speed (owner, 2026/11/27): 100% by default, 0% to 2000%. 0% uploads a still of the
// frame on show; high speeds merge frames so the upload keeps to 50 fps instead of failing it.
import { describe, expect, it } from 'vitest';
import { createDoc, exportDoc, exportFrames, frozen, resize, setSpeed, speedOf } from './model';
import { MAX_SPEED_PCT, gifDelays, shortest, speedFromSlider, speedToSlider, total } from './timing';

const anim = (delays: number[]) => createDoc(100, 100, delays);

describe('playback speed', () => {
  it('is 100% by default and the upload is the frames as made', () => {
    const d = anim([40, 40, 40]);
    expect(speedOf(d)).toBe(1);
    expect(exportFrames(d, 0, 20)).toBe(d.frames);
    expect(exportDoc(d, 0, 20)).toBe(d);
  });

  it('clamps to 0 to 2000%', () => {
    const d = anim([40, 40]);
    expect(speedOf(setSpeed(d, 50))).toBe(20);
    expect(speedOf(setSpeed(d, -1))).toBe(0);
    expect(setSpeed(d, 1)).toBe(d);
  });

  it('slows down: every delay longer, the same frames', () => {
    const f = exportFrames(setSpeed(anim([40, 60]), 0.5), 0, 20);
    expect(f.map((x) => x.delay)).toEqual([80, 120]);
  });

  it('at 0% freezes on the frame on show and uploads a still', () => {
    const d = setSpeed(anim([40, 40, 40, 40]), 0);
    expect(frozen(d)).toBe(true);
    const f = exportFrames(d, 2, 20);
    expect(f).toHaveLength(1);
    expect(f[0].id).toBe(d.frames[2].id);
    // A still image is never frozen: it has nothing to play.
    expect(frozen(setSpeed(anim([0]), 0))).toBe(false);
  });

  it('at high speeds merges frames so it never goes over 50 fps, at the right speed', () => {
    // 20 frames of 40 ms (800 ms a loop) at 2000%: 40 ms a loop, two frames of 20 ms.
    const d = setSpeed(anim(Array(20).fill(40)), 20);
    const f = exportFrames(d, 0, 20);
    expect(shortest(f)).toBeGreaterThanOrEqual(20);
    expect(total(f)).toBe(40);
    expect(f.length).toBeLessThan(20);
    // 400%: 10 ms frames would be 100 fps; sampled every 20 ms, every 2nd frame stays.
    const g = exportFrames(setSpeed(anim(Array(8).fill(40)), 4), 0, 20);
    expect(g.map((x) => x.delay)).toEqual([20, 20, 20, 20]);
    expect(g.map((x) => x.src)).toEqual([0, 2, 4, 6]);
    // GIF keeps 2 hundredths at least, so no browser shows those frames for 100 ms.
    expect(Math.min(...gifDelays(g.map((x) => x.delay)))).toBeGreaterThanOrEqual(2);
  });

  it('leaves an animation too fast at 100% to the frame-rate rule', () => {
    const f = exportFrames(setSpeed(anim([10, 10, 10, 10]), 2), 0, 20);
    expect(f.map((x) => x.delay)).toEqual([5, 5, 5, 5]);
  });

  it('the upload document keeps the stretch and the drawings, at speed 1', () => {
    const d = setSpeed(resize(anim(Array(8).fill(40)), 300, 100), 4);
    const out = exportDoc(d, 0, 20);
    expect(out.speed).toBe(1);
    expect([out.sx, out.sy]).toEqual([d.sx, d.sy]);
    expect(out.frames).toHaveLength(4);
  });
});

describe('the speed slider', () => {
  it('is 100% in the middle, 0 at the left and 2000% at the right', () => {
    expect(speedFromSlider(0)).toBe(0);
    expect(speedFromSlider(500)).toBe(100);
    expect(speedFromSlider(1000)).toBe(MAX_SPEED_PCT);
    expect(speedToSlider(100)).toBe(500);
    expect(speedToSlider(0)).toBe(0);
    expect(speedToSlider(2000)).toBe(1000);
  });

  it('has fine steps near 100% and rests on 100 just around it', () => {
    expect(speedFromSlider(495)).toBe(100);
    expect(speedFromSlider(503)).toBe(100);
    expect(speedFromSlider(505)).toBe(103);
    expect(speedFromSlider(510)).toBe(106);
    expect(speedFromSlider(450)).toBe(90);
    for (const p of [0, 25, 50, 150, 200, 500, 1000, 2000]) expect(Math.abs(speedFromSlider(speedToSlider(p)) - p)).toBeLessThanOrEqual(p / 100);
  });
});
