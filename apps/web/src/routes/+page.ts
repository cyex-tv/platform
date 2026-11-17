import { gql } from '$lib/api';
import { GLOBAL_SET } from '$lib/queries';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  // The page stands without it: a down API only hides the preview.
  try {
    const { globalSet } = await gql(GLOBAL_SET, {}, fetch);
    return { global: globalSet.emotes.slice(0, 24) };
  } catch {
    return { global: [] };
  }
};
