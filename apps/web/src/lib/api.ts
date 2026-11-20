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

// Who wants to know when a call answers `signed_out` (the session ended while the page was open).
const signedOutListeners = new Set<() => void>();
/** Calls `fn` whenever any API call answers `signed_out`. Returns a function that stops listening. */
export function onSignedOut(fn: () => void): () => void {
  signedOutListeners.add(fn);
  return () => signedOutListeners.delete(fn);
}
/** Tells the listeners when `err` is a `signed_out`, and hands it back for `throw`. */
function seen(err: ApiError): ApiError {
  if (err.code === 'signed_out') for (const fn of [...signedOutListeners]) fn();
  return err;
}

/** How long a GraphQL call may take before it counts as unanswered. */
export const GQL_TIMEOUT_MS = 20_000;
/** An upload that sends nothing for this long has stalled. */
export const UPLOAD_STALL_MS = 30_000;
/** After the last byte is sent, the API gets this long to answer (it checks the picture, a second or two), as long as a stall. */
export const UPLOAD_ANSWER_MS = 30_000;

/**
 * A fetch that also stops when `outer` does, and keeps the call's own deadline (the one `gql`
 * sets): a plain `{ ...init, signal: outer }` would replace that deadline and let a call hang.
 */
export function fetchWith(outer: AbortSignal, base: Fetch = fetch): Fetch {
  return (input, init) => base(input, { ...init, signal: init?.signal ? AbortSignal.any([outer, init.signal]) : outer });
}

/** Runs a query or mutation. Throws `ApiError` for the first GraphQL error. */
export async function gql<TResult, TVariables>(
  doc: TypedDocumentString<TResult, TVariables>,
  variables: TVariables,
  fetcher: Fetch = fetch,
): Promise<TResult> {
  let response: Response;
  // One deadline for the whole call, body included: a server that never answers ends in the
  // shared "cyex.tv didn't answer" problem with Try again, not in a skeleton that never goes.
  const signal = AbortSignal.timeout(GQL_TIMEOUT_MS);
  try {
    response = await fetcher(`${apiUrl()}/v1/gql`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ query: doc.toString(), variables }),
      signal,
    });
  } catch {
    if (signal.aborted) throw new ApiError(unreachableMessage(), 'timeout');
    throw new ApiError(unreachableMessage(), 'offline');
  }
  if (response.status === 429) throw new ApiError('Too many requests. Try again in a moment.', 'rate_limited');
  let body: { data?: TResult; errors?: { message: string; extensions?: { code?: string } }[] };
  try {
    body = await response.json();
  } catch {
    if (signal.aborted) throw new ApiError(unreachableMessage(), 'timeout');
    throw new ApiError(serverMessage(), 'bad_answer');
  }
  const first = body.errors?.[0];
  if (first) throw seen(new ApiError(first.message, first.extensions?.code ?? 'error'));
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
    return seen(new ApiError(body.message ?? 'That didn’t work.', body.error ?? 'error'));
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
export function uploadForm(file: Blob, name: string, tags: string[], flags?: Partial<UploadFlags>, ownerId?: string | null): FormData {
  const form = new FormData();
  form.set('name', name);
  // Upload as (ADR 0014): an account you edit with Upload owns it. Left out for your own.
  if (ownerId) form.set('owner_id', ownerId);
  if (tags.length) form.set('tags', tags.join(','));
  if (flags?.zeroWidth) form.set('zero_width', 'true');
  if (flags?.visibility === 'PRIVATE') form.set('private', 'true');
  if (flags?.visibility && flags.visibility !== 'PUBLIC') form.set('visibility', flags.visibility.toLowerCase());
  if (flags?.nsfw) form.set('nsfw', 'true');
  form.set('file', file);
  return form;
}

/** What `uploadEmote` takes besides the file and fields. */
export type UploadOptions = {
  /** Aborts the upload: it rejects with code `aborted`. */
  signal?: AbortSignal;
  /** No progress for this long (default `UPLOAD_STALL_MS`) rejects with code `stalled`. */
  stallMs?: number;
  /** Waiting for the answer after the last byte (default `UPLOAD_ANSWER_MS`); also `stalled`. */
  answerMs?: number;
};

