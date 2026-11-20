import { baseUrl } from '$lib/docs/rest';
import { navGroups, pageFacts } from '$lib/docs/lazy.server';
import type { PageServerLoad } from './$types';

// Built once, at build time, from openapi/openapi.json and schema/schema.graphql. The page is
// static: it makes no request to the API. It carries the guides and the index only; a closed
// group's body is its own prerendered JSON (groups/[id].json), fetched when the group is opened.
export const prerender = true;

export const load: PageServerLoad = () => ({
  footer: 'slim' as const, // the root layout shows the one-line footer here, so the scroll ends at the last section
  skipTo: 'api-contents', // the second skip link: past the index, to the guides and reference
  base: baseUrl,
  nav: navGroups(),
  ...pageFacts(),
});
