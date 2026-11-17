// A page of every shared component with live data, for building and checking pages.
// Development only: in a production build it's a 404.
import { error } from '@sveltejs/kit';
import { dev } from '$app/environment';
import { gql } from '$lib/api';
import { SEARCH } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  if (!dev) error(404, 'Not found');
  try {
    const { searchEmotes } = await gql(SEARCH, { query: '', animated: null, page: 1, perPage: 36 }, fetch);
    return { emotes: searchEmotes.items };
  } catch {
    return { emotes: [] };
  }
};
