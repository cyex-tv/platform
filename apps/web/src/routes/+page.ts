import { gql } from '$lib/api';
import { SEARCH } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  // One query feeds the hero cluster, the strip and the chat previews. The page stands
  // without it: a down API hides the art and the strip says so.
  // wire: sort - Top and New become searchEmotes(sort: TOP) and searchEmotes(sort: NEW).
  try {
    const { searchEmotes } = await gql(SEARCH, { query: '', animated: null, page: 1, perPage: 60 }, fetch);
    return { emotes: searchEmotes.items, total: searchEmotes.total, failed: false };
  } catch {
    return { emotes: [], total: 0, failed: true };
  }
};
