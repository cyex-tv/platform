import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';
import { ApiError, gql } from '$lib/api';
import type { Look } from '$lib/cosmetics/store.svelte';
import { legacyRedirect } from '$lib/ids';
import { missingMeta, profileMeta } from '$lib/meta';
import { preloadFor } from '$lib/profile/sections/lazy.svelte';
import { ACCOUNT_COSMETICS, ACCOUNT_PROFILE, SEARCH_BY_OWNER } from '$lib/queries';
import type { PageLoad } from './$types';

const PER_PAGE = 60;

export const load: PageLoad = async ({ fetch, params, url, untrack }) => {
  // The Collection tab is gone: its old links open Cosmetics. Read without tracking, so switching
  // tabs (?s=) never loads the page again.
  if (untrack(() => url.searchParams.get('s')) === 'collection') {
    redirect(308, `/users/${encodeURIComponent(params.id)}?s=cosmetics`);
  }
  // The section the address asks for loads with the page, so it renders at once (server and browser).
  const preload = preloadFor(untrack(() => url.searchParams.get('s')), !browser);
  let loaded;
  try {
    loaded = await Promise.all([
      gql(SEARCH_BY_OWNER, { owner: params.id, page: 1, perPage: PER_PAGE, sort: 'TOP' }, fetch),
      gql(ACCOUNT_PROFILE, { accountId: params.id }, fetch),
      gql(ACCOUNT_COSMETICS, { accountIds: [params.id] }, fetch),
    ]);
  } catch (e) {
    const message = e instanceof ApiError ? e.message : 'This page can’t load right now.';
    return { id: params.id, profile: null, creator: null, look: null, items: [], total: 0, error: message, meta: missingMeta(`/users/${encodeURIComponent(params.id)}`, 'cyex.tv') };
  }
  const [{ searchEmotes }, { accountProfile }, { accountCosmetics }] = loaded;
  await preload;
  // An old link with the account's ULID goes to its snowflake address (the query stays; the browser keeps the #hash).
  const moved = legacyRedirect('users', params.id, accountProfile?.account.id, url.search);
  if (moved) redirect(308, moved);
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
};
