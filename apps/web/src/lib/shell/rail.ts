// SideRail's data: groups of rows. A row is a link (a section that is its own route) with an
// optional icon and count; `current` marks the page you are on (aria-current="page").
import type { IconName } from '$lib/components/Icon.svelte';

export type RailItem = {
  href: string;
  label: string;
  icon?: IconName;
  count?: number | string;
  current?: boolean;
  /** How full it is (a set): a 3px meter under the row, as the browse rail's sets (redesign round 2). */
  meter?: { used: number; capacity: number };
  /** A channel shows it: a small green LIVE after the label. */
  live?: boolean;
};
/**
 * A group of rows. `toggle` makes it collapsible under a row of its own (7TV's admin "Tickets"
 * and "Cosmetics"): open by default, or while one of its rows is current.
 */
export type RailGroup = { caption?: string; toggle?: { label: string; icon?: IconName }; items: RailItem[] };

/** Moves focus between a list's links with the arrow keys (and Home / End). */
export function arrowNav(node: HTMLElement, axis: 'vertical' | 'horizontal' = 'vertical') {
  const prev = axis === 'vertical' ? 'ArrowUp' : 'ArrowLeft';
  const next = axis === 'vertical' ? 'ArrowDown' : 'ArrowRight';
  function onkeydown(e: KeyboardEvent) {
    if (![prev, next, 'Home', 'End'].includes(e.key)) return;
    const links = [...node.querySelectorAll<HTMLElement>('a[href]')];
    const i = links.indexOf(document.activeElement as HTMLElement);
    if (i < 0) return;
    e.preventDefault();
    const to =
      e.key === 'Home' ? 0 : e.key === 'End' ? links.length - 1 : (i + (e.key === next ? 1 : -1) + links.length) % links.length;
    links[to].focus();
    if (axis === 'horizontal') links[to].scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }
  node.addEventListener('keydown', onkeydown);
  return { destroy: () => node.removeEventListener('keydown', onkeydown) };
}

/** Marks the item whose href is the longest prefix of `path` as current (exact for `exact` hrefs). */
export function markCurrent(groups: RailGroup[], path: string, exact: string[] = []): RailGroup[] {
  let best = '';
  for (const g of groups)
    for (const i of g.items) {
      const hit = exact.includes(i.href) ? path === i.href : path === i.href || path.startsWith(`${i.href}/`);
      if (hit && i.href.length > best.length) best = i.href;
    }
  return groups.map((g) => ({ ...g, items: g.items.map((i) => ({ ...i, current: i.href === best })) }));
}
