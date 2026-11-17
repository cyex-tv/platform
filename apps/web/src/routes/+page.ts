import { gql } from '$lib/api';
import { SEARCH } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  // Top feeds the hero cluster, the chat previews and the Top strip; New is its own query for the
  // New strip. The page stands without them: a down API hides the art and the strip says so.
  const ask = (sort: 'TOP' | 'NEW') =>
    gql(SEARCH, { query: '', animated: null, page: 1, perPage: 60, sort }, fetch).then((r) => r.searchEmotes);
  try {
    const [top, fresh] = await Promise.all([ask('TOP'), ask('NEW').catch(() => null)]);
    return { emotes: top.items, fresh: fresh?.items ?? top.items, total: top.total, failed: false };
  } catch {
    return { emotes: [], fresh: [], total: 0, failed: true };
  }
};
