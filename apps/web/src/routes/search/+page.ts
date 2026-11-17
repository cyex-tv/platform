import { ApiError, gql } from '$lib/api';
import { PER_PAGE, parse } from '$lib/components/browse/params';
import { SEARCH } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, url }) => {
  const view = parse(url.searchParams);
  // wire: sort (searchEmotes(sort: RELEVANCE | TOP | NEW) is not in the schema yet; every order is today's)
  const ask = (page: number) =>
    gql(
      SEARCH,
      {
        query: view.q,
        animated: view.kind === 'animated' ? true : view.kind === 'still' ? false : null,
        nsfw: view.nsfw ? true : null,
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
    return { view, total: first.total, items, pages: 1 + more.length, error: null };
  } catch (e) {
    const error = e instanceof ApiError ? e.message : 'Search isn’t available right now.';
    return { view, total: 0, items: [], pages: 1, error };
  }
};
