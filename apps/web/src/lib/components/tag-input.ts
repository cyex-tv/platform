// TagInput's rules, pure so they are unit tested: what typing, pasting, Enter and Backspace do
// to the list of tags.

export type TagRules = {
  /** Turns typed text into a tag ('' drops it). Default: trimmed, lower case, no leading #. */
  clean?: (raw: string) => string;
  /** At most this many tags. */
  max?: number;
};

export const defaultClean = (raw: string) => raw.trim().replace(/^#+/, '').toLowerCase();

/** Splits typed or pasted text at commas and new lines (spaces stay inside a tag until cleaned). */
export function pieces(raw: string): string[] {
  return raw.split(/[,\n\r]+/);
}

export type Added = {
  tags: string[];
  /** Tags that were already there. */
  repeated: string[];
  /** Tags left out because the list is full. */
  over: string[];
};

/** `tags` with the tags in `raw` added at the end: cleaned, without empties or repeats, up to `max`. */
export function addTags(tags: readonly string[], raw: string, rules: TagRules = {}): Added {
  const clean = rules.clean ?? defaultClean;
  const max = rules.max ?? Infinity;
  const out = [...tags];
  const repeated: string[] = [];
  const over: string[] = [];
  for (const piece of pieces(raw)) {
    const t = clean(piece);
    if (!t) continue;
    if (out.includes(t)) repeated.push(t);
    else if (out.length >= max) over.push(t);
    else out.push(t);
  }
  return { tags: out, repeated, over };
}

/** `tags` without the one at `i`. */
export function removeAt(tags: readonly string[], i: number): string[] {
  return tags.filter((_, j) => j !== i);
}

/**
 * What a key in the text field does. Enter (and a comma) adds what is typed; Backspace in an
 * empty field takes the last tag off; anything else types.
 */
export function keyAction(key: string, text: string, count: number): 'add' | 'pop' | 'none' {
  if ((key === 'Enter' || key === ',') && text.trim()) return 'add';
  if (key === 'Backspace' && text === '' && count > 0) return 'pop';
  return 'none';
}
