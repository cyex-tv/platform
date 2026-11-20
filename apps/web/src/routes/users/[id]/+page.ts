import { ApiError, gql } from '$lib/api';
import type { Look } from '$lib/cosmetics/store.svelte';
import { missingMeta, profileMeta } from '$lib/meta';
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
      meta: accountProfile ? profileMeta(accountProfile, searchEmotes.total) : missingMeta(`/users/${encodeURIComponent(params.id)}`, 'No profile here'),
    };
  } catch (e) {
    const message = e instanceof ApiError ? e.message : 'This page can’t load right now.';
    return { id: params.id, profile: null, creator: null, look: null, items: [], total: 0, error: message, meta: missingMeta(`/users/${encodeURIComponent(params.id)}`, 'cyex.tv') };
  }
};
