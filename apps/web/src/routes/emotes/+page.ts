import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// /emotes is /search, query string kept.
export const load: PageLoad = ({ url }) => {
  redirect(308, `/search${url.search}`);
};
