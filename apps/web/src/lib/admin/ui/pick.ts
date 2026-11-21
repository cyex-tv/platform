// The picked item of an admin list with an inspector (Split): it lives in the address (?badge=…),
// so a reload, Back and a shared link open the same one. And the arrow keys over the list's items
// (a grid or a column): one Tab stop, arrows move, Home and End jump.
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { step } from '../queue';

/** The address with `param` set to `id` (or removed for null), other parameters kept. */
export function withParam(url: URL, param: string, id: string | null): string {
  const q = new URLSearchParams(url.searchParams);
  if (id) q.set(param, id);
  else q.delete(param);
  const s = q.toString();
  return s ? `?${s}` : url.pathname;
}

/** Picks `id` (null: none) into the address without a new history entry or a scroll. */
export function pickInUrl(param: string, id: string | null) {
  return goto(withParam(page.url, param, id), { replaceState: true, keepFocus: true, noScroll: true });
}

/** How many columns a CSS grid lays out (1 for anything else). */
export function columnsOf(el: HTMLElement): number {
  const t = getComputedStyle(el).gridTemplateColumns;
  return t && t !== 'none' ? t.split(' ').filter(Boolean).length : 1;
}

/**
 * Arrow keys over the items (matched by `item`) inside `node`: focus moves by the grid's columns.
 * The items keep one Tab stop between them (tabindex 0 on the current one, -1 on the rest, set by
 * the markup); this only moves focus. Enter and Space are the items' own (they are buttons).
 */
export function gridKeys(node: HTMLElement, item = '[data-item]') {
  function onkeydown(e: KeyboardEvent) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const t = e.target as HTMLElement;
    if (!t.matches(item)) return;
    const list = [...node.querySelectorAll<HTMLElement>(item)];
    const at = list.indexOf(t);
    const to = step(e.key, at, list.length, columnsOf(node));
    if (to === null || to === at) return;
    e.preventDefault();
    for (const el of list) el.tabIndex = -1;
    list[to].tabIndex = 0;
    list[to].focus();
    list[to].scrollIntoView({ block: 'nearest' });
  }
  node.addEventListener('keydown', onkeydown);
  return { destroy: () => node.removeEventListener('keydown', onkeydown) };
}
