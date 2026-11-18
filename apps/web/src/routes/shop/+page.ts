import { ApiError, gql } from '$lib/api';
import { SHOP_ITEMS } from '$lib/queries';
import type { PageLoad } from './$types';

// What is on sale (public, signed out too). Who owns what comes in the browser, where the session is.
export const load: PageLoad = async ({ fetch }) => {
  try {
    const { shopItems } = await gql(SHOP_ITEMS, { all: false }, fetch);
    return { items: shopItems, error: null };
  } catch (e) {
    return { items: [], error: e instanceof ApiError ? e.message : 'The shop didn’t load.' };
  }
};
