import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// The footer links the legal pages; until their routes land, prerendering a page that links
// them must not fail the build. Every other broken link still does.
const PENDING = new Set(['/terms', '/privacy', '/contact']);

/** @type {import('@sveltejs/kit').Config} */
export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    prerender: {
      handleHttpError: ({ path, message }) => {
        if (PENDING.has(path)) return;
        throw new Error(message);
      },
    },
  },
};
