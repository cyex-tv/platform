import type { Handle } from '@sveltejs/kit';

/** Headers set on every response unless a route already set them. */
const HEADERS: Record<string, string> = {
  'X-Frame-Options': 'DENY',
  'Content-Security-Policy': "frame-ancestors 'none'",
  'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
  'Permissions-Policy': 'camera=(), microphone=(), geolocation=()',
};

export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);
  for (const [name, value] of Object.entries(HEADERS)) {
    if (!response.headers.has(name)) response.headers.set(name, value);
  }
  // Rendered pages may carry the visitor's data: shared caches must not keep them. Only HTML
  // that the server renders goes through here (prerendered files and assets are served before
  // this hook), and a route's own Cache-Control wins.
  if ((response.headers.get('content-type') ?? '').startsWith('text/html') && !response.headers.has('Cache-Control')) {
    response.headers.set('Cache-Control', 'private, no-store');
  }
  if (event.url.protocol === 'https:' && !response.headers.has('Strict-Transport-Security')) {
    response.headers.set('Strict-Transport-Security', 'max-age=31536000');
  }
  return response;
};
