import { ApiError, gql } from '$lib/api';
import { channelsOf, lookupAccount, type Creator } from '$lib/components/profile/wire';
import { SEARCH_BY_OWNER } from '$lib/queries';
import type { PageLoad } from './$types';

const PER_PAGE = 60;

export const load: PageLoad = async ({ fetch, params }) => {
  try {
    const [{ searchEmotes }, account, channels] = await Promise.all([
      gql(SEARCH_BY_OWNER, { owner: params.id, page: 1, perPage: PER_PAGE, sort: 'TOP' }, fetch),
      lookupAccount(params.id, fetch),
      channelsOf(params.id),
    ]);
    const items = searchEmotes.items;
    // No account but emotes: the name comes from the first emote's owner. Neither: not found.
    const creator: Creator | null = account ?? items[0]?.owner ?? null;
    return { id: params.id, creator, items, total: searchEmotes.total, channels, error: null as string | null };
  } catch (e) {
    const message = e instanceof ApiError ? e.message : 'This page can’t load right now.';
    return { id: params.id, creator: null, items: [], total: 0, channels: [], error: message };
  }
};
