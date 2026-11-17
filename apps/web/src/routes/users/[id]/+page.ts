import { ApiError, gql } from '$lib/api';
import { channelsOf, lookupAccount, type Creator } from '$lib/components/profile/wire';
import { SEARCH_BY_OWNER } from '$lib/queries';
import type { PageLoad } from './$types';

const PER_PAGE = 60;

export const load: PageLoad = async ({ fetch, params }) => {
  try {
    const [{ searchEmotes }, account, channels] = await Promise.all([
      gql(SEARCH_BY_OWNER, { owner: params.id, page: 1, perPage: PER_PAGE }, fetch),
      lookupAccount(params.id),
      channelsOf(params.id),
    ]);
    // The API only returns this owner's emotes; the filter keeps the page honest if it ever doesn't.
    const items = searchEmotes.items.filter((e) => e.owner?.id === params.id);
    // wire: account (until then the name comes from the first emote's owner)
    const creator: Creator | null = account ?? items[0]?.owner ?? null;
    return { id: params.id, creator, items, total: searchEmotes.total, channels, error: null as string | null };
  } catch (e) {
    const message = e instanceof ApiError ? e.message : 'This page can’t load right now.';
    return { id: params.id, creator: null, items: [], total: 0, channels: [], error: message };
  }
};
