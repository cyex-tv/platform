// The API's rules for names, tags and uploads, checked in the browser first so a mistake shows
// next to the field. The API checks them again; these only save a round trip.

import { unreachableMessage } from './components/problem';

export const MAX_UPLOAD_BYTES = 5 * 1024 * 1024;
export const UPLOAD_TYPES = ['image/png', 'image/apng', 'image/gif', 'image/webp'];

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
  if (tags.length > 10) return 'At most 10 tags.';
  const bad = tags.find((t) => [...t].length > 32 || !/^[\p{L}\p{N}_-]+$/u.test(t));
  return bad ? `“${bad}” isn’t a tag: 1 to 32 letters, digits, - or _.` : null;
}

/** A file's problem for upload, or null: PNG, APNG, GIF or WebP, at most 5 MB. */
export function fileProblem(file: Pick<File, 'type' | 'size'>): string | null {
  if (!UPLOAD_TYPES.includes(file.type)) return 'PNG, APNG, GIF or WebP only.';
  if (file.size > MAX_UPLOAD_BYTES) return 'At most 5 MB.';
  return null;
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
      return 'Too big: at most 5 MB, 1000 × 1000 px and 1000 frames.';
    case 'unsupported_format':
      return 'Not a format cyex.tv takes: PNG, APNG, GIF or WebP.';
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
