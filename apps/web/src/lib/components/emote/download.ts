// Downloading an emote's files from the CDN (ADR 0003, ADR 0012): which size, format and
// still/animated choices the emote's file list offers, the file and URL for a choice, and the
// name the saved file gets. The file list is what the API sends, so only files that exist are offered.
// The CDN has WebP and AVIF only (no GIF, and no original). GIF is offered wherever a WebP exists
// and is made in the browser from that WebP (`makeGif`): decoded and encoded by the editor's media
// worker. GIF has 1-bit transparency, so soft edges are lost.
import type { PageEmote } from './emote-page';

export type DownloadFile = PageEmote['files'][number];
export type Choice = { scale: number; format: string; still: boolean };

const rank = (f: string) => (f === 'GIF' ? 0 : f === 'WEBP' ? 1 : f === 'AVIF' ? 2 : 3);

/** Animated emotes also come as still first frames; still emotes have only one kind. */
export function hasBoth(files: DownloadFile[]): boolean {
  return files.some((f) => f.isStatic) && files.some((f) => !f.isStatic);
}

/** The files of one kind: the moving ones (or, for a still emote, all) or the still frames. */
function ofKind(files: DownloadFile[], still: boolean): DownloadFile[] {
  return hasBoth(files) ? files.filter((f) => f.isStatic === still) : files;
}

/** The sizes (1 to 4) the kind has, smallest first. */
export function sizes(files: DownloadFile[], still: boolean): number[] {
  return [...new Set(ofKind(files, still).map((f) => f.scale))].sort((a, b) => a - b);
}

/** The formats the kind has at a size: GIF (made from the WebP, so only with one), WebP, AVIF. */
export function formatsAt(files: DownloadFile[], still: boolean, scale: number): string[] {
  const f: string[] = ofKind(files, still).filter((x) => x.scale === scale).map((x) => x.format);
  if (f.includes('WEBP')) f.push('GIF');
  return [...new Set(f)].sort((a, b) => rank(a) - rank(b));
}

/** The file a choice is saved from, or null when the emote doesn't have it. For GIF that is the WebP it is made from. */
export function pick(files: DownloadFile[], c: Choice): DownloadFile | null {
  const format = c.format === 'GIF' ? 'WEBP' : c.format;
  return ofKind(files, c.still).find((f) => f.scale === c.scale && f.format === format) ?? null;
}

/** Fixes a choice the new kind or size doesn't have: the next size up (or the largest), then the format (WebP if it has it). */
export function settle(files: DownloadFile[], c: Choice): Choice {
  const have = sizes(files, c.still);
  const scale = have.includes(c.scale) ? c.scale : (have.find((s) => s >= c.scale) ?? have.at(-1) ?? c.scale);
  const fmts = formatsAt(files, c.still, scale);
  return { still: c.still, scale, format: fmts.includes(c.format) ? c.format : fmts.includes('WEBP') ? 'WEBP' : (fmts[0] ?? c.format) };
}

/** `https://cdn…/emotes/{sha}/2x.webp`. */
export function fileUrl(host: string, f: DownloadFile): string {
  return `${host}/${f.name}`;
}

/** The extension of a file: `webp`, `avif`. */
export function ext(f: DownloadFile): string {
  const i = f.name.lastIndexOf('.');
  return (i >= 0 ? f.name.slice(i + 1) : f.format).toLowerCase();
}

/** `name_2x.webp`; the still frame of an animated emote is `name_2x_still.webp`; a GIF made from the
 * file passes `format: 'GIF'` for `.gif`. Path separators, quotes and control characters become
 * `_`; an empty name falls back to `emote`. */
export function fileName(name: string, f: DownloadFile, animated: boolean, format?: string): string {
  const safe = name
    .replace(/[\u0000-\u001f\u007f\\/:*?"<>|‪-‮⁦-⁩]/g, '_')
    .replace(/\s+/g, '_')
    .replace(/^\.+/, '')
    .slice(0, 80);
  return `${safe || 'emote'}_${f.scale}x${animated && f.isStatic ? '_still' : ''}.${format === 'GIF' ? 'gif' : ext(f)}`;
}

/** Human size of a file: `12 KB`. Empty when unknown. */
export function bytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return '';
  return n < 1024 ? `${n} B` : n < 1024 * 1024 ? `${Math.round(n / 1024)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;
}

function click(href: string, filename: string, download: boolean) {
  const a = document.createElement('a');
  a.href = href;
  if (download) a.download = filename;
  else {
    a.target = '_blank';
    a.rel = 'noopener';
  }
  document.body.append(a);
  a.click();
  a.remove();
}

/** Saves a file. The same origin saves through the `download` attribute; any other (the CDN) is
 * fetched and saved from a Blob, because the attribute is ignored across origins. When the fetch
 * is refused (no CORS), the file opens in a new tab instead. Resolves true when it saved. */
export async function save(url: string, filename: string): Promise<boolean> {
  const abs = new URL(url, location.href);
  if (abs.origin === location.origin) {
    click(abs.href, filename, true);
    return true;
  }
  try {
    const r = await fetch(abs.href, { mode: 'cors', credentials: 'omit' });
    if (!r.ok) throw new Error(String(r.status));
    saveBlob(await r.blob(), filename);
    return true;
  } catch {
    click(abs.href, filename, false);
    return false;
  }
}

/** Saves a Blob made here (a GIF) under `filename`. */
export function saveBlob(blob: Blob, filename: string): void {
  const obj = URL.createObjectURL(blob);
  click(obj, filename, true);
  setTimeout(() => URL.revokeObjectURL(obj), 10_000);
}

/** Makes a GIF from an emote's WebP at `url`: fetched from the CDN, decoded and encoded by the
 * editor's media worker (loaded only now). Rejects when the fetch (CORS) or the decoding fails;
 * `signal` cancels it. */
export async function makeGif(url: string, signal: AbortSignal): Promise<Blob> {
  const r = await fetch(new URL(url, location.href).href, { mode: 'cors', credentials: 'omit', signal });
  if (!r.ok) throw new Error(String(r.status));
  const file = new File([await r.blob()], 'emote.webp', { type: 'image/webp' });
  const { importFile, encodeFrames } = await import('$lib/editor/media');
  const { budgetFor } = await import('$lib/editor/sniff');
  const got = await importFile(file, budgetFor(), () => {}, signal);
  try {
    return await encodeFrames(got.bitmaps, got.delays, got.scaledWidth, got.scaledHeight, 'gif', () => {}, signal);
  } finally {
    for (const b of got.bitmaps) b.close();
  }
}
