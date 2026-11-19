// /upload: a short link to the upload page (which signs you in first when needed). On the site
// itself a link here opens the upload dialog instead ($lib/upload).
import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = ({ url }) => {
  redirect(308, `/me/upload${url.search}`);
};
