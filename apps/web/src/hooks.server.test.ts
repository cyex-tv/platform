import type { RequestEvent } from '@sveltejs/kit';
import { describe, expect, it } from 'vitest';
import { handle } from './hooks.server';

const run = (url: string, response: Response) =>
  handle({
    event: { url: new URL(url) } as RequestEvent,
    resolve: async () => response,
  } as Parameters<typeof handle>[0]) as Promise<Response>;

describe('hooks.server security headers', () => {
  it('sets the fixed headers on every response', async () => {
    const r = await run('http://localhost/', new Response('ok'));
    expect(r.headers.get('X-Frame-Options')).toBe('DENY');
    expect(r.headers.get('Content-Security-Policy')).toBe("frame-ancestors 'none'");
    expect(r.headers.get('X-Content-Type-Options')).toBe('nosniff');
    expect(r.headers.get('Referrer-Policy')).toBe('strict-origin-when-cross-origin');
    expect(r.headers.get('Permissions-Policy')).toBe('camera=(), microphone=(), geolocation=()');
  });

  it('leaves a header the route already set alone', async () => {
    const own = new Response('ok', { headers: { 'Content-Security-Policy': "default-src 'self'", 'X-Frame-Options': 'SAMEORIGIN' } });
    const r = await run('http://localhost/', own);
    expect(r.headers.get('Content-Security-Policy')).toBe("default-src 'self'");
    expect(r.headers.get('X-Frame-Options')).toBe('SAMEORIGIN');
    expect(r.headers.get('X-Content-Type-Options')).toBe('nosniff');
  });

  it('marks rendered HTML private, no-store, and leaves other responses and own values alone', async () => {
    const html = (h: Record<string, string> = {}) => new Response('<p>x</p>', { headers: { 'content-type': 'text/html; charset=utf-8', ...h } });
    expect((await run('http://localhost/', html())).headers.get('Cache-Control')).toBe('private, no-store');
    expect((await run('http://localhost/', html({ 'Cache-Control': 'max-age=60' }))).headers.get('Cache-Control')).toBe('max-age=60');
    const asset = new Response('x', { headers: { 'content-type': 'text/javascript' } });
    expect((await run('http://localhost/a.js', asset)).headers.get('Cache-Control')).toBeNull();
  });

  it('adds HSTS on https only, and keeps one a route set', async () => {
    expect((await run('http://localhost/', new Response('ok'))).headers.get('Strict-Transport-Security')).toBeNull();
    expect((await run('https://cyex.tv/', new Response('ok'))).headers.get('Strict-Transport-Security')).toBe('max-age=31536000');
    const own = new Response('ok', { headers: { 'Strict-Transport-Security': 'max-age=1' } });
    expect((await run('https://cyex.tv/', own)).headers.get('Strict-Transport-Security')).toBe('max-age=1');
  });
});
