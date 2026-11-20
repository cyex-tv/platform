// The set editor's "Add emotes" results: your own uploads matching the search first, then
// everyone's, without repeats.
type Found = { id: string; name: string; state: string; tags: readonly string[] };

/** Whether one of your uploads matches what was typed: its name or a tag contains it. */
export function ownMatches(text: string, e: Found, personal: boolean): boolean {
  const q = text.trim().toLowerCase();
  if (!q) return false;
  // A personal set takes approved emotes only; other sets take your pending ones too.
  if (!(e.state === 'APPROVED' || (e.state === 'PENDING' && !personal))) return false;
  return e.name.toLowerCase().includes(q) || e.tags.some((t) => t.toLowerCase().includes(q));
}

/** A name for an emote whose own name is taken in the set: the name with 2, 3, … on it, the first free one (at most 100 characters). */
export function freeName(name: string, taken: ReadonlySet<string>): string {
  const chars = [...name];
  for (let n = 2; n < 1000; n++) {
    const tail = String(n);
    const next = chars.slice(0, 100 - tail.length).join('') + tail;
    if (!taken.has(next)) return next;
  }
  return name;
}

/** Your matching uploads, then the public results, each emote once. */
export function mergeResults<T extends Found>(text: string, own: readonly T[], everyone: readonly T[], personal: boolean): T[] {
  const mine = own.filter((e) => ownMatches(text, e, personal));
  const seen = new Set(mine.map((e) => e.id));
  return [...mine, ...everyone.filter((e) => !seen.has(e.id))];
}

/** The results so far with the next page of public ones after them, each emote once. */
export function appendResults<T extends { id: string }>(current: readonly T[], next: readonly T[]): T[] {
  const seen = new Set(current.map((e) => e.id));
  return [...current, ...next.filter((e) => !seen.has(e.id))];
}

/** Public results per page. */
export const RESULT_PAGE = 40;

/** "Showing 40 of 213": what the list holds of what the search found; null when it holds all. */
export function showingLine(fetched: number, total: number, count: (n: number) => string): string | null {
  if (!(total > fetched)) return null;
  return `Showing ${count(fetched)} of ${count(total)}.`;
}
