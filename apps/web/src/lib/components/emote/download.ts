// Downloading an emote's files from the CDN (ADR 0003, ADR 0012): which size, format and
// still/animated choices the emote's file list offers, the file and URL for a choice, and the
// name the saved file gets. The file list is what the API sends, so only files that exist are offered.
import type { PageEmote } from './emote-page';

export type DownloadFile = PageEmote['files'][number];
export type Choice = { scale: number; format: string; still: boolean };

const rank = (f: string) => (f === 'WEBP' ? 0 : f === 'AVIF' ? 1 : 2);

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

/** The formats the kind has at a size, WebP first. */
export function formatsAt(files: DownloadFile[], still: boolean, scale: number): string[] {
  const f = ofKind(files, still).filter((x) => x.scale === scale).map((x) => x.format);
  return [...new Set(f)].sort((a, b) => rank(a) - rank(b));
}

/** The file for a choice, or null when the emote doesn't have it. */
export function pick(files: DownloadFile[], c: Choice): DownloadFile | null {
  return ofKind(files, c.still).find((f) => f.scale === c.scale && f.format === c.format) ?? null;
}

/** Fixes a choice the new kind or size doesn't have: the next size up (or the largest), then the format. */
export function settle(files: DownloadFile[], c: Choice): Choice {
  const have = sizes(files, c.still);
  const scale = have.includes(c.scale) ? c.scale : (have.find((s) => s >= c.scale) ?? have.at(-1) ?? c.scale);
  const fmts = formatsAt(files, c.still, scale);
  return { still: c.still, scale, format: fmts.includes(c.format) ? c.format : (fmts[0] ?? c.format) };
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

/** `name-2x.webp`; the still frame of an animated emote is `name-2x-still.webp`. Path separators,
 * quotes and control characters become `_`; an empty name falls back to `emote`. */
export function fileName(name: string, f: DownloadFile, animated: boolean): string {
  const safe = name
    .replace(/[\u0000-\u001f\u007f\\/:*?"<>|‪-‮⁦-⁩]/g, '_')
    .replace(/\s+/g, '_')
    .replace(/^\.+/, '')
    .slice(0, 80);
  return `${safe || 'emote'}-${f.scale}x${animated && f.isStatic ? '-still' : ''}.${ext(f)}`;
}

/** Human size of a file: `12 KB`. Empty when unknown. */
export function bytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return '';
  return n < 1024 ? `${n} B` : n < 1024 * 1024 ? `${Math.round(n / 1024)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;
}

/** Saves a file. The same origin saves through the `download` attribute; any other (the CDN) is
 * fetched and saved from a Blob, because the attribute is ignored across origins. When the fetch
 * is refused (no CORS), the file opens in a new tab instead. Resolves true when it saved. */
export async function save(url: string, filename: string): Promise<boolean> {
  const click = (href: string, download: boolean) => {
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
  };
  const abs = new URL(url, location.href);
  if (abs.origin === location.origin) {
    click(abs.href, true);
    return true;
  }
  try {
    const r = await fetch(abs.href, { mode: 'cors', credentials: 'omit' });
    if (!r.ok) throw new Error(String(r.status));
    const obj = URL.createObjectURL(await r.blob());
    click(obj, true);
    setTimeout(() => URL.revokeObjectURL(obj), 10_000);
    return true;
  } catch {
    click(abs.href, false);
    return false;
  }
}
