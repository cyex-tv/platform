import { graphqlReference } from '$lib/docs/graphql';
import { baseUrl, restGroups } from '$lib/docs/rest';
import type { PageServerLoad } from './$types';

// Built once, at build time, from openapi/openapi.json and schema/schema.graphql. The page is
// static: it makes no request to the API, and the GraphQL parser stays out of the browser bundle.
export const prerender = true;

export const load: PageServerLoad = () => ({
  footer: 'slim' as const, // the root layout shows the one-line footer here, so the scroll ends at the last section
  skipTo: 'api-contents', // the second skip link: past the index, to the guides and reference
  base: baseUrl,
  rest: restGroups(),
  gql: graphqlReference(),
});
