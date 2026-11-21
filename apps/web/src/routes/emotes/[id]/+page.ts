import { error, redirect } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import { channelsUsing, moreBy, trendingOf } from '$lib/components/emote/data';
import type { PageEmote } from '$lib/components/emote/emote-page';
import { legacyRedirect } from '$lib/ids';
import { EMOTE } from '$lib/queries';
import { emoteMeta, missingMeta } from '$lib/meta';
import { unreachableMessage } from '$lib/components/problem';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params, url }) => {
  let found: PageEmote | null;
  try {
    found = (await gql(EMOTE, { id: params.id }, fetch)).emote ?? null;
  } catch (e) {
    error(503, e instanceof ApiError ? e.message : unreachableMessage());
  }
  // The server asks without the session cookie, so pending, rejected and taken-down emotes come
  // back as nothing to it; the page asks again from the browser before it says
  // the emote isn't there.
  if (!found) return { id: params.id, emote: null, more: [], channels: null, trending: null, meta: missingMeta(`/emotes/${encodeURIComponent(params.id)}`, 'No emote here') };
  // An old link with the emote's ULID goes to its snowflake address (the query stays; the browser keeps the #hash).
  const moved = legacyRedirect('emotes', params.id, found.id, url.search);
  if (moved) redirect(308, moved);
  const [more, channels, trending] = await Promise.all([
    moreBy(found, fetch),
    channelsUsing(found, fetch),
    trendingOf(found, fetch),
  ]);
  return { id: params.id, emote: found, more, channels, trending, meta: emoteMeta(found) };
};
