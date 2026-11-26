import { describe, expect, it } from 'vitest';
import { EDITOR_LIMITS, allPass, checkRules, editorLimits, fixLabel, onlySizePending } from './rules';
import { chunks, isWebp, muxAnimation, readAnimation, stillWebp } from './riff';

const ok = { width: 112, height: 112, frames: 20, shortest: 40, duration: 800, size: 200_000 };
const state = (o: Partial<typeof ok>) => Object.fromEntries(checkRules({ ...ok, ...o }, EDITOR_LIMITS).map((r) => [r.id, r.state]));

describe('upload rules', () => {
  it('passes a normal emote', () => {
    const r = checkRules(ok, EDITOR_LIMITS);
    expect(allPass(r)).toBe(true);
    expect(r.find((x) => x.id === 'side')?.value).toBe('112 × 112 px');
  });

  it('fails each limit with its fix', () => {
    const fixes = (o: Partial<typeof ok>) => checkRules({ ...ok, ...o }, EDITOR_LIMITS).filter((r) => r.state === 'fail').map((r) => r.fix);
    expect(fixes({ width: 1001, height: 1001 })).toEqual(['fit']);
    expect(fixes({ width: 450, height: 100 })).toEqual(['aspect']);
    expect(fixes({ frames: 1001, width: 10, height: 10 })).toEqual(['drop']);
    expect(fixes({ shortest: 19 })).toEqual(['fps']);
    expect(fixes({ width: 1000, height: 1000, frames: 101 })).toEqual(['drop']);
    expect(fixes({ size: 9 * 1024 * 1024 })).toEqual(['shrink']);
  });

  it('accepts the edges: 1000 px, 4:1, 1000 frames, 20 ms, 8 MB', () => {
    expect(state({ width: 1000, height: 250 })).toMatchObject({ side: 'pass', aspect: 'pass' });
    expect(state({ frames: 1000, width: 10, height: 10 }).frames).toBe('pass');
    expect(state({ shortest: 20 }).fps).toBe('pass');
    expect(state({ size: 8 * 1024 * 1024 }).bytes).toBe('pass');
    // a still has no frame rate
    expect(state({ frames: 1, shortest: Infinity }).fps).toBe('pass');
  });

  it('waits for the encoder before the size passes', () => {
    const r = checkRules({ ...ok, size: null }, EDITOR_LIMITS);
    expect(allPass(r)).toBe(false);
    expect(onlySizePending(r)).toBe(true);
  });

  it('reads the API numbers, the account caps first, with the built-in fallback', () => {
    expect(editorLimits(null, null)).toEqual(EDITOR_LIMITS);
    const l = editorLimits({ maxBytes: 4, maxSide: 500, maxFrames: 10, maxAspect: 3, maxFps: 50, maxTotalPixels: 50, maxDurationMs: null }, { maxBytes: 9, maxSide: 600, maxFrames: 20 });
    expect(l).toMatchObject({ maxBytes: 9, maxSide: 600, maxFrames: 20, maxAspect: 3, maxTotalPixels: 50, minDelayMs: 20, maxDurationMs: null });
    expect(editorLimits(null, { maxBytes: 1, maxSide: 1, maxFrames: 1, maxFps: 25, maxAspect: 2 })).toMatchObject({ minDelayMs: 40, maxAspect: 2 });
    expect(fixLabel('fps', l)).toBe('Limit to 50 fps');
    expect(fixLabel('aspect', l)).toBe('Crop to 3:1');
  });
});

describe('webp container', () => {
  const vp8l = { id: 'VP8L', data: new Uint8Array([0x2f, 1, 2, 3, 4]) };
  it('muxes frames into an animation and reads them back', () => {
    const b = muxAnimation(30, 20, [
      { chunks: [vp8l], w: 30, h: 20, delay: 40 },
      { chunks: [vp8l], w: 30, h: 20, delay: 1234 },
    ]);
    expect(isWebp(b)).toBe(true);
    const a = readAnimation(b)!;
    expect(a).toMatchObject({ width: 30, height: 20, loops: 0 });
    expect(a.frames.map((f) => [f.delay, f.w, f.h, f.blend, f.dispose])).toEqual([
      [40, 30, 20, false, false],
      [1234, 30, 20, false, false],
    ]);
    expect([...a.frames[0].chunks[0].data]).toEqual([...vp8l.data]);
    // RIFF size: the file length minus 8
    expect(new DataView(b.buffer).getUint32(4, true)).toBe(b.length - 8);
  });

  it('makes a still from one frame, with VP8X when it has alpha', () => {
    const s = stillWebp([{ id: 'ALPH', data: new Uint8Array([1]) }, { id: 'VP8 ', data: new Uint8Array([2, 3]) }], 5, 6);
    expect(chunks(s).map((c) => c.id)).toEqual(['VP8X', 'ALPH', 'VP8 ']);
    expect(chunks(stillWebp([vp8l], 5, 6)).map((c) => c.id)).toEqual(['VP8L']);
    expect(readAnimation(s)).toBeNull();
  });
});
