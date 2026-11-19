// Which cyex deck installer the front page's download button points at, picked the way the
// cyex.app/deck page picks it: the newest release of github.com/cyex-app/cyex-deck that has a
// `_x64-setup.exe`, pre-releases counted (every release so far is one, and GitHub's
// /releases/latest skips them, so the list is read instead). Pure, so it's tested without a browser;
// the live part is deck.svelte.ts.

/** GitHub's release list for the deck. Ten is plenty: only the newest one with an installer matters. */
export const API = 'https://api.github.com/repos/cyex-app/cyex-deck/releases?per_page=10';
/** Where the deck's release files live; nothing else is accepted as a download. */
export const DOWNLOADS = 'https://github.com/cyex-app/cyex-deck/releases/download/';
/** The same cache key and shape the cyex.app pages write (each origin keeps its own copy). */
export const KEY = 'cyexDeckRelease';
/** The deck's release page, for the "all versions" link. */
export const RELEASES_PAGE = 'https://github.com/cyex-app/cyex-deck/releases';

export type DeckRelease = { url: string; version: string; prerelease: boolean };

/** The offline fallback: the newest installer when this was written (move it with each release). */
export const PINNED: Readonly<DeckRelease> = Object.freeze({
  url: 'https://github.com/cyex-app/cyex-deck/releases/download/v0.1.09/cyex-deck_0.1.9_x64-setup.exe',
  version: '0.1.09',
  prerelease: true,
});

const INSTALLER = /_x64-setup\.exe$/;
const PATH = new URL(DOWNLOADS).pathname;

/** Is this a setup exe served from the deck repo's own release files? Checked on the parsed URL. */
export function isInstaller(url: unknown): url is string {
  if (typeof url !== 'string') return false;
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  if (u.protocol !== 'https:' || u.host !== 'github.com' || u.username || u.password) return false;
  if (!u.pathname.startsWith(PATH) || !INSTALLER.test(u.pathname)) return false;
  // new URL() resolves `..`, so a climbing path is looked for in the text itself (plain and encoded).
  let text: string;
  try {
    text = decodeURIComponent(url);
  } catch {
    return false;
  }
  return !text.split(/[/\\?#]/).includes('..');
}

// A release tag: digits and dots, then an optional short pre-release tag. Anything else isn't put on a button.
const TAG = /^v?\d+(\.\d+){0,3}(-[0-9A-Za-z.]{1,20})?$/;

type Asset = { name?: unknown; browser_download_url?: unknown };
type Release = { tag_name?: unknown; draft?: unknown; prerelease?: unknown; published_at?: unknown; assets?: unknown };

const when = (r: Release) => (typeof r.published_at === 'string' ? Date.parse(r.published_at) || 0 : 0);

/** The newest release that isn't a draft and has an installer, version without its v. Null if none. */
export function pick(releases: unknown): DeckRelease | null {
  if (!Array.isArray(releases)) return null;
  const sorted = (releases as Release[])
    .filter((r) => r && typeof r === 'object' && r.draft !== true && typeof r.tag_name === 'string' && TAG.test(r.tag_name))
    .sort((a, b) => when(b) - when(a));
  for (const r of sorted) {
    const assets = (Array.isArray(r.assets) ? r.assets : []) as Asset[];
    // The first asset that is an installer by name and by address; a lookalike doesn't hide a good one.
    const hit = assets.find((a) => a && typeof a.name === 'string' && INSTALLER.test(a.name) && isInstaller(a.browser_download_url));
    if (hit) return { url: hit.browser_download_url as string, version: (r.tag_name as string).replace(/^v/, ''), prerelease: r.prerelease === true };
  }
  return null;
}

const segments = (v: string) => (v.match(/\d+/g) ?? []).map(Number);

function compare(x: number[], y: number[]): number {
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d) return d;
  }
  return 0;
}

/** Is version a newer than b? '0.1.07' equals '0.1.7', '0.1.10' beats '0.1.9', a tagged version is older than its plain one. */
export function newer(a: string, b: string): boolean {
  const [coreA, ...tagA] = String(a ?? '').replace(/^v/, '').split('-');
  const [coreB, ...tagB] = String(b ?? '').replace(/^v/, '').split('-');
  const x = segments(coreA);
  const y = segments(coreB);
  if (!x.length || !y.length) return false;
  const core = compare(x, y);
  if (core) return core > 0;
  if (!tagA.length || !tagB.length) return !tagA.length && tagB.length > 0;
  return compare(segments(tagA.join('-')), segments(tagB.join('-'))) > 0;
}

export type Kept = DeckRelease & { at: number };

/** A kept answer in the shape the cyex.app pages write, or null. The URL gets the same check as a fresh one. */
export function validCache(x: unknown): Kept | null {
  if (!x || typeof x !== 'object') return null;
  const { at, url, version, prerelease } = x as Record<string, unknown>;
  if (typeof at !== 'number' || !Number.isFinite(at)) return null;
  if (!isInstaller(url) || typeof version !== 'string' || typeof prerelease !== 'boolean') return null;
  return { at, url, version, prerelease };
}
