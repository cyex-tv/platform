import { error, redirect } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import { legacyRedirect } from '$lib/ids';
import { PUBLIC_SET } from '$lib/queries';
import { missingMeta, setMeta } from '$lib/meta';
import { unreachableMessage } from '$lib/components/problem';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params, url }) => {
  let found;
  try {
    found = (await gql(PUBLIC_SET, { id: params.id }, fetch)).set ?? null;
  } catch (e) {
    error(503, e instanceof ApiError ? e.message : unreachableMessage());
  }
  // An old link with the set's ULID goes to its snowflake address (the query stays; the browser keeps the #hash).
  const moved = legacyRedirect('sets', params.id, found?.id, url.search);
  if (moved) redirect(308, moved);
  // The server asks without the session cookie, so a private set answers as nothing to it;
  // the page asks again from the browser before it says the set isn't there.
  const meta = found ? setMeta(found) : missingMeta(`/sets/${encodeURIComponent(params.id)}`, 'No set here');
  return { id: params.id, set: found, meta };
};
