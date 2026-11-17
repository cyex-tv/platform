// The API's rules for names, tags and uploads, checked in the browser first so a mistake shows
// next to the field. The API checks them again; these only save a round trip.

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
