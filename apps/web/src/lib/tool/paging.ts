// "Show more" lists: a late answer for an old filter must not land in the new list, and a row that
// is already shown must not be added again (a keyed each throws on a duplicate id).

/** `base` then the rows of `more` that are not in it yet, each id once, first one wins. */
export function appendUnique<T extends { id: string }>(base: readonly T[], more: readonly T[]): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const row of [...base, ...more]) {
    if (seen.has(row.id)) continue;
    seen.add(row.id);
    out.push(row);
  }
  return out;
}

/**
 * A counter for "the list I asked for". `begin()` before a request returns whether that request
 * is still the newest; `reset()` (the filter changed, the list reloaded) makes every older one stale.
 */
export function pageToken() {
  let n = 0;
  return {
    reset(): void {
      n++;
    },
    begin(): () => boolean {
      const mine = ++n;
      return () => mine === n;
    },
  };
}
