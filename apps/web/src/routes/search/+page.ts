import { ApiError, gql } from '$lib/api';
import { apiQuery, apiSort, PER_PAGE, parse, USERS_PER_PAGE } from '$lib/components/browse/params';
import { SEARCH, SEARCH_ACCOUNTS } from '$lib/queries';
import type { Browse } from '$lib/components/browse/params';
import type { FindPeopleQuery } from '$lib/gql/graphql';
import type { PageLoad } from './$types';

type People = FindPeopleQuery['searchAccounts']['items'];

/** The Users tab: people matching the name, as many pages as the address says (no name, nobody). */
async function loadUsers(view: Browse, fetch: typeof globalThis.fetch) {
  if (!view.q) return { view, total: 0, items: [], pages: 1, people: [] as People, error: null as string | null };
  const ask = (page: number) =>
    gql(SEARCH_ACCOUNTS, { query: view.q, page, perPage: USERS_PER_PAGE }, fetch).then((r) => r.searchAccounts);
  try {
    const first = await ask(1);
    const rest = Math.min(view.pages, Math.ceil(first.total / USERS_PER_PAGE)) - 1;
    const more = await Promise.all(Array.from({ length: Math.max(0, rest) }, (_, i) => ask(i + 2)));
    const people = [first, ...more].flatMap((r) => r.items);
    return { view, total: first.total, items: [], pages: 1 + more.length, people, error: null as string | null };
  } catch (e) {
    const error = e instanceof ApiError ? e.message : 'Search isn’t available right now.';
    return { view, total: 0, items: [], pages: 1, people: [] as People, error };
  }
}

export const load: PageLoad = async ({ fetch, url }) => {
  const view = parse(url.searchParams);
  if (view.tab === 'users') return loadUsers(view, fetch);
  const ask = (page: number) =>
    gql(
      SEARCH,
      {
        query: apiQuery(view),
        animated: view.kind === 'animated' ? true : view.kind === 'still' ? false : null,
        nsfw: view.nsfw ? true : null,
        sort: apiSort(view.sort),
        page,
        perPage: PER_PAGE,
      },
      fetch,
    ).then((r) => r.searchEmotes);
  try {
    // A shared link with n=3 renders three pages at once, so the view matches.
    const first = await ask(1);
    const rest = Math.min(view.pages, Math.ceil(first.total / PER_PAGE)) - 1;
    const more = await Promise.all(Array.from({ length: Math.max(0, rest) }, (_, i) => ask(i + 2)));
    const items = [first, ...more].flatMap((r) => r.items);
    return { view, total: first.total, items, pages: 1 + more.length, people: [] as People, error: null };
  } catch (e) {
    const error = e instanceof ApiError ? e.message : 'Search isn’t available right now.';
    return { view, total: 0, items: [], pages: 1, people: [] as People, error };
  }
};
