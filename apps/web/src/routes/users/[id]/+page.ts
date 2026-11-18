import { ApiError, gql } from '$lib/api';
import type { Look } from '$lib/cosmetics/store.svelte';
import { ACCOUNT_COSMETICS, ACCOUNT_PROFILE, SEARCH_BY_OWNER } from '$lib/queries';
import type { PageLoad } from './$types';

const PER_PAGE = 60;

export const load: PageLoad = async ({ fetch, params }) => {
  try {
    const [{ searchEmotes }, { accountProfile }, { accountCosmetics }] = await Promise.all([
      gql(SEARCH_BY_OWNER, { owner: params.id, page: 1, perPage: PER_PAGE, sort: 'TOP' }, fetch),
      gql(ACCOUNT_PROFILE, { accountId: params.id }, fetch),
      gql(ACCOUNT_COSMETICS, { accountIds: [params.id] }, fetch),
    ]);
    // Unknown, deleted or blocked: the API has no profile, so there is no page (whatever emotes remain).
    const shown = accountCosmetics.find((c) => c.accountId === params.id);
    const look: Look = { badge: shown?.badge ?? null, paint: shown?.paint ?? null };
    return {
      id: params.id,
      profile: accountProfile ?? null,
      creator: accountProfile?.account ?? null,
      look,
      items: searchEmotes.items,
      total: searchEmotes.total,
      error: null as string | null,
    };
  } catch (e) {
    const message = e instanceof ApiError ? e.message : 'This page can’t load right now.';
    return { id: params.id, profile: null, creator: null, look: null, items: [], total: 0, error: message };
  }
};
