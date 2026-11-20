// Recent searches in the header field: the last few queries sent from it, per device (localStorage,
// like the theme), newest first, no repeats. Storage can be missing or refuse writes (private
// windows, blocked site data): then nothing is remembered and nothing breaks.
export const RECENTS_KEY = 'cyex.search.recent';
export const RECENTS_MAX = 5;

/** The stored list, cleaned: strings only, trimmed, no repeats (any case), at most RECENTS_MAX. */
export function tidy(raw: unknown): string[] {
  if (!Array.isArray(raw)) return [];
  const seen = new Set<string>();
  const out: string[] = [];
  for (const v of raw) {
    if (typeof v !== 'string') continue;
    const t = v.trim().slice(0, 100);
    const k = t.toLowerCase();
    if (!t || seen.has(k)) continue;
    seen.add(k);
    out.push(t);
    if (out.length === RECENTS_MAX) break;
  }
  return out;
}

/** `list` with `text` first (an older copy of it dropped). */
export function push(list: readonly string[], text: string): string[] {
  return tidy([text, ...list]);
}

export function load(): string[] {
  try {
    return tidy(JSON.parse(localStorage.getItem(RECENTS_KEY) ?? '[]'));
  } catch {
    return [];
  }
}

/** Saves `list`; false when this browser would not take it (so the caller can show only what stuck). */
export function save(list: readonly string[]): boolean {
  try {
    if (list.length) localStorage.setItem(RECENTS_KEY, JSON.stringify(list));
    else localStorage.removeItem(RECENTS_KEY);
    return true;
  } catch {
    return false;
  }
}
