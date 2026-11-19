// App sign-in (ADR 0021): the consent page's side. An app such as cyex deck opens
// `/v1/auth/app/authorize`, the API checks the client and the loopback redirect and sends the
// browser here with the same request; Allow or Deny posts it back and the answer says where to go.
import { ApiError, apiUrl } from './api';
import { serverMessage, unreachableMessage } from './components/problem';

/** The authorize request, as it arrives in the page's query. */
export type AppRequest = {
  clientId: string;
  redirectUri: string;
  codeChallenge: string;
  codeChallengeMethod: string;
  state: string;
};

export type AppClient = { clientId: string; name: string; scopes: { id: string; description: string }[] };

/**
 * `http://127.0.0.1:<port>/callback` or `http://[::1]:<port>/callback`, as the API takes them.
 * The API checks again; this only spares a round trip for a link that can't work.
 */
export function isLoopback(raw: string): boolean {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return false;
  }
  return (
    url.protocol === 'http:' &&
    (url.hostname === '127.0.0.1' || url.hostname === '[::1]') &&
    url.pathname === '/callback' &&
    !url.username &&
    !url.password &&
    !url.search &&
    !url.hash
  );
}

/** The request from the query, or null when a part is missing or the redirect isn't loopback. */
export function readRequest(params: URLSearchParams): AppRequest | null {
  const clientId = params.get('client_id') ?? '';
  const redirectUri = params.get('redirect_uri') ?? '';
  const codeChallenge = params.get('code_challenge') ?? '';
  const codeChallengeMethod = params.get('code_challenge_method') ?? '';
  const state = params.get('state') ?? '';
  if (!clientId || !codeChallenge || codeChallengeMethod !== 'S256' || !state || !isLoopback(redirectUri)) return null;
  return { clientId, redirectUri, codeChallenge, codeChallengeMethod, state };
}

/** Where the app listens, for people: `127.0.0.1:53682`. */
export function listener(redirectUri: string): string {
  try {
    return new URL(redirectUri).host;
  } catch {
    return '';
  }
}

async function errorOf(response: Response): Promise<ApiError> {
  if (response.status === 429) return new ApiError('Too many requests. Try again in a moment.', 'rate_limited');
  try {
    const body = (await response.json()) as { error?: string; message?: string };
    return new ApiError(body.message ?? serverMessage(), body.error ?? 'error');
  } catch {
    return new ApiError(serverMessage(), 'bad_answer');
  }
}

/** The app's name and what it asks for. `null` for an unknown client. */
export async function appClient(clientId: string, fetcher: typeof fetch = fetch): Promise<AppClient | null> {
  let response: Response;
  try {
    response = await fetcher(`${apiUrl()}/v1/auth/app/clients/${encodeURIComponent(clientId)}`);
  } catch {
    throw new ApiError(unreachableMessage(), 'offline');
  }
  if (response.status === 404) return null;
  if (!response.ok) throw await errorOf(response);
  return (await response.json()) as AppClient;
}

/** Allow or deny. Resolves to where to send the browser (the app's loopback address). */
export async function answer(request: AppRequest, allow: boolean, fetcher: typeof fetch = fetch): Promise<string> {
  let response: Response;
  try {
    response = await fetcher(`${apiUrl()}/v1/auth/app/consent`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ ...request, allow }),
    });
  } catch {
    throw new ApiError(unreachableMessage(), 'offline');
  }
  if (!response.ok) throw await errorOf(response);
  const body = (await response.json()) as { location?: string };
  if (!body.location || !isLoopback(body.location.split('?')[0])) throw new ApiError(serverMessage(), 'bad_answer');
  return body.location;
}
