// Limits checked on what a file *says* it is (its headers) before anything is allocated for it,
// so a small crafted file can't ask the fallback decoders for gigabytes. Pure (guard.test.ts).
import { parseGIF, type ParsedGif } from 'gifuct-js';
import type { WebpAnimation } from './riff';
import type { Budget } from './sniff';

/** A message shown to the person; the worker passes it on as the import's error. */
export class Failure extends Error {}

/** The largest file the editor reads into memory: 512 MiB. */
export const MAX_FILE_BYTES = 512 * 1024 * 1024;
/** A frame can't be wider or taller than this (GIF's own limit is 65 535; canvases stop near 32 768). */
export const MAX_SIDE = 16_384;

export function assertFileSize(bytes: number) {
  if (bytes > MAX_FILE_BYTES) throw new Failure('That file is over 512 MB, too big for the editor to open. Trim or compress it first.');
}

/** Refuses a canvas that is empty, too wide or tall, or has more pixels than one budget. */
export function assertCanvasSize(w: number, h: number, maxPixels: number) {
  if (!Number.isFinite(w) || !Number.isFinite(h) || w < 1 || h < 1) throw new Failure('That file’s picture has no size.');
  if (w > MAX_SIDE || h > MAX_SIDE || w * h > maxPixels) throw new Failure('That picture is too large for the editor to open. Shrink it first.');
}

/**
 * The checks every decoder makes once it knows the logical size and frame count and before it
 * allocates: the whole canvas within the pixel budget, and not an absurd number of frames (up to
 * the budget's frames are read, a few times more are cut off, beyond that it is refused).
 */
export function assertDecodable(w: number, h: number, frames: number, b: Budget) {
  assertCanvasSize(w, h, b.pixels);
  if (frames > b.frames * 10) throw new Failure('That file has too many frames for the editor to open.');
}

/** A GIF's logical screen size from its first 10 bytes, before the file is parsed. */
export function gifScreen(b: Uint8Array): { width: number; height: number } | null {
  if (b.length < 10) return null;
  return { width: b[6] | (b[7] << 8), height: b[8] | (b[9] << 8) };
}

export type GifFrame = Extract<ParsedGif['frames'][number], { image: unknown }>;

/**
 * Parses a GIF after checking its logical screen from the header, then its frame count and each
 * frame's size, all before a single frame is decompressed (the caller does that one at a time).
 */
export function readGif(buf: ArrayBuffer, b: Budget): { gif: ParsedGif; frames: GifFrame[]; width: number; height: number } {
  const screen = gifScreen(new Uint8Array(buf, 0, Math.min(buf.byteLength, 10)));
  if (screen && screen.width && screen.height) assertDecodable(screen.width, screen.height, 0, b);
  const gif = parseGIF(buf);
  const frames = gif.frames.filter((f): f is GifFrame => 'image' in f && !!f.image);
  const { width, height } = gif.lsd;
  if (!frames.length || !width || !height) throw new Failure('That GIF has no pictures in it.');
  assertDecodable(width, height, frames.length, b);
  for (const f of frames.slice(0, b.frames)) assertCanvasSize(f.image.descriptor.width, f.image.descriptor.height, b.pixels);
  return { gif, frames, width, height };
}

/** The same checks for an animated WebP's canvas, frame count and frame rectangles. */
export function assertWebp(anim: WebpAnimation, b: Budget) {
  assertDecodable(anim.width, anim.height, anim.frames.length, b);
  for (const f of anim.frames.slice(0, b.frames)) assertCanvasSize(f.w, f.h, b.pixels);
}
