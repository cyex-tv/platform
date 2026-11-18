// Arrow keys inside a short list of buttons (a popover's menu or choices): Up and Down step,
// Home and End jump, and the ends stop instead of wrapping. Tab and Escape are left alone, so
// Tab still leaves the list and Escape still closes whatever holds it.

/** Where focus goes next in a list of `count` items from `current` (-1: none focused), or null when the key isn't ours. */
export function nextIndex(key: string, current: number, count: number): number | null {
  if (count <= 0) return null;
  switch (key) {
    case 'ArrowDown':
      return Math.min(count - 1, current + 1);
    case 'ArrowUp':
      return current < 0 ? count - 1 : Math.max(0, current - 1);
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    default:
      return null;
  }
}

const ITEMS = 'button:not([disabled]), a[href]';

/** `onkeydown` for the list's container: moves focus between its buttons and links. */
export function roving(e: KeyboardEvent, items = ITEMS): void {
  if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
  const list = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>(items)];
  const to = nextIndex(e.key, list.indexOf(document.activeElement as HTMLElement), list.length);
  if (to === null) return;
  e.preventDefault();
  list[to]?.focus();
}
