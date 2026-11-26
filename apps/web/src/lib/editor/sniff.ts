// What a file is, from its first bytes (a file's type and name can lie or be missing), and how
// small it has to be decoded so the tab keeps its memory. Pure (sniff.test.ts); the quick upload
// form uses `isVideoName` and `EDITOR_ACCEPT` without loading the editor.

export type MediaKind = 'gif' | 'png' | 'jpeg' | 'webp' | 'avif' | 'mp4' | 'webm' | 'unknown';

export const MIME: Record<Exclude<MediaKind, 'unknown'>, string> = {
  gif: 'image/gif',
  png: 'image/png',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  avif: 'image/avif',
  mp4: 'video/mp4',
  webm: 'video/webm',
};

const ascii = (b: Uint8Array, at: number, n: number) => String.fromCharCode(...b.subarray(at, at + n));

export function sniff(b: Uint8Array): MediaKind {
  if (b.length < 12) return 'unknown';
  if (ascii(b, 0, 4) === 'GIF8') return 'gif';
  if (b[0] === 0x89 && ascii(b, 1, 3) === 'PNG') return 'png';
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return 'jpeg';
  if (ascii(b, 0, 4) === 'RIFF' && ascii(b, 8, 4) === 'WEBP') return 'webp';
  if (b[0] === 0x1a && b[1] === 0x45 && b[2] === 0xdf && b[3] === 0xa3) return 'webm';
  if (ascii(b, 4, 4) === 'ftyp') {
    // The major brand, then the compatible ones: AVIF pictures are ISO files too.
    const size = Math.min(b.length, (b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3] || 32);
    const brands = [ascii(b, 8, 4)];
    for (let at = 16; at + 4 <= size; at += 4) brands.push(ascii(b, at, 4));
    if (brands.some((x) => x === 'avif' || x === 'avis')) return 'avif';
    return 'mp4';
  }
  return 'unknown';
}

export const isVideo = (k: MediaKind) => k === 'mp4' || k === 'webm';

/** A video file, by its type or name (before reading it). */
export const isVideoName = (f: Pick<File, 'type' | 'name'>) => f.type.startsWith('video/') || /\.(mp4|m4v|mov|webm|mkv)$/i.test(f.name);

/** What the editor's file picker offers: any picture or video. */
export const EDITOR_ACCEPT = 'image/*,video/mp4,video/webm,video/quicktime,.mp4,.m4v,.mov,.webm,.avif,.apng';

/** How much the editor decodes at most: pixels of all frames together, a side, frames. */
export type Budget = { pixels: number; side: number; frames: number };

/**
 * 120 MP of frames (about 480 MB of pictures; more than the 100 MP an upload may have, so nothing
 * that could upload is shrunk), 2048 px a side, 3000 frames. Less on a device that says it has
 * 4 GB of memory or less.
 */
export function budgetFor(deviceMemory?: number): Budget {
  const low = deviceMemory !== undefined && deviceMemory <= 4;
  return { pixels: low ? 48_000_000 : 120_000_000, side: 2048, frames: 3000 };
}

/**
 * The scale to decode a w × h picture of `frames` frames at (≤ 1), and how many frames to read
 * (fewer than it has when it has more than the budget's frames).
 */
export function importScale(w: number, h: number, frames: number, b: Budget): { scale: number; frames: number; width: number; height: number } {
  const n = Math.max(1, Math.min(frames, b.frames));
  const byPixels = Math.sqrt(b.pixels / (w * h * n));
  const bySide = b.side / Math.max(w, h);
  const scale = Math.min(1, byPixels, bySide);
  return { scale, frames: n, width: Math.max(1, Math.round(w * scale)), height: Math.max(1, Math.round(h * scale)) };
}
