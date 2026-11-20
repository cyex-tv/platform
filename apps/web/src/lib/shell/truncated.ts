// Text cut off with an ellipsis says what it is in full on hover: the first time the pointer is
// over a clipped line (or inside one, a few levels down) that has no title of its own, its text
// becomes the title. Static titles on known long labels still win; this catches the rest (names,
// hints and device lines that are only long for some people or at some widths).

/** The clipped element at or above `from` (at most `depth` levels up) that has no title or label yet. */
export function clippedUntitled(from: EventTarget | null, depth = 4): HTMLElement | null {
  let el = from instanceof Element ? from : null;
  for (let i = 0; el && i < depth; i++, el = el.parentElement) {
    if (!(el instanceof HTMLElement)) continue;
    if (el.title || el.hasAttribute('aria-label')) return null;
    if (el.scrollWidth > el.clientWidth + 1 && getComputedStyle(el).textOverflow === 'ellipsis') return el;
  }
  return null;
}

/** pointerover handler: mouse and pen only (touch has no hover). */
export function titleTruncated(e: PointerEvent) {
  if (e.pointerType === 'touch') return;
  const el = clippedUntitled(e.target);
  const text = el?.textContent?.replace(/\s+/g, ' ').trim();
  if (el && text) el.title = text;
}
