// The upload dialog, open from anywhere: the header's Upload button, an empty "Your emotes", any
// link to /me/upload. One dialog for the whole site lives in the root layout; its queue stays
// while you browse, so closing it and opening it again finds your files where you left them.
// /me/upload itself still renders the same form as a page, for old links and new tabs.

export const UPLOAD_PATHS = ['/me/upload', '/upload'];

export const upload = $state({ open: false });

export function openUpload() {
  upload.open = true;
}

export function closeUpload() {
  upload.open = false;
}

type ClickLike = Pick<MouseEvent, 'button' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'defaultPrevented'>;
type LinkLike = { href: string; target?: string; hasAttribute(name: string): boolean };

/**
 * Whether a click on `link` should open the dialog instead of following it: a plain left click
 * (no modifier, so Ctrl/⌘-click still opens a new tab) on a same-site link to the upload page,
 * from any page but the upload page itself.
 */
export function opensDialog(e: ClickLike, link: LinkLike, here: URL): boolean {
  if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return false;
  if ((link.target && link.target !== '_self') || link.hasAttribute('download')) return false;
  let to: URL;
  try {
    to = new URL(link.href, here);
  } catch {
    return false;
  }
  if (to.origin !== here.origin || !UPLOAD_PATHS.includes(to.pathname.replace(/\/$/, ''))) return false;
  return !UPLOAD_PATHS.includes(here.pathname.replace(/\/$/, ''));
}
