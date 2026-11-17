import { ApiError, gql } from '$lib/api';
import { SEARCH } from '$lib/queries';
import type { PageLoad } from './$types';

const PER_PAGE = 60;

export const load: PageLoad = async ({ fetch, url }) => {
  const query = url.searchParams.get('q')?.trim() ?? '';
  const kind = url.searchParams.get('kind');
  const animated = kind === 'animated' ? true : kind === 'still' ? false : null;
  const page = Math.max(1, Number.parseInt(url.searchParams.get('page') ?? '1', 10) || 1);
  try {
    const { searchEmotes } = await gql(SEARCH, { query, animated, page, perPage: PER_PAGE }, fetch);
    return { query, kind: kind ?? 'all', results: searchEmotes, error: null };
  } catch (e) {
    const message = e instanceof ApiError ? e.message : 'Search isn’t available right now.';
    return { query, kind: kind ?? 'all', results: null, error: message };
  }
};
