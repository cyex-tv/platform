import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { loadEnv } from 'vite';

// The footer links the legal pages; until their routes land, prerendering a page that links
// them must not fail the build. Every other broken link still does.
const PENDING = new Set(['/terms', '/privacy', '/contact']);

// `vite dev` (or plain `vite`); `vite build` and `vite preview` are not dev.
const dev = !process.argv.includes('build') && !process.argv.includes('preview');

// The CSP is fixed when the site is built, so it allows the API origin `PUBLIC_API_URL` has at
// build time (from the environment or a .env file; production when unset). A build for another
// API origin must set PUBLIC_API_URL at build time too (the Dockerfile passes it as a build arg).
const env = { ...loadEnv(dev ? 'development' : 'production', process.cwd(), 'PUBLIC_'), ...process.env };
const api = new URL(env.PUBLIC_API_URL || 'https://api.cyex.tv').origin;
// The dev mock (dev/mock-api.mjs) on its default port, in dev only.
const mock = dev ? ['http://localhost:8787'] : [];

// The inline theme script in app.html runs before first paint, on SSR and prerendered pages
// alike, so it is allowed by its hash (a nonce placeholder would block prerendering). The hash
// is taken from the file, so editing the script keeps it allowed.
const template = readFileSync(new URL('./src/app.html', import.meta.url), 'utf8');
const inline = [...template.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(
  ([, body]) => /** @type {const} */ (`sha256-${createHash('sha256').update(body).digest('base64')}`),
);

// Svelte's server render puts `onload="this.__e=event"` / `onerror="this.__e=event"` on images
// with load or error handlers, so an event that fires before hydration is replayed. 'unsafe-hashes'
// with that one hash allows exactly this handler and no other inline handler.
const replay = /** @type {const} */ (`sha256-${createHash('sha256').update('this.__e=event').digest('base64')}`);

// Hosts of the profile pictures the API passes through from Twitch and Kick.
const AVATARS = ['https://static-cdn.jtvnw.net', 'https://files.kick.com'];

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    // 'auto': nonces on server-rendered pages, hashes on prerendered ones (a <meta> tag there,
    // which can't carry frame-ancestors; Cloudflare adds it, see docs/setup/owner-steps.md).
    csp: {
      mode: 'auto',
      directives: {
        'default-src': ['self'],
        'script-src': ['self', ...inline, 'unsafe-hashes', replay],
        // Inline style attributes (Svelte's style: directives and static style="" in templates)
        // need 'unsafe-inline'; <style> elements don't, so they get their own, stricter list.
        'style-src': ['self', 'unsafe-inline', 'https://fonts.googleapis.com'],
        'style-src-elem': ['self', 'https://fonts.googleapis.com'],
        'style-src-attr': ['unsafe-inline'],
        'font-src': ['self', 'https://fonts.gstatic.com'],
        'img-src': ['self', 'data:', 'blob:', 'https://cdn.cyex.tv', ...AVATARS, api, ...mock],
        'connect-src': ['self', api, ...mock],
        'form-action': ['self'],
        'base-uri': ['self'],
        'object-src': ['none'],
        'frame-ancestors': ['none'],
      },
    },
    prerender: {
      handleHttpError: ({ path, message }) => {
        if (PENDING.has(path)) return;
        throw new Error(message);
      },
    },
  },
};
