import { error } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import { channelsUsing, globalPool, moreBy } from '$lib/components/emote/data';
import type { PageEmote } from '$lib/components/emote/emote-page';
import { EMOTE } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
  let found: PageEmote | null;
  try {
    found = (await gql(EMOTE, { id: params.id }, fetch)).emote ?? null;
  } catch (e) {
    error(503, e instanceof ApiError ? e.message : 'cyex.tv can’t be reached right now.');
  }
  // The server asks without the session cookie, so pending, rejected and taken-down emotes come
  // back as nothing to it; the page asks again from the browser before it says
  // the emote isn't there.
  if (!found) return { id: params.id, emote: null, pool: [], more: [], channels: null };
  const [pool, more, channels] = await Promise.all([globalPool(fetch), moreBy(found, fetch), channelsUsing(found, fetch)]);
  return { id: params.id, emote: found, pool, more, channels };
};
