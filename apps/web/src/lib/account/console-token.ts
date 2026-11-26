// Your API token from the browser console (ADR 0028). The site's own session is an HttpOnly
// cookie that no script can read, and stays one. Instead, signed in, the page offers
// `cyextv.token()`: it asks the API for a new personal Bearer token (a console token) and hands
// it back, so `copy(await cyextv.token())` puts it on the clipboard. Nothing here keeps it: not
// in localStorage, sessionStorage, a cookie or a variable. Making one signs the last one out.
// Your profile → Account shows the live token's expiry and makes a new one to show (ADR 0029).
import { ApiError, apiUrl } from '$lib/api';
import { serverMessage, unreachableMessage } from '$lib/components/problem';
import { clock, date } from '$lib/format';

export const EXPORT_PATH = '/v1/auth/token/export';

export const HINT =
  'cyex.tv: copy(await cyextv.token()) copies a personal API token. Anyone with it can act as you; revoke it in your account.';

/** A new console token and when it stops working (RFC 3339). */
export type MadeToken = { token: string; expiresAt: string };

/**
 * Asks the API for a new console token. The previous one stops working. It lasts 3 days and
 * rotates whenever a session signs out (ADR 0029); the API never shows it again.
 */
export async function makeToken(fetcher: typeof fetch = fetch): Promise<MadeToken> {
  let response: Response;
  try {
    response = await fetcher(`${apiUrl()}${EXPORT_PATH}`, { method: 'POST', credentials: 'include', cache: 'no-store' });
  } catch {
    throw new ApiError(unreachableMessage(), 'offline');
  }
  let body: { token?: unknown; expires_at?: unknown; error?: string; message?: string } | null = null;
  try {
    body = await response.json();
  } catch {
    /* handled below */
  }
  if (!response.ok) {
    if (response.status === 429) throw new ApiError(body?.message ?? 'Too many requests. Try again later.', 'rate_limited');
    throw new ApiError(body?.message ?? serverMessage(), body?.error ?? 'error');
  }
  if (typeof body?.token !== 'string' || !body.token) throw new ApiError(serverMessage(), 'bad_answer');
  return { token: body.token, expiresAt: typeof body.expires_at === 'string' ? body.expires_at : '' };
}

/** Asks the API for a new console token and returns it. The previous one stops working. */
export async function exportToken(fetcher: typeof fetch = fetch): Promise<string> {
  return (await makeToken(fetcher)).token;
}

/** When a token expires, as `2026/11/26, 14:05` in the reader's time zone; '' if unknown. */
export function expiryText(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = date(iso);
  return d ? `${d}, ${clock(iso)}` : '';
}

type ConsoleWindow = { cyextv?: { token?: () => Promise<string> } };

let hinted = false;

/**
 * Offers `cyextv.token()` while someone is signed in, and takes it away when they aren't. The
 * hint is printed once per page load, the first time a signed-in session is seen.
 */
export function offerConsoleToken(
  signedIn: boolean,
  win: ConsoleWindow = globalThis as unknown as ConsoleWindow,
  fetcher?: typeof fetch,
  log: (message: string) => void = (m) => console.info(m),
) {
  if (!signedIn) {
    if (win.cyextv) delete win.cyextv.token;
    return;
  }
  const token = () => exportToken(fetcher ?? fetch);
  win.cyextv = { ...win.cyextv, token };
  if (!hinted) {
    hinted = true;
    log(HINT);
  }
}

/** For tests: the next signed-in page load prints the hint again. */
export function resetHint() {
  hinted = false;
}
