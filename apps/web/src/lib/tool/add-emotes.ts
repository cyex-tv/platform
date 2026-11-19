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

/** Your matching uploads, then the public results, each emote once. */
export function mergeResults<T extends Found>(text: string, own: readonly T[], everyone: readonly T[], personal: boolean): T[] {
  const mine = own.filter((e) => ownMatches(text, e, personal));
  const seen = new Set(mine.map((e) => e.id));
  return [...mine, ...everyone.filter((e) => !seen.has(e.id))];
}
