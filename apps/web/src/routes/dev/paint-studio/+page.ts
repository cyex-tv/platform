// The paint studio on its own, for building and checking it. Development only: in a production
// build it's a 404. Saves go to PUBLIC_API_URL (the dev mock with MOCK_ADMIN=1).
import { error } from '@sveltejs/kit';
import { dev } from '$app/environment';
import type { PageLoad } from './$types';

export const ssr = false;

export const load: PageLoad = ({ url }) => {
  if (!dev) error(404, 'Not found');
  return { paintId: url.searchParams.get('id') };
};
