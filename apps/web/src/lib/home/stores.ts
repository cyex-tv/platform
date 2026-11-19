// Where each browser gets the cyex.tv extension. The one place to change: once a store listing
// is live, put its URL in `url` (e.g. 'https://chromewebstore.google.com/detail/…'). Until then
// each entry points at its part of /download, which explains the test build.

export type StoreKey = 'chrome' | 'edge' | 'firefox' | 'opera' | 'brave';

export type Store = { key: StoreKey; browser: string; store: string; url: string };

export const STORES: readonly Store[] = [
  { key: 'chrome', browser: 'Chrome', store: 'Chrome Web Store', url: '/download#chrome' },
  { key: 'edge', browser: 'Edge', store: 'Edge Add-ons', url: '/download#edge' },
  { key: 'firefox', browser: 'Firefox', store: 'Firefox Add-ons', url: '/download#firefox' },
  { key: 'opera', browser: 'Opera', store: 'Opera add-ons', url: '/download#opera' },
  // Brave installs from the Chrome Web Store: give it the Chrome listing once there is one.
  { key: 'brave', browser: 'Brave', store: 'Chrome Web Store', url: '/download#brave' },
];

/** Is this a real store listing yet (an outside https URL), rather than the /download fallback? */
export const listed = (s: Store) => /^https:\/\//.test(s.url);

/**
 * The browser this page is open in, when it can take the extension; null on phones and others.
 * `brave` is `'brave' in navigator` (Brave's user agent looks like Chrome's).
 */
export function storeFor(ua: string, brave = false): StoreKey | null {
  if (/iPhone|iPad|iPod|Android/.test(ua)) return null; // no desktop extensions there
  if (/Firefox\//.test(ua)) return 'firefox';
  if (/Edg\//.test(ua)) return 'edge';
  if (/OPR\//.test(ua)) return 'opera';
  if (brave) return 'brave';
  if (/Chrome\/|Chromium\//.test(ua)) return 'chrome';
  return null;
}
