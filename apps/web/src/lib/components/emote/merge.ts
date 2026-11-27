// Pure helpers for "Merge into another emote" (ADR 0040): which emote a pasted link or ID means,
// the plain-words answer for each refusal, and the summary line for the toast.

const SNOWFLAKE = /^[1-9]\d{0,18}$/;
const ULID = /^[0-9A-HJKMNP-TV-Z]{26}$/i;

/**
 * The emote ID in what was typed or pasted: a bare ID (Snowflake digits or a ULID), or an emote page
 * link on any origin (`https://cyex.tv/emotes/<id>?tab=activity#top`, or just `/emotes/<id>`).
 * The query and the hash are ignored. Null when it is neither.
 */
export function mergeTarget(text: string): string | null {
  const t = text.trim();
  if (!t) return null;
  if (SNOWFLAKE.test(t) || ULID.test(t)) return t;
  const m = /(?:^|\/)emotes\/([^/?#\s]+)/i.exec(t);
  if (!m) return null;
  let id: string;
  try {
    id = decodeURIComponent(m[1]);
  } catch {
    return null;
  }
  return SNOWFLAKE.test(id) || ULID.test(id) ? id : null;
}

/** Where a merged emote's page goes: the target's page, with the query string kept. */
export const mergedPath = (target: string, search = ''): string => `/emotes/${encodeURIComponent(target)}${search}`;

const WORDS: Record<string, string> = {
  merge_into_self: 'That is this emote. Pick a different one to merge into.',
  merge_target_unavailable: 'That emote can’t take it: it is deleted, waiting for review, rejected or taken down. Pick an approved emote.',
  already_merged: 'This emote was merged into another already, so it can’t be merged again.',
  not_found: 'No emote with that link or ID.',
  forbidden: 'Merging emotes needs the emotes.merge permission.',
};

/** What a refusal means, in plain words; the server's own message for any other code. */
export function mergeProblem(code: string | undefined, fallback: string): string {
  return (code && WORDS[code]) || fallback || 'That didn’t work.';
}

/** "Merged Pogg into Pog: 12 sets moved, 1 already had it." */
export function mergeSummary(source: string, target: string, moved: number, collapsed: number): string {
  const sets = (n: number) => `${n} ${n === 1 ? 'set' : 'sets'}`;
  const bits = [moved ? `${sets(moved)} moved` : 'no sets to move'];
  if (collapsed) bits.push(`${collapsed} already had it`);
  return `Merged ${source} into ${target}: ${bits.join(', ')}.`;
}
