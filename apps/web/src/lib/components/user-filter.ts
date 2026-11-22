// The pure side of UserFilter: the person shape both lists use, the typed text as the people
// search reads it, the "most important" list put together, and the default search.
import { fetchWith, gql } from '$lib/api';
import { SEARCH_ACCOUNTS } from '$lib/queries';

/** One person a filter can pick: the account, the logins it signed in with, and (for the top
 * list) how many entries it has in the list being filtered. */
export type Person = {
  account: { id: string; displayName: string; avatar?: string | null };
  identities: { platform: string; login: string }[];
  count?: number;
};

/** People offered while typing, and the most important ones offered on an empty focus. */
export const SUGGESTIONS = 6;
export const TOP = 5;
/** The pause after the last key before the search goes out. */
export const DEBOUNCE = 150;

/** The text as the people search reads it: trimmed, a leading @ dropped, at most 100 chars. */
export function queryOf(text: string): string {
  return text.trim().replace(/^@+/, '').trim().slice(0, 100);
}

/** Searchable at all: the account search finds nobody under two characters. */
export const searchable = (q: string) => [...q].length >= 2;

/**
 * The most important people: `pinned` first (the profile's owner on their own profile), then the
 * counted ones, each person once, `max` at most. When the counted list is empty, `fallback` (the
 * context's editors or staff) fills in.
 */
export function topList(counted: Person[], pinned: Person[] = [], fallback: Person[] = [], max = TOP): Person[] {
  const out: Person[] = [];
  const seen = new Set<string>();
  const add = (p: Person) => {
    if (out.length >= max || seen.has(p.account.id)) return;
    seen.add(p.account.id);
    // A pinned person takes what the list knows of them (picture, logins, count).
    const known = counted.find((c) => c.account.id === p.account.id);
    out.push(known ? { ...p, ...known, count: p.count ?? known.count } : p);
  };
  pinned.forEach(add);
  counted.forEach(add);
  if (!counted.length) fallback.forEach(add);
  return out;
}

/** The next active row for an arrow key: wraps, and -1 (none) goes to the first or last. */
export function step(active: number, by: 1 | -1, length: number): number {
  if (length <= 0) return -1;
  if (active < 0) return by === 1 ? 0 : length - 1;
  return (active + by + length) % length;
}

/** "lumen_fox on Twitch" for a person's first login, or ''. */
export function loginHint(p: Person): string {
  const id = p.identities[0];
  if (!id) return '';
  return `${id.login} on ${id.platform === 'KICK' ? 'Kick' : 'Twitch'}`;
}

/** The default search: the site's account search (public people, blocked accounts left out). */
export async function searchPeople(query: string, signal: AbortSignal): Promise<Person[]> {
  const r = await gql(SEARCH_ACCOUNTS, { query, page: 1, perPage: SUGGESTIONS }, fetchWith(signal));
  return r.searchAccounts.items;
}