/**
 * Uploads an emote (`POST /v1/emotes`, multipart). Throws `ApiError` with the API's code.
 * `progress` gets the share of bytes sent (0 to 1); XHR, since fetch can't report an upload's
 * progress. It never hangs: no progress for `stallMs` ends it (`stalled`), and `signal` cancels it (`aborted`).
 */
export function uploadEmote(
  file: File,
  name: string,
  tags: string[],
  flags?: Partial<UploadFlags>,
  progress?: (share: number) => void,
  ownerId?: string | null,
  options: UploadOptions = {},
): Promise<Uploaded> {
  const form = uploadForm(file, name, tags, flags, ownerId);
  const { signal, stallMs = UPLOAD_STALL_MS, answerMs = UPLOAD_ANSWER_MS } = options;
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(new ApiError(CANCELLED, 'aborted'));
    const xhr = new XMLHttpRequest();
    let timer: ReturnType<typeof setTimeout> | undefined;
    let settled = false;
    const end = (fail?: ApiError, ok?: Uploaded) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      signal?.removeEventListener('abort', onAbort);
      if (fail) {
        xhr.abort();
        reject(fail);
      } else resolve(ok!);
    };
    const watch = (ms: number) => {
      clearTimeout(timer);
      timer = setTimeout(() => end(new ApiError(STALLED, 'stalled')), ms);
    };
    const onAbort = () => end(new ApiError(CANCELLED, 'aborted'));
    signal?.addEventListener('abort', onAbort);
    xhr.open('POST', `${apiUrl()}/v1/emotes`);
    xhr.withCredentials = true;
    xhr.responseType = 'text';
    xhr.upload.onprogress = (e) => {
      watch(stallMs);
      if (progress && e.lengthComputable) progress(e.loaded / e.total);
    };
    xhr.upload.onload = () => watch(answerMs);
    xhr.onerror = () => end(new ApiError(unreachableMessage(), 'offline'));
    xhr.onload = async () => {
      if (settled) return;
      clearTimeout(timer);
      const response = new Response(xhr.responseText || null, { status: xhr.status });
      if (xhr.status < 200 || xhr.status >= 300) return end(await restError(response));
      try {
        end(undefined, JSON.parse(xhr.responseText) as Uploaded);
      } catch {
        end(new ApiError(serverMessage(), 'bad_answer'));
      }
    };
    watch(stallMs);
    xhr.send(form);
  });
}

/** The words for an upload that went quiet, and for one the sender stopped. */
export const STALLED = 'The upload stalled. Check your connection, then try again.';
export const CANCELLED = 'Cancelled.';

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

/**
 * `POST /v1/cosmetics/images`'s answer, in the REST shape (snake_case, lower-case kind). Only the
 * ID is used: the picture is always read back through GraphQL (`cosmeticImage`), because bytes the
 * API already has come back at once, ready, and a REST file list isn't the GraphQL one.
 */
export type CosmeticUpload = { id: string; kind: 'badge' | 'paint'; ready: boolean; created_at?: string };

/** Uploads a badge or paint picture (`POST /v1/cosmetics/images`, staff). Poll `cosmeticImage(id)` until ready. */
export async function uploadCosmeticImage(file: File, kind: 'badge' | 'paint', signal?: AbortSignal): Promise<CosmeticUpload> {
  const form = new FormData();
  form.set('kind', kind);
  form.set('file', file);
  let response: Response;
  try {
    response = await fetch(`${apiUrl()}/v1/cosmetics/images`, { method: 'POST', credentials: 'include', body: form, signal });
  } catch {
    if (signal?.aborted) throw new ApiError('Stopped.', 'aborted');
    throw new ApiError(unreachableMessage(), 'offline');
  }
  if (response.status === 429) throw new ApiError('Too many uploads. Try again in a moment.', 'rate_limited');
  if (!response.ok) throw await restError(response);
  return (await response.json()) as CosmeticUpload;
}
