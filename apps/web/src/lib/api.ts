// Talking to api.cyex.tv. The browser sends the session cookie itself (credentials: 'include';
// the API allows it for this site's origin). Server-side rendering only asks for public data.
import { env } from '$env/dynamic/public';
import type { TypedDocumentString } from './gql/graphql';

/** Where the API is: `PUBLIC_API_URL`, or production. */
export function apiUrl(): string {
  return (env.PUBLIC_API_URL || 'https://api.cyex.tv').replace(/\/+$/, '');
}

/** The site's own address, for sign-in return links. */
export function siteUrl(): string {
  return (env.PUBLIC_SITE_URL || 'https://cyex.tv').replace(/\/+$/, '');
}

/** A GraphQL error, with the code the API sets (`signed_out`, `forbidden`, `not_found`, ...). */
export class ApiError extends Error {
  constructor(
    message: string,
    readonly code: string,
  ) {
    super(message);
  }
}

type Fetch = typeof fetch;

/** Runs a query or mutation. Throws `ApiError` for the first GraphQL error. */
export async function gql<TResult, TVariables>(
  doc: TypedDocumentString<TResult, TVariables>,
  variables: TVariables,
  fetcher: Fetch = fetch,
): Promise<TResult> {
  let response: Response;
  try {
    response = await fetcher(`${apiUrl()}/v1/gql`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ query: doc.toString(), variables }),
    });
  } catch {
    throw new ApiError('cyex.tv can’t be reached right now.', 'offline');
  }
  if (response.status === 429) throw new ApiError('Too many requests. Try again in a moment.', 'rate_limited');
  let body: { data?: TResult; errors?: { message: string; extensions?: { code?: string } }[] };
  try {
    body = await response.json();
  } catch {
    throw new ApiError('cyex.tv answered in a way the site didn’t expect.', 'bad_answer');
  }
  const first = body.errors?.[0];
  if (first) throw new ApiError(first.message, first.extensions?.code ?? 'error');
  if (!body.data) throw new ApiError('cyex.tv sent no data.', 'bad_answer');
  return body.data;
}

/** The address that starts signing in with a platform, and comes back to `back`. */
export function signInUrl(platform: 'twitch' | 'kick', back = '/'): string {
  const returnTo = encodeURIComponent(back.startsWith('/') ? back : '/');
  return `${apiUrl()}/v1/auth/${platform}/login?return_to=${returnTo}`;
}
