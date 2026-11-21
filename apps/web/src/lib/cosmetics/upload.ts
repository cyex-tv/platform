// Badge and paint pictures before and after `POST /v1/cosmetics/images`: the API's rules checked
// on the page first (so a wrong file says why at once instead of after an upload), and the wait
// until the worker has made the files, read back through GraphQL every time (an upload of bytes
// the API already has answers at once with ready set, in the REST shape, which is not the one the
// site renders from).
import { ApiError } from '$lib/api';

export type CosmeticUploadKind = 'badge' | 'paint';

/**
 * What the API takes (crates/media `Limits::badge` and `Limits::paint`, and
 * cosmetics::check_dimensions). Badges follow the cyex badge upload rules: PNG, APNG, JPEG, WebP
 * or GIF, square, at least 72 px, up to 8 MB; an animation at most 120 frames, 10 seconds and
 * 64 MP (width × height × frames). Paints take the same file types, size and animation limits,
 * with their own shape: at least 64 px tall and at most 8:1 wide, up to 4096 px a side.
 */
export const PICTURE_RULES = {
  badge: {
    types: ['image/png', 'image/apng', 'image/jpeg', 'image/gif', 'image/webp'],
    maxBytes: 8 * 1024 * 1024,
    minSide: 72,
    maxSide: 2048,
    maxFrames: 120,
    maxDurationMs: 10_000,
    maxTotalPixels: 64_000_000,
  },
  paint: {
    types: ['image/png', 'image/apng', 'image/jpeg', 'image/gif', 'image/webp'],
    maxBytes: 8 * 1024 * 1024,
    minHeight: 64,
    maxAspect: 8,
    maxSide: 4096,
    maxFrames: 120,
    maxDurationMs: 10_000,
    maxTotalPixels: 64_000_000,
  },
} as const;

/** The `accept` list for a picture input of `kind`. */
export const pictureAccept = (kind: CosmeticUploadKind) => PICTURE_RULES[kind].types.join(',');

const mb = (n: number) => (n / 1024 / 1024).toFixed(1).replace('.', ',');

/** Why a file can't be a `kind` picture (type and size); null when it can be sent. */
export function checkFile(file: Pick<File, 'type' | 'size'>, kind: CosmeticUploadKind): string | null {
  const rules = PICTURE_RULES[kind];
  if (!(rules.types as readonly string[]).includes(file.type))
    return 'Choose a PNG, JPEG, GIF or WebP file.';
  const most = rules.maxBytes / 1024 / 1024;
  if (file.size > rules.maxBytes)
    return `${kind === 'badge' ? 'Badge pictures' : 'Paint pictures'} are at most ${most} MB (this one is ${mb(file.size)} MB).`;
  return null;
}

/** Why a picture of this size can't be used for `kind`; null when it can. */
export function checkSize(kind: CosmeticUploadKind, width: number, height: number): string | null {
  if (kind === 'badge') {
    const r = PICTURE_RULES.badge;
    if (!(width === height && width >= r.minSide))
      return `Badge pictures are square and at least 72 × 72 px (this one is ${width} × ${height} px).`;
    if (width > r.maxSide)
      return `Badge pictures are at most ${r.maxSide} × ${r.maxSide} px (this one is ${width} × ${height} px). The largest size made is 512 px.`;
    return null;
  }
  const r = PICTURE_RULES.paint;
  if (height < r.minHeight) return `Paint pictures are at least 64 px tall (this one is ${height} px).`;
  if (width > height * r.maxAspect)
    return `Paint pictures are at most 8 times as wide as they are tall, 8:1 (this one is ${width} × ${height} px).`;
  if (width > r.maxSide || height > r.maxSide)
    return `Paint pictures are at most ${r.maxSide} px a side (this one is ${width} × ${height} px). The largest size made is 512 px tall.`;
  return null;
}

/** An animation's frame count and how long one run shows, as browsers show it. */
export type Animation = { frames: number; durationMs: number };

/** Frames under 20 ms show for 100 ms, as browsers (and the API) count them. */
const shown = (ms: number) => (ms < 20 ? 100 : ms);

function gifAnimation(b: Uint8Array): Animation | null {
  if (b.length < 13) return null;
  let i = 13;
  if (b[10] & 0x80) i += 3 * 2 ** ((b[10] & 7) + 1);
  const skipBlocks = () => {
    while (i < b.length && b[i] !== 0) i += b[i] + 1;
    i += 1;
  };
  let frames = 0;
  let durationMs = 0;
  let delay = 0;
  while (i < b.length) {
    const tag = b[i];
    if (tag === 0x3b) break;
    if (tag === 0x21) {
      const label = b[i + 1];
      i += 2;
      if (label === 0xf9 && b[i] >= 4) delay = (b[i + 2] | (b[i + 3] << 8)) * 10;
      skipBlocks();
    } else if (tag === 0x2c) {
      const packed = b[i + 9];
      i += 10;
      if (packed & 0x80) i += 3 * 2 ** ((packed & 7) + 1);
      i += 1; // the LZW minimum code size
      skipBlocks();
      frames += 1;
      durationMs += shown(delay);
      delay = 0;
    } else break;
  }
  return frames > 1 ? { frames, durationMs } : null;
}

