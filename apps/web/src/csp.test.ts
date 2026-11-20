import { describe, expect, it } from 'vitest';
// The CSP lives in svelte.config.js (SvelteKit sends it); these checks keep its rules from drifting.
// Loaded by a computed path so the type checker doesn't hold the config to this file's looser types.
const path = '../svelte.config.js';
const config = (await import(/* @vite-ignore */ path)).default as { kit: { csp: { mode: string; directives: Record<string, string[]> } } };

const csp = config.kit.csp.directives;

describe('Content-Security-Policy', () => {
  it('is on, and falls back to the site itself', () => {
    expect(config.kit.csp.mode).toBe('auto');
    expect(csp['default-src']).toEqual(['self']);
  });

  it('allows connections only to the site, the API and GitHub (and the dev mock in dev)', () => {
    const allowed = csp['connect-src'].filter((s) => s !== 'self');
    expect(csp['connect-src'][0]).toBe('self');
    expect(allowed).toContain('https://api.github.com');
    for (const origin of allowed) expect(origin).toMatch(/^https?:\/\/[^*]+$/); // no wildcards
  });

  it('allows pictures from the CDN, the API (passed-through avatars), the platforms and data/blob URLs', () => {
    expect(csp['img-src']).toEqual(
      expect.arrayContaining(['self', 'data:', 'blob:', 'https://cdn.cyex.tv', 'https://static-cdn.jtvnw.net', 'https://files.kick.com']),
    );
    expect(csp['img-src']).not.toContain('*');
  });

  it('never allows eval, a bare https: source, or wildcard scripts', () => {
    for (const [name, sources] of Object.entries(csp)) {
      expect(sources, name).not.toContain('unsafe-eval');
      expect(sources, name).not.toContain('*');
      expect(sources, name).not.toContain('https:');
    }
    expect(csp['script-src']).not.toContain('unsafe-inline');
    expect(csp['script-src'].filter((s) => s.startsWith('sha256-')).length).toBeGreaterThan(1); // the theme script and the image replay handler
    expect(csp['script-src']).toContain('https://cdn.paddle.com/paddle/v2/paddle.js');
  });

  it('closes the usual holes: no frames of the site, no plugins, no foreign forms or base', () => {
    expect(csp['frame-ancestors']).toEqual(['none']);
    expect(csp['object-src']).toEqual(['none']);
    expect(csp['form-action']).toEqual(['self']);
    expect(csp['base-uri']).toEqual(['self']);
    expect(csp['frame-src']).toEqual(['self', 'https://buy.paddle.com', 'https://sandbox-buy.paddle.com']);
  });
});
