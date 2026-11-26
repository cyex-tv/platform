import { describe, expect, it } from 'vitest';
import { Failure, MAX_FILE_BYTES, assertCanvasSize, assertDecodable, assertFileSize, assertWebp, gifScreen, readGif } from './guard';
import { budgetFor } from './sniff';

const budget = budgetFor();
const le16 = (n: number) => [n & 255, n >> 8];

/** A GIF with a logical screen of w × h and `frames` one-pixel frames of the given size each. */
function gif(w: number, h: number, frames: number, frame = { w: 1, h: 1 }): Uint8Array {
  const out: number[] = [...Buffer.from('GIF89a'), ...le16(w), ...le16(h), 0x80, 0, 0, 0, 0, 0, 255, 255, 255];
  const one = [0x2c, ...le16(0), ...le16(0), ...le16(frame.w), ...le16(frame.h), 0, 2, 2, 0x44, 0x01, 0];
  for (let i = 0; i < frames; i++) out.push(...one);
  out.push(0x3b);
  return Uint8Array.from(out);
}
const buf = (b: Uint8Array) => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) as ArrayBuffer;

describe('import limits', () => {
  it('refuses a GIF whose logical screen is huge from its header alone, before parsing', () => {
    const header = gif(65535, 65535, 0).subarray(0, 13); // not even a whole file: nothing is parsed
    expect(gifScreen(header)).toEqual({ width: 65535, height: 65535 });
    expect(() => readGif(buf(header), budget)).toThrow(Failure);
    expect(() => readGif(buf(header), budget)).toThrow(/too large/);
  });

  it('refuses a screen over the pixel budget even when each side is allowed', () => {
    expect(() => assertCanvasSize(12_000, 12_000, budget.pixels)).toThrow(Failure); // 144 MP
    expect(() => assertCanvasSize(10_000, 10_000, budget.pixels)).not.toThrow(); // 100 MP
    expect(() => assertCanvasSize(0, 10, budget.pixels)).toThrow(Failure);
    expect(() => assertCanvasSize(NaN, 10, budget.pixels)).toThrow(Failure);
  });

  it('refuses an absurd frame count and reads a big but allowed one', () => {
    expect(() => readGif(buf(gif(4, 4, budget.frames * 10 + 1)), budget)).toThrow(/too many frames/);
    const ok = readGif(buf(gif(4, 4, budget.frames + 1)), budget);
    expect(ok.frames).toHaveLength(budget.frames + 1); // the worker reads the first `budget.frames`
    expect(ok.width).toBe(4);
  });

  it('refuses a frame that claims a huge picture inside a small screen', () => {
    expect(() => readGif(buf(gif(10, 10, 1, { w: 65535, h: 65535 })), budget)).toThrow(/too large/);
  });

  it('accepts a normal GIF', () => {
    const g = readGif(buf(gif(32, 16, 3)), budget);
    expect(g.frames).toHaveLength(3);
    expect([g.width, g.height]).toEqual([32, 16]);
  });

  it('checks size and frames of an animated WebP before decoding', () => {
    const frame = { x: 0, y: 0, w: 2, h: 2, delay: 40, blend: true, dispose: false, chunks: [] };
    expect(() => assertWebp({ width: 16_000_000, height: 16_000_000, loops: 0, frames: [frame] }, budget)).toThrow(Failure);
    expect(() => assertWebp({ width: 2, height: 2, loops: 0, frames: Array(budget.frames * 10 + 1).fill(frame) }, budget)).toThrow(/too many frames/);
    expect(() => assertWebp({ width: 8, height: 8, loops: 0, frames: [{ ...frame, w: 20_000, h: 20_000 }] }, budget)).toThrow(Failure);
    expect(() => assertWebp({ width: 8, height: 8, loops: 0, frames: [frame] }, budget)).not.toThrow();
  });

  it('refuses a file over the size cap with a clear message', () => {
    expect(() => assertFileSize(MAX_FILE_BYTES)).not.toThrow();
    expect(() => assertFileSize(MAX_FILE_BYTES + 1)).toThrow(/512 MB/);
  });

  it('checks the frame count against the budget', () => {
    expect(() => assertDecodable(10, 10, budget.frames, budget)).not.toThrow();
    expect(() => assertDecodable(10, 10, budget.frames * 10 + 1, budget)).toThrow(Failure);
  });
});
