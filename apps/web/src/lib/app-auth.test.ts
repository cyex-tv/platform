import { describe, expect, it, vi } from 'vitest';
import { answer, appClient, isLoopback, listener, readRequest } from './app-auth';

const query = (pairs: Record<string, string>) => new URLSearchParams(pairs);
const good = {
  client_id: 'cyex-deck',
  redirect_uri: 'http://127.0.0.1:53682/callback',
  code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM',
  code_challenge_method: 'S256',
  state: 'xyz',
};

describe('app sign-in', () => {
  it('takes loopback redirects only', () => {
    expect(isLoopback('http://127.0.0.1:53682/callback')).toBe(true);
    expect(isLoopback('http://[::1]:8080/callback')).toBe(true);
    for (const bad of [
      'https://127.0.0.1:53682/callback',
      'http://localhost:53682/callback',
      'http://127.0.0.1.evil.example/callback',
      'http://127.0.0.1:53682/other',
      'http://127.0.0.1:53682/callback?x=1',
      'http://u@127.0.0.1:53682/callback',
      'not a url',
    ]) {
      expect(isLoopback(bad), bad).toBe(false);
    }
  });

  it('reads the request from the query', () => {
    expect(readRequest(query(good))).toEqual({
      clientId: 'cyex-deck',
      redirectUri: good.redirect_uri,
      codeChallenge: good.code_challenge,
      codeChallengeMethod: 'S256',
      state: 'xyz',
    });
    expect(readRequest(query({ ...good, state: '' }))).toBeNull();
    expect(readRequest(query({ ...good, code_challenge_method: 'plain' }))).toBeNull();
    expect(readRequest(query({ ...good, redirect_uri: 'https://evil.example/callback' }))).toBeNull();
    expect(listener(good.redirect_uri)).toBe('127.0.0.1:53682');
  });

  it('knows unknown clients', async () => {
    const f = vi.fn(async () => new Response('{"error":"unknown_client","message":"x"}', { status: 404 }));
    expect(await appClient('nope', f as unknown as typeof fetch)).toBeNull();
  });

  it('only follows an answer that goes back to the loopback', async () => {
    const request = readRequest(query(good))!;
    const ok = vi.fn(async () => Response.json({ location: `${good.redirect_uri}?code=abc&state=xyz` }));
    expect(await answer(request, true, ok as unknown as typeof fetch)).toBe(`${good.redirect_uri}?code=abc&state=xyz`);
    const evil = vi.fn(async () => Response.json({ location: 'https://evil.example/?code=abc' }));
    await expect(answer(request, true, evil as unknown as typeof fetch)).rejects.toThrow();
    const out = vi.fn(async () => new Response('{"error":"signed_out","message":"Sign in"}', { status: 401 }));
    await expect(answer(request, true, out as unknown as typeof fetch)).rejects.toMatchObject({ code: 'signed_out' });
  });
});
