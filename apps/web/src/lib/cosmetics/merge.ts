// Pure helpers of the by-account cosmetics cache (store.svelte.ts), kept apart so they are tested.
import type { Look } from './store.svelte';

export const MAX_ACCOUNTS = 100;

type Row = { accountId: string; badge?: Look['badge']; paint?: Look['paint'] };

/** The ids still to ask for: not known yet, not already waiting, each once, in order. */
export function toAsk(ids: readonly (string | null | undefined)[], known: ReadonlySet<string>): string[] {
  const out = new Set<string>();
  for (const id of ids) if (id && !known.has(id)) out.add(id);
  return [...out];
}

/** Batches of at most 100 ids, the API's limit. */
export function batches<T>(ids: readonly T[], size = MAX_ACCOUNTS): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < ids.length; i += size) out.push(ids.slice(i, i + size));
  return out;
}

/**
 * What was asked, as looks: every asked id gets one (an account the API sent nothing for, because it
 * shows nothing, gets an empty look so it is never asked again). Ids in `keep` are left out: a look
 * already known from the account's own selection is newer than this answer.
 */
export function mergeLooks(asked: readonly string[], rows: readonly Row[], keep: ReadonlySet<string> = new Set()): Record<string, Look> {
  const by = new Map(rows.map((r) => [r.accountId, r]));
  const out: Record<string, Look> = {};
  for (const id of asked) {
    if (keep.has(id)) continue;
    const r = by.get(id);
    out[id] = { badge: r?.badge ?? null, paint: r?.paint ?? null };
  }
  return out;
}
