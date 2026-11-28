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
// Where emote, badge and paint pictures are served (the API's CYEX_TV_CDN_URL), when it isn't
// https://cdn.cyex.tv: a local stack's S3 server, say. Without it the CSP blocks those pictures,
// and a painted name drawn from a blocked picture used to vanish.
const cdn = env.PUBLIC_CDN_URL ? [new URL(env.PUBLIC_CDN_URL).origin] : [];
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

// The shop's checkout (Paddle.js, loaded only when someone starts a purchase; ADR 0016). Paddle
// publishes no list of hosts for a Content Security Policy, so these were found by loading Paddle.js
// and opening a checkout with this policy on, in both environments, and allowing exactly what it
// asked for: the script from cdn.paddle.com (https://developer.paddle.com/paddlejs/include-paddlejs),
// its overlay stylesheet from cdn.paddle.com (live) or sandbox-cdn.paddle.com (sandbox), and the
// checkout itself in a frame from buy.paddle.com (live) or sandbox-buy.paddle.com (sandbox). The
// page never makes a request of its own to Paddle, so connect-src is unchanged. The API says which
// environment is in use, so both are allowed.
const PADDLE_SCRIPT = ['https://cdn.paddle.com/paddle/v2/paddle.js'];
const PADDLE_STYLE = ['https://cdn.paddle.com', 'https://sandbox-cdn.paddle.com'];
const PADDLE_FRAME = ['https://buy.paddle.com', 'https://sandbox-buy.paddle.com'];

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
        'script-src': ['self', ...inline, 'unsafe-hashes', replay, ...PADDLE_SCRIPT],
        // Inline style attributes (Svelte's style: directives and static style="" in templates)
        // need 'unsafe-inline'; <style> elements don't, so they get their own, stricter list.
        'style-src': ['self', 'unsafe-inline'],
        'style-src-elem': ['self', ...PADDLE_STYLE],
        'style-src-attr': ['unsafe-inline'],
        'font-src': ['self'],
        'img-src': ['self', 'data:', 'blob:', 'https://cdn.cyex.tv', ...cdn, ...AVATARS, api, ...mock],
        // GitHub's release list: the front page reads which cyex deck installer is newest (src/lib/home/deck.svelte.ts).
        // The CDN: an emote's Download and GIF export fetch the file (lib/components/emote/download.ts).
        'connect-src': ['self', api, 'https://cdn.cyex.tv', ...cdn, 'https://api.github.com', ...mock],
        'frame-src': ['self', ...PADDLE_FRAME],
        // The paint studio renders animated paints in a module worker; the dev server hands it
        // over as a blob: URL (without this, script-src blocked it and rendering fell back to the page).
        'worker-src': ['self', 'blob:'],
        // The emote editor plays a video it can't decode itself in a <video> from a blob: URL
        // (the file never leaves the browser).
        'media-src': ['self', 'blob:'],
        'form-action': ['self'],
        'base-uri': ['self'],
        'object-src': ['none'],
        'frame-ancestors': ['none'],
      },
    },
    prerender: {
      // The reference's operations are drawn when their group opens, so a link to one finds no element in the
      // prerendered HTML; the index names every one of them (the docs tests check each link target exists).
      handleMissingId: ({ path, id, message }) => {
        if (path === '/developers' && /^(rest|gql|ws)-/.test(id)) return;
        throw new Error(message);
      },
      handleHttpError: ({ path, message }) => {
        if (PENDING.has(path)) return;
        throw new Error(message);
      },
    },
  },
};
