// Sign-out semantics (owner rule, 2026/11/22): the account menu's Sign out ends only this
// session (POST /v1/auth/logout, which deletes the caller's own session token in the API); ending
// every session is the one "Sign out everywhere" action on your profile's Account tab.
import { afterEach, describe, expect, it, vi } from 'vitest';

const signOut = vi.hoisted(() => vi.fn(async () => {}));
const gql = vi.hoisted(() => vi.fn(async () => ({})));
const goto = vi.hoisted(() => vi.fn(async () => {}));

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto, invalidateAll: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), signOut, gql, onSignedOut: () => () => {} }));

afterEach(() => vi.clearAllMocks());

describe('Sign out (the account menu)', () => {
  it('ends this session only: one logout call, no sign-out-everywhere or revoke mutation', async () => {
    const { leave } = await import('./signout');
    await leave();
    expect(signOut).toHaveBeenCalledTimes(1);
    const sent = gql.mock.calls.map((c) => String((c as unknown[])[0]));
    expect(sent.some((q) => /signOutEverywhere|revokeSession|revokeOtherSessions/.test(q))).toBe(false);
    expect(goto).toHaveBeenCalledWith('/', { invalidateAll: true });
  });

  it('stays put when the logout call fails', async () => {
    signOut.mockRejectedValueOnce(new Error('offline'));
    const { leave } = await import('./signout');
    await leave();
    expect(goto).not.toHaveBeenCalled();
  });
});

describe('signOut (the REST call)', () => {
  it('posts to /v1/auth/logout with the session cookie', async () => {
    const { signOut: real } = await vi.importActual<typeof import('$lib/api')>('$lib/api');
    const fetch = vi.fn(async () => new Response(null, { status: 204 }));
    vi.stubGlobal('fetch', fetch);
    try {
      await real();
    } finally {
      vi.unstubAllGlobals();
    }
    expect(fetch).toHaveBeenCalledTimes(1);
    const [url, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toMatch(/\/v1\/auth\/logout$/);
    expect(init).toMatchObject({ method: 'POST', credentials: 'include' });
  });
});
