// Talking to api.cyex.tv. The browser sends the session cookie itself (credentials: 'include';
// the API allows it for this site's origin). Server-side rendering only asks for public data.
import { env } from '$env/dynamic/public';
import { serverMessage, unreachableMessage } from './components/problem';
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
    throw new ApiError(unreachableMessage(), 'offline');
  }
  if (response.status === 429) throw new ApiError('Too many requests. Try again in a moment.', 'rate_limited');
  let body: { data?: TResult; errors?: { message: string; extensions?: { code?: string } }[] };
  try {
    body = await response.json();
  } catch {
    throw new ApiError(serverMessage(), 'bad_answer');
  }
  const first = body.errors?.[0];
  if (first) throw new ApiError(first.message, first.extensions?.code ?? 'error');
  if (!body.data) throw new ApiError(serverMessage(), 'bad_answer');
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
export function linkUrl(platform: 'twitch' | 'kick', back = '/me/settings/account'): string {
  return `${signInUrl(platform, back)}&link=true`;
}

/** A REST call's error: `{ error, message }` from the API, or a stand-in when it didn't answer. */
async function restError(response: Response): Promise<ApiError> {
  if (response.status === 429) return new ApiError('Too many requests. Try again in a moment.', 'rate_limited');
  try {
    const body = (await response.json()) as { error?: string; message?: string };
    return new ApiError(body.message ?? 'That didn’t work.', body.error ?? 'error');
  } catch {
    return new ApiError(serverMessage(), 'bad_answer');
  }
}

/** The upload's answer: the new emote, pending while its files are made. */
export type Uploaded = { id: string; name: string; state: string };

/** Who can find an emote (ADR 0020): public, unlisted (by link only) or private. */
export type EmoteVisibility = 'PUBLIC' | 'UNLISTED' | 'PRIVATE';

/** The flags an upload may set at once (all off, and public, when left out). */
export type UploadFlags = { zeroWidth: boolean; visibility: EmoteVisibility; nsfw: boolean };

/**
 * The multipart form of an upload: name, tags (comma-separated), the flags that are on, the
 * visibility unless public (a private one also sends the older `private` flag), the file.
 */
export function uploadForm(file: Blob, name: string, tags: string[], flags?: Partial<UploadFlags>): FormData {
  const form = new FormData();
  form.set('name', name);
  if (tags.length) form.set('tags', tags.join(','));
  if (flags?.zeroWidth) form.set('zero_width', 'true');
  if (flags?.visibility === 'PRIVATE') form.set('private', 'true');
  if (flags?.visibility && flags.visibility !== 'PUBLIC') form.set('visibility', flags.visibility.toLowerCase());
  if (flags?.nsfw) form.set('nsfw', 'true');
  form.set('file', file);
  return form;
}

/**
 * Uploads an emote (`POST /v1/emotes`, multipart). Throws `ApiError` with the API's code.
 * `progress` gets the share of bytes sent (0 to 1); XHR, since fetch can't report an upload's progress.
 */
export function uploadEmote(
  file: File,
  name: string,
  tags: string[],
  flags?: Partial<UploadFlags>,
  progress?: (share: number) => void,
): Promise<Uploaded> {
  const form = uploadForm(file, name, tags, flags);
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('POST', `${apiUrl()}/v1/emotes`);
    xhr.withCredentials = true;
    xhr.responseType = 'text';
    if (progress) xhr.upload.onprogress = (e) => e.lengthComputable && progress(e.loaded / e.total);
    xhr.onerror = () => reject(new ApiError(unreachableMessage(), 'offline'));
    xhr.onload = async () => {
      const response = new Response(xhr.responseText || null, { status: xhr.status });
      if (xhr.status < 200 || xhr.status >= 300) return reject(await restError(response));
      try {
        resolve(JSON.parse(xhr.responseText) as Uploaded);
      } catch {
        reject(new ApiError(serverMessage(), 'bad_answer'));
      }
    };
    xhr.send(form);
  });
}

/** Ends the session (`POST /v1/auth/logout`). */
export async function signOut(): Promise<void> {
  let response: Response;
  try {
    response = await fetch(`${apiUrl()}/v1/auth/logout`, { method: 'POST', credentials: 'include' });
  } catch {
    throw new ApiError(unreachableMessage(), 'offline');
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
    throw new ApiError(unreachableMessage(), 'offline');
  }
  if (!response.ok) throw await restError(response);
  return (await response.json()) as CosmeticUpload;
}
