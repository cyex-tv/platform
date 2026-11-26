// WebP's container (RIFF), pure (riff.test.ts): reading an animated WebP's frames apart, so a
// browser without ImageDecoder can still decode each one as a still picture, and putting still
// WebP frames (as the browser's canvas encoder writes them) together into one animated WebP.
// See the WebP container specification (developers.google.com/speed/webp/docs/riff_container).

export type Chunk = { id: string; data: Uint8Array };

const tag = (b: Uint8Array, at: number) => String.fromCharCode(b[at], b[at + 1], b[at + 2], b[at + 3]);
const u24 = (b: Uint8Array, at: number) => b[at] | (b[at + 1] << 8) | (b[at + 2] << 16);
const u32 = (b: Uint8Array, at: number) => (b[at] | (b[at + 1] << 8) | (b[at + 2] << 16) | (b[at + 3] << 24)) >>> 0;

export const isWebp = (b: Uint8Array) => b.length >= 16 && tag(b, 0) === 'RIFF' && tag(b, 8) === 'WEBP';

/** The chunks in `b` from `start` to `end` (a RIFF body, or an ANMF frame's data). */
export function chunks(b: Uint8Array, start = 12, end = b.length): Chunk[] {
  const out: Chunk[] = [];
  let at = start;
  while (at + 8 <= end) {
    const id = tag(b, at);
    const size = u32(b, at + 4);
    const from = at + 8;
    if (from + size > end) break;
    out.push({ id, data: b.subarray(from, from + size) });
    at = from + size + (size & 1);
  }
  return out;
}

export type WebpFrame = { x: number; y: number; w: number; h: number; delay: number; blend: boolean; dispose: boolean; chunks: Chunk[] };
export type WebpAnimation = { width: number; height: number; loops: number; frames: WebpFrame[] };

/** An animated WebP's frames, or null when `b` isn't one. */
export function readAnimation(b: Uint8Array): WebpAnimation | null {
  if (!isWebp(b)) return null;
  const all = chunks(b);
  const vp8x = all.find((c) => c.id === 'VP8X');
  if (!vp8x || !(vp8x.data[0] & 0x02)) return null;
  const width = u24(vp8x.data, 4) + 1;
  const height = u24(vp8x.data, 7) + 1;
  const anim = all.find((c) => c.id === 'ANIM');
  const loops = anim ? anim.data[4] | (anim.data[5] << 8) : 0;
  const frames = all
    .filter((c) => c.id === 'ANMF' && c.data.length >= 16)
    .map((c) => {
      const d = c.data;
      return {
        x: u24(d, 0) * 2,
        y: u24(d, 3) * 2,
        w: u24(d, 6) + 1,
        h: u24(d, 9) + 1,
        delay: u24(d, 12),
        blend: !(d[15] & 0x02),
        dispose: !!(d[15] & 0x01),
        chunks: chunks(d, 16, d.length),
      };
    });
  return frames.length ? { width, height, loops, frames } : null;
}

/** Bytes put together from parts, copied once at the end. */
class Bytes {
  parts: Uint8Array[] = [];
  length = 0;
  add(p: ArrayLike<number>) {
    const u = p instanceof Uint8Array ? p : Uint8Array.from(p);
    this.parts.push(u);
    this.length += u.length;
  }
  done(): Uint8Array {
    const out = new Uint8Array(this.length);
    let at = 0;
    for (const p of this.parts) {
      out.set(p, at);
      at += p.length;
    }
    return out;
  }
}

const le32 = (n: number) => [n & 255, (n >> 8) & 255, (n >> 16) & 255, (n >>> 24) & 255];
const ascii = (id: string) => [id.charCodeAt(0), id.charCodeAt(1), id.charCodeAt(2), id.charCodeAt(3)];

function writeChunk(out: Bytes, id: string, data: ArrayLike<number>) {
  out.add([...ascii(id), ...le32(data.length)]);
  out.add(data);
  if (data.length & 1) out.add([0]);
}

const le24 = (n: number) => [n & 255, (n >> 8) & 255, (n >> 16) & 255];

function riff(body: Bytes): Uint8Array {
  const out = new Bytes();
  out.add([...ascii('RIFF'), ...le32(body.length + 4), ...ascii('WEBP')]);
  for (const p of body.parts) out.add(p);
  return out.done();
}

/** The picture chunks of a still WebP (ALPH and VP8, or VP8L), without its header chunks. */
export function pictureChunks(b: Uint8Array): Chunk[] {
  return chunks(b).filter((c) => c.id === 'ALPH' || c.id === 'VP8 ' || c.id === 'VP8L');
}

const hasAlpha = (cs: Chunk[]) => cs.some((c) => c.id === 'ALPH' || c.id === 'VP8L');

/** A still WebP of w × h from one frame's picture chunks (so a browser can decode it alone). */
export function stillWebp(cs: Chunk[], w: number, h: number): Uint8Array {
  const body = new Bytes();
  if (cs.length === 1 && cs[0].id !== 'ALPH') {
    writeChunk(body, cs[0].id, cs[0].data);
    return riff(body);
  }
  writeChunk(body, 'VP8X', [hasAlpha(cs) ? 0x10 : 0, 0, 0, 0, ...le24(w - 1), ...le24(h - 1)]);
  for (const c of cs) writeChunk(body, c.id, c.data);
  return riff(body);
}

export type MuxFrame = { chunks: Chunk[]; w: number; h: number; delay: number };

/**
 * An animated WebP of width × height from whole frames (each covering the canvas at 0, 0), looping
 * forever. Each frame replaces the one before (no blending), so transparent pixels stay clear.
 */
export function muxAnimation(width: number, height: number, frames: readonly MuxFrame[]): Uint8Array {
  const body = new Bytes();
  const alpha = frames.some((f) => hasAlpha(f.chunks));
  writeChunk(body, 'VP8X', [0x02 | (alpha ? 0x10 : 0), 0, 0, 0, ...le24(width - 1), ...le24(height - 1)]);
  // Background: clear; loop count 0 = forever.
  writeChunk(body, 'ANIM', [0, 0, 0, 0, 0, 0]);
  for (const f of frames) {
    const data = new Bytes();
    data.add([...le24(0), ...le24(0), ...le24(f.w - 1), ...le24(f.h - 1), ...le24(Math.min(0xffffff, Math.max(0, Math.round(f.delay)))), 0x02]);
    for (const c of f.chunks) writeChunk(data, c.id, c.data);
    writeChunk(body, 'ANMF', data.done());
  }
  return riff(body);
}
