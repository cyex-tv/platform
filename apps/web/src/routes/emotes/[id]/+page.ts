import { error } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import { EMOTE } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
  let found;
  try {
    found = (await gql(EMOTE, { id: params.id }, fetch)).emote;
  } catch (e) {
    error(503, e instanceof ApiError ? e.message : 'cyex.tv can’t be reached right now.');
  }
  // Pending, rejected and taken-down emotes aren't public: the same answer as one that never was.
  if (!found) error(404, 'No emote here.');
  return { emote: found };
};
