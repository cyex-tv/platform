// The upload dialog's addresses and limits, pure (unit tested; the /upload and /me/upload
// redirects use them on the server). The dialog's state is upload.svelte.ts.

export const UPLOAD_PATHS = ['/me/upload', '/upload'];

/** What `uploadQuota` says about the files an account may upload (null: not read yet). */
export type EmoteCaps = { maxBytes: number; maxSide: number; maxFrames: number };

/** The built-in caps, shown until the API says its own (crates/media defaults). */
export const DEFAULT_EMOTE_CAPS: EmoteCaps = { maxBytes: 8 * 1024 * 1024, maxSide: 1000, maxFrames: 1000 };

const mb = (n: number) => String(Math.round((n / (1024 * 1024)) * 10) / 10).replace('.', ',');

/** cyex.tv's limits, listed under the drop zone: the account's own numbers once known. */
export function uploadLimits(caps: EmoteCaps | null | undefined = null): string[] {
  const c = caps ?? DEFAULT_EMOTE_CAPS;
  return [
    'PNG, APNG, GIF, WebP or JPEG',
    `Up to ${mb(c.maxBytes)} MB`,
    `Up to ${c.maxSide} × ${c.maxSide} px and ${c.maxFrames} frames`,
    'Made 32 to 128 px tall (1x to 4x)',
    'Wider than 4:1 is scaled to fit',
  ];
}

/** The built-in list (see `uploadLimits`). */
export const UPLOAD_LIMITS = uploadLimits();

type ClickLike = Pick<MouseEvent, 'button' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'defaultPrevented'>;
type LinkLike = { href: string; target?: string; hasAttribute(name: string): boolean };

/**
 * Whether a click on `link` should open the dialog instead of following it: a plain left click
 * (no modifier) on a same-site link to the old upload addresses. Its `?for=` is the account to
 * upload as (`uploadFor`).
 */
export function opensDialog(e: ClickLike, link: LinkLike, here: URL): boolean {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return false;
  if ((link.target && link.target !== '_self') || link.hasAttribute('download')) return false;
  const to = uploadUrl(link.href, here);
  return !!to;
}

/** The account a link to the old upload addresses asks to upload as (`?for=`), or null. */
export function uploadFor(href: string, here: URL): string | null {
  return uploadUrl(href, here)?.searchParams.get('for') || null;
}

function uploadUrl(href: string, here: URL): URL | null {
  let to: URL;
  try {
    to = new URL(href, here);
  } catch {
    return null;
  }
  return to.origin === here.origin && UPLOAD_PATHS.includes(to.pathname.replace(/\/$/, '')) ? to : null;
}

/**
 * Where /me/upload and /upload send you: back to the page you came from (`referer`, when it is
 * this site and not an upload address itself), else home, with `?upload` (the account to upload
 * as, or `1`) so the dialog opens there.
 */
export function uploadRedirect(url: URL, referer: string | null): string {
  let back = new URL('/', url);
  if (referer) {
    try {
      const r = new URL(referer);
      if (r.origin === url.origin && !UPLOAD_PATHS.includes(r.pathname.replace(/\/$/, ''))) back = r;
    } catch {
      /* not a URL: home */
    }
  }
  back.searchParams.set('upload', url.searchParams.get('for') || '1');
  back.hash = '';
  return back.pathname + back.search;
}

/** What `?upload` in an address asks for: null (nothing), or the account to upload as (null = you). */
export function uploadParam(url: URL): { for: string | null } | null {
  const v = url.searchParams.get('upload');
  if (v === null) return null;
  return { for: v && v !== '1' ? v : null };
}

/** The address without `?upload`, for replacing it once the dialog is open. */
export function withoutUploadParam(url: URL): string {
  const u = new URL(url);
  u.searchParams.delete('upload');
  return u.pathname + u.search + u.hash;
}
