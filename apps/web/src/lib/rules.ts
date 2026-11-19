// The API's rules for names, tags and uploads, checked in the browser first so a mistake shows
// next to the field. The API checks them again; these only save a round trip.

import { unreachableMessage } from './components/problem';

/** An emote file's size cap, as the API's (8 MB; `cyex_tv_media::EMOTE_MAX_BYTES`). */
export const MAX_UPLOAD_BYTES = 8 * 1024 * 1024;
/** The types the API decodes. AVIF is left out until the API takes it. */
export const UPLOAD_TYPES = ['image/png', 'image/apng', 'image/gif', 'image/webp', 'image/jpeg'];
/** The same by extension, for a browser that names no type (some systems don't know .apng). */
const UPLOAD_EXTENSIONS = /\.(png|apng|gif|webp|jpe?g)$/i;
/** What the file picker offers. */
export const UPLOAD_ACCEPT = [...UPLOAD_TYPES, '.png', '.apng', '.gif', '.webp', '.jpg', '.jpeg'].join(',');
/** The formats in words, for hints and errors. */
export const UPLOAD_FORMATS = 'PNG, APNG, GIF, WebP or JPEG';

/** Tags per emote, and characters per tag, as the API allows. */
export const TAG_MAX = 10;
export const TAG_LENGTH = 32;

/** An emote name's problem, or null when it's fine: 1 to 100 characters, no spaces. */
export function nameProblem(raw: string): string | null {
  const name = raw.trim();
  const length = [...name].length;
  if (length === 0) return 'Give it a name.';
  if (length > 100) return 'At most 100 characters.';
  if (/[\s\p{Cc}]/u.test(name)) return 'No spaces in names.';
  return null;
}

/**
 * A name made from a file's name, for the upload form: no extension, no spaces or control
 * characters between words (they join, "cat jam.gif" is "catjam"), at most 100 characters. Empty
 * when nothing usable is left.
 */
export function nameFromFile(file: string): string {
  const base = file.replace(/\.[^.]+$/, '');
  return [...base.replace(/[\s\p{Cc}]+/gu, '')].slice(0, 100).join('');
}

/** Tags from a comma-separated field: trimmed, lowercased, no duplicates. */
export function parseTags(raw: string): string[] {
  return [...new Set(raw.split(',').map((t) => t.trim().toLowerCase()).filter(Boolean))];
}

/** The tags' problem, or null: at most 10, each 1 to 32 letters, digits, - or _. */
export function tagsProblem(tags: string[]): string | null {
  if (tags.length > TAG_MAX) return `At most ${TAG_MAX} tags.`;
  const bad = tags.find((t) => [...t].length > TAG_LENGTH || !/^[\p{L}\p{N}_-]+$/u.test(t));
  return bad ? `“${bad}” isn’t a tag: 1 to 32 letters, digits, - or _.` : null;
}

/** A file's problem for upload, or null: PNG, APNG, GIF, WebP or JPEG, at most 8 MB. */
export function fileProblem(file: Pick<File, 'type' | 'size'> & { name?: string }): string | null {
  const known = UPLOAD_TYPES.includes(file.type) || (!file.type && !!file.name && UPLOAD_EXTENSIONS.test(file.name));
  if (!known) {
    // AVIF is a format people have, so say it by name and what to do instead.
    if (file.type === 'image/avif' || (!file.type && !!file.name && /\.avif$/i.test(file.name))) return 'AVIF isn’t accepted yet. Save it as PNG, WebP or JPEG.';
    return `${UPLOAD_FORMATS} only.`;
  }
  if (file.size > MAX_UPLOAD_BYTES) return `${sizeOver(file.size)}; at most 8 MB.`;
  return null;
}

/**
 * How far over the cap a file is, in the same one-decimal megabytes the queue row shows (`bytes`),
 * so the row and the message never disagree. A file that rounds to 8 MB itself is "just over".
 */
function sizeOver(size: number): string {
  const rounded = Math.round((size / (1024 * 1024)) * 10) / 10;
  return rounded <= 8 ? 'It’s just over 8 MB' : `It’s ${rounded.toFixed(1).replace('.', ',')} MB`;
}

/**
 * A typed tag as the API takes it: trimmed, lower case, no leading #, spaces as -, only letters,
 * digits, - and _, at most 32 characters. '' when nothing is left (the tag field drops it).
 */
export function cleanTag(raw: string): string {
  const t = raw.trim().replace(/^#+/, '').toLowerCase().replace(/\s+/g, '-').replace(/[^\p{L}\p{N}_-]/gu, '');
  return [...t].slice(0, TAG_LENGTH).join('');
}

/** About twice as wide as tall or more (1.75:1), as the API's `wide` flag is worked out. */
export function isWide(width: number, height: number): boolean {
  return height > 0 && width >= height * 1.75;
}

/** At most this many files in one upload batch: the API takes 20 uploads an hour per account. */
export const QUEUE_MAX = 20;

/** Two picked files that are the same file (same name, size and date), so a batch doesn't hold it twice. */
export function sameFile(a: Pick<File, 'name' | 'size' | 'lastModified'>, b: Pick<File, 'name' | 'size' | 'lastModified'>): boolean {
  return a.name === b.name && a.size === b.size && a.lastModified === b.lastModified;
}

/**
 * What an upload's refusal means, in words that say what to do. `code` is the API's error code;
 * `message` its own text, kept where it names the limit that was hit.
 */
export function uploadProblem(code: string, message: string): string {
  switch (code) {
    case 'blocked':
      return 'This image was taken down before (or is too close to one that was), so it can’t be uploaded again.';
    case 'too_large':
      return 'Too big: at most 8 MB, 1000 × 1000 px and 1000 frames.';
    case 'unsupported_format':
      return `Not a format cyex.tv takes: ${UPLOAD_FORMATS}.`;
    case 'invalid_image':
      return 'The file doesn’t open as an image; it may be damaged. Save it again and retry.';
    case 'rate_limited':
      return 'That’s 20 uploads in the last hour, the most there can be. The rest can go in an hour.';
    case 'restricted':
      return 'Your account can’t upload right now. Your notifications say why.';
    case 'signed_out':
      return 'You were signed out. Sign in again, then upload.';
    case 'bad_name':
    case 'bad_tag':
    case 'too_many_tags':
      return message;
    case 'offline':
      return unreachableMessage();
    case 'unavailable':
      return 'Uploads are paused for a moment. Try again in a minute.';
    default:
      return message || 'The upload didn’t go through.';
  }
}
