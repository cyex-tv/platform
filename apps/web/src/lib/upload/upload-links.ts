// The upload dialog's addresses and limits, pure (unit tested; the /upload and /me/upload
// redirects use them on the server). The dialog's state is upload.svelte.ts.

export const UPLOAD_PATHS = ['/me/upload', '/upload'];

/** cyex.tv's own limits, listed under the drop zone (crates/media; the API checks them again). */
export const UPLOAD_LIMITS = [
  'PNG, APNG, GIF, WebP or JPEG',
  'Up to 8 MB',
  'Up to 1000 × 1000 px and 1000 frames',
  'Made 32 to 128 px tall (1x to 4x)',
  'Wider than 4:1 is scaled to fit',
] as const;

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
