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

/** `back` when it is a path on this site; otherwise `/`. Rejects `//host` and backslash forms that browsers read as another host. */
export function safeBack(back: string): string {
  return back.startsWith('/') && !back.startsWith('//') && !back.includes('\\') ? back : '/';
}

/** The address that starts signing in with a platform, and comes back to `back`. */
export function signInUrl(platform: 'twitch' | 'kick', back = '/'): string {
  const returnTo = encodeURIComponent(safeBack(back));
  return `${apiUrl()}/v1/auth/${platform}/login?return_to=${returnTo}`;
}

/** The address that links another platform to the signed-in account, and comes back to `back`. */
export function linkUrl(platform: 'twitch' | 'kick', back = '/me/account'): string {
  return `${signInUrl(platform, back)}&link=true`;
}

/** A REST call's error: `{ error, message }` from the API, or a stand-in when it didn't answer. */
async function restError(response: Response): Promise<ApiError> {
  if (response.status === 429) return new ApiError('Too many requests. Try again in a moment.', 'rate_limited');
  try {
    const body = (await response.json()) as { error?: string; message?: string };
    return new ApiError(body.message ?? 'That didn’t work.', body.error ?? 'error');
  } catch {
    return new ApiError('cyex.tv answered in a way the site didn’t expect.', 'bad_answer');
  }
}

/** The upload's answer: the new emote, pending while its files are made. */
export type Uploaded = { id: string; name: string; state: string };

/** Uploads an emote (`POST /v1/emotes`, multipart). Throws `ApiError` with the API's code. */
export async function uploadEmote(file: File, name: string, tags: string[]): Promise<Uploaded> {
  const form = new FormData();
  form.set('name', name);
  if (tags.length) form.set('tags', tags.join(','));
  form.set('file', file);
  let response: Response;
  try {
    response = await fetch(`${apiUrl()}/v1/emotes`, { method: 'POST', credentials: 'include', body: form });
  } catch {
    throw new ApiError('cyex.tv can’t be reached right now.', 'offline');
  }
  if (!response.ok) throw await restError(response);
  return (await response.json()) as Uploaded;
}

/** Ends the session (`POST /v1/auth/logout`). */
export async function signOut(): Promise<void> {
  let response: Response;
  try {
    response = await fetch(`${apiUrl()}/v1/auth/logout`, { method: 'POST', credentials: 'include' });
  } catch {
    throw new ApiError('cyex.tv can’t be reached right now.', 'offline');
  }
  if (!response.ok && response.status !== 401) throw await restError(response);
}

/** A badge or paint picture's answer: not ready until the worker made its files. */
export type CosmeticUpload = { id: string; kind: 'BADGE' | 'PAINT'; ready: boolean };

/** Uploads a badge or paint picture (`POST /v1/cosmetics/images`, staff). Poll `cosmeticImage(id)` until ready. */
export async function uploadCosmeticImage(file: File, kind: 'badge' | 'paint'): Promise<CosmeticUpload> {
  const form = new FormData();
  form.set('kind', kind);
  form.set('file', file);
  let response: Response;
  try {
    response = await fetch(`${apiUrl()}/v1/cosmetics/images`, { method: 'POST', credentials: 'include', body: form });
  } catch {
    throw new ApiError('cyex.tv can’t be reached right now.', 'offline');
  }
  if (!response.ok) throw await restError(response);
  return (await response.json()) as CosmeticUpload;
}
