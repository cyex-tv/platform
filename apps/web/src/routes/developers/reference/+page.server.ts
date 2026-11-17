import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// The reference is part of /developers now. A redirect keeps the URL's #anchor, so deep links
// such as /developers/reference#rest-get_channel still land on their endpoint.
export const load: PageServerLoad = () => redirect(308, '/developers');
