import { ApiError, gql } from '$lib/api';
import { COSMETICS } from '$lib/queries';
import type { PageLoad } from './$types';

// Every badge and paint (the public catalog). Who owns which is never part of it (ADR 0015).
export const load: PageLoad = async ({ fetch }) => {
  try {
    const { cosmetics } = await gql(COSMETICS, {}, fetch);
    return { badges: cosmetics.badges, paints: cosmetics.paints, error: null };
  } catch (e) {
    return { badges: [], paints: [], error: e instanceof ApiError ? e.message : 'The cosmetics didn’t load.' };
  }
};
