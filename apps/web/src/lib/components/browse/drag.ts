// Dragging emotes from the browse grid onto a set in the rail. The drag carries the emotes' ids
// as text under our own type (so other drop zones ignore it); the pictures and names the rail
// needs for its toast and the tray ride along here, since a drag can only carry strings.
import type { SetItem } from './target.svelte';

export const EMOTE_DRAG = 'application/x-cyex-emotes';

let carrying: SetItem[] = [];

/** Starts a drag of `items` (one tile, or the picked ones). */
export function startDrag(dt: DataTransfer, items: SetItem[]) {
  carrying = items;
  dt.effectAllowed = 'copy';
  dt.setData(EMOTE_DRAG, JSON.stringify(items.map((i) => i.id)));
  // A link's own URL stays for drops outside the page (another tab, a chat box).
}

/** Whether a drag over a set is ours. */
export function isOurs(dt: DataTransfer | null): boolean {
  return !!dt && [...dt.types].includes(EMOTE_DRAG);
}

/** The emotes being dropped (only those the drag started with). */
export function dropped(dt: DataTransfer): SetItem[] {
  let ids: string[] = [];
  try {
    ids = JSON.parse(dt.getData(EMOTE_DRAG) || '[]');
  } catch {
    ids = [];
  }
  return carrying.filter((i) => ids.includes(i.id));
}

export function endDrag() {
  carrying = [];
}
