import { graphqlReference } from '$lib/docs/graphql';
import { baseUrl, restGroups } from '$lib/docs/rest';
import type { PageServerLoad } from './$types';

// Built once, at build time, from openapi/openapi.json and schema/schema.graphql. The page is
// static: it makes no request to the API, and the GraphQL parser stays out of the browser bundle.
export const prerender = true;

export const load: PageServerLoad = () => ({
  // The docs bring their own header and footer (cyex-docs); the root layout leaves its out.
  chrome: false as const,
  base: baseUrl,
  rest: restGroups(),
  gql: graphqlReference(),
});
