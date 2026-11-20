import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// The cosmetics gallery is gone (what exists is not public; what is for sale is in the shop).
export const load: PageLoad = () => {
  redirect(308, '/shop');
};
