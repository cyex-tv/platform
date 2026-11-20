import { error } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import { PUBLIC_SET } from '$lib/queries';
import { missingMeta, setMeta } from '$lib/meta';
import { unreachableMessage } from '$lib/components/problem';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
  try {
    const { set } = await gql(PUBLIC_SET, { id: params.id }, fetch);
    // The server asks without the session cookie, so a private set answers as nothing to it;
    // the page asks again from the browser before it says the set isn't there.
    const meta = set ? setMeta(set) : missingMeta(`/sets/${encodeURIComponent(params.id)}`, 'No set here');
    return { id: params.id, set: set ?? null, meta };
  } catch (e) {
    error(503, e instanceof ApiError ? e.message : unreachableMessage());
  }
};