function pngAnimation(b: Uint8Array): Animation | null {
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  let i = 8;
  let animated = false;
  let frames = 0;
  let durationMs = 0;
  while (i + 8 <= b.length) {
    const len = v.getUint32(i);
    const type = String.fromCharCode(b[i + 4], b[i + 5], b[i + 6], b[i + 7]);
    const data = i + 8;
    if (type === 'acTL') animated = true;
    if (type === 'fcTL' && data + 24 <= b.length) {
      const num = v.getUint16(data + 20);
      const den = v.getUint16(data + 22) || 100;
      frames += 1;
      durationMs += shown(Math.floor((num * 1000) / den));
    }
    if (type === 'IEND') break;
    i = data + len + 4;
  }
  return animated && frames > 1 ? { frames, durationMs } : null;
}

function webpAnimation(b: Uint8Array): Animation | null {
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  let i = 12;
  let frames = 0;
  let durationMs = 0;
  while (i + 8 <= b.length) {
    const type = String.fromCharCode(b[i], b[i + 1], b[i + 2], b[i + 3]);
    const len = v.getUint32(i + 4, true);
    const data = i + 8;
    if (type === 'ANMF' && data + 16 <= b.length) {
      frames += 1;
      durationMs += shown(b[data + 12] | (b[data + 13] << 8) | (b[data + 14] << 16));
    }
    i = data + len + (len & 1);
  }
  return frames > 1 ? { frames, durationMs } : null;
}

/**
 * The frames and run time of an animated GIF, APNG or WebP, read from its bytes without decoding
 * a picture; null for a still image or anything else (the API has the last word either way).
 */
export function animationOf(bytes: Uint8Array): Animation | null {
  const tag = (at: number, s: string) => [...s].every((c, k) => bytes[at + k] === c.charCodeAt(0));
  try {
    if (tag(0, 'GIF8')) return gifAnimation(bytes);
    if (bytes[0] === 0x89 && tag(1, 'PNG')) return pngAnimation(bytes);
    if (tag(0, 'RIFF') && tag(8, 'WEBP')) return webpAnimation(bytes);
  } catch {
    return null;
  }
  return null;
}

/** Why an animated `kind` picture of `width`×`height` can't be used; null when it can. */
export function checkAnimation(kind: CosmeticUploadKind, anim: Animation | null, width: number, height: number): string | null {
  if (!anim) return null;
  const r = PICTURE_RULES[kind];
  const what = kind === 'badge' ? 'badges' : 'paints';
  if (anim.frames > r.maxFrames) return `Animated ${what} have at most ${r.maxFrames} frames (this one has ${anim.frames}).`;
  if (anim.durationMs > r.maxDurationMs)
    return `Animated ${what} run at most ${r.maxDurationMs / 1000} seconds (this one runs ${(anim.durationMs / 1000).toFixed(1).replace('.', ',')} s).`;
  if (width * height * anim.frames > r.maxTotalPixels)
    return `This animation is too big to process: width × height × frames is over ${r.maxTotalPixels / 1_000_000} MP. Use fewer frames or a smaller picture (${kind === 'badge' ? '512 × 512 px' : '512 px tall'} is the largest size made).`;
  return null;
}

/** Codes that asking again won't fix: stop waiting and say so. */
const FINAL = new Set(['signed_out', 'forbidden', 'bad_origin', 'not_found', 'aborted']);

/**
 * Asks `read(id)` until the picture is ready (every `everyMs`, at most `timeoutMs`), and returns
 * it. A network blip is asked again; signed out, forbidden or gone end the wait with that reason.
 */
export async function waitReady<T extends { ready: boolean }>(
  read: () => Promise<T | null | undefined>,
  opts: { everyMs?: number; timeoutMs?: number; signal?: AbortSignal; sleep?: (ms: number) => Promise<void> } = {},
): Promise<T> {
  const every = opts.everyMs ?? 1500;
  const until = Date.now() + (opts.timeoutMs ?? 120_000);
  const sleep = opts.sleep ?? ((ms: number) => new Promise<void>((r) => setTimeout(r, ms)));
  for (;;) {
    if (opts.signal?.aborted) throw new ApiError('Stopped.', 'aborted');
    try {
      const img = await read();
      if (!img) throw new ApiError('The uploaded picture is gone. Try again.', 'not_found');
      if (img.ready) return img;
    } catch (e) {
      if (e instanceof ApiError && FINAL.has(e.code)) throw e;
    }
    if (Date.now() >= until) throw new ApiError('The picture is taking long to process. Try again in a minute.', 'image_not_ready');
    await sleep(every);
  }
}
