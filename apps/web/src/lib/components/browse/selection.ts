// Multi-select on the browse grid (and the admin review queue and reports): which emotes are picked (in the order picked), the anchor a
// Shift+click range starts from, and the cap. Clicks, ranges, Select all and the drag rectangle
// all go through here, so each stays under the cap and says how many it had to leave out.
// Pure, so it is unit tested; the page only feeds it ids and boxes.

/** The most emotes picked at once. */
export const CAP = 100;

export type Selection = {
  /** Picked ids, in the order they were picked. */
  ids: readonly string[];
  /** Where the next Shift+click range starts: the last tile toggled on or off by hand. */
  anchor: string | null;
};

/** A change, and how many emotes it wanted to pick but couldn't (the cap). */
export type Step = { sel: Selection; refused: number };

export const EMPTY: Selection = { ids: [], anchor: null };

/** Adds `more` (in order, repeats and already picked ones skipped) while there is room. */
export function addMany(sel: Selection, more: readonly string[], cap = CAP): Step {
  const have = new Set(sel.ids);
  const fresh = [...new Set(more)].filter((id) => !have.has(id));
  const room = Math.max(0, cap - sel.ids.length);
  return { sel: { ...sel, ids: [...sel.ids, ...fresh.slice(0, room)] }, refused: Math.max(0, fresh.length - room) };
}

/** Click, Ctrl/⌘+click or Space: picks or unpicks one, and makes it the anchor. */
export function toggle(sel: Selection, id: string, cap = CAP): Step {
  if (sel.ids.includes(id)) return { sel: { ids: sel.ids.filter((x) => x !== id), anchor: id }, refused: 0 };
  if (sel.ids.length >= cap) return { sel, refused: 1 };
  return { sel: { ids: [...sel.ids, id], anchor: id }, refused: 0 };
}

/**
 * Shift+click: every tile from the anchor to `to` in grid order (`order`) takes the state `to`
 * ends up with: `to` unpicked, the range is picked (keeping what was picked before); `to` already
 * picked, the whole range is unpicked. The anchor stays put. No anchor yet (or it is no longer
 * shown): a plain toggle.
 */
export function range(sel: Selection, order: readonly string[], to: string, cap = CAP): Step {
  const a = sel.anchor === null ? -1 : order.indexOf(sel.anchor);
  const b = order.indexOf(to);
  if (b < 0) return { sel, refused: 0 };
  if (a < 0) return toggle(sel, to, cap);
  const span = a <= b ? order.slice(a, b + 1) : order.slice(b, a + 1).reverse();
  if (sel.ids.includes(to)) return { sel: { ids: without(sel.ids, span), anchor: sel.anchor }, refused: 0 };
  const step = addMany(sel, span, cap);
  return { sel: { ...step.sel, anchor: sel.anchor }, refused: step.refused };
}

/** `ids` less every one in `drop`, order kept. */
function without(ids: readonly string[], drop: readonly string[]): string[] {
  const out = new Set(drop);
  return ids.filter((id) => !out.has(id));
}

/** Ctrl+A: everything shown, in grid order, up to the cap. */
export function all(sel: Selection, order: readonly string[], cap = CAP): Step {
  return addMany(sel, order, cap);
}

/**
 * What the drag rectangle does to the tiles it covers, file-manager style:
 *   - `add`: picks them (a drag that starts on an empty spot or an unpicked tile);
 *   - `remove`: unpicks them (a drag that starts on a picked tile, or a long-press on one);
 *   - `toggle`: flips each one (Ctrl/⌘ held when the drag starts).
 */
export type DragMode = 'add' | 'remove' | 'toggle';

/** The mode for a drag that starts on `from` (a tile id, or null for an empty spot). */
export function dragMode(base: Selection, from: string | null, ctrl = false): DragMode {
  if (ctrl) return 'toggle';
  return from !== null && base.ids.includes(from) ? 'remove' : 'add';
}

/**
 * The drag rectangle: what was picked when the drag started (`base`) with every tile it touches
 * (in grid order) picked, unpicked or flipped as `mode` says. Recomputed from `base` on every
 * move, so tiles the rectangle leaves again go back to how they were. The anchor stays put.
 */
export function drag(base: Selection, hits: readonly string[], mode: DragMode = 'add', cap = CAP): Step {
  if (mode === 'remove') return { sel: { ids: without(base.ids, hits), anchor: base.anchor }, refused: 0 };
  if (mode === 'toggle') {
    const had = new Set(base.ids);
    const kept = without(base.ids, hits.filter((id) => had.has(id)));
    const step = addMany({ ids: kept, anchor: base.anchor }, hits.filter((id) => !had.has(id)), cap);
    return { sel: { ...step.sel, anchor: base.anchor }, refused: step.refused };
  }
  const step = addMany(base, hits, cap);
  return { sel: { ...step.sel, anchor: base.anchor }, refused: step.refused };
}

/** Esc mid-drag: back to exactly what was picked before the drag. */
export function cancelDrag(base: Selection): Step {
  return { sel: base, refused: 0 };
}

export type Point = { x: number; y: number };
export type Box = { left: number; top: number; right: number; bottom: number };

/** The rectangle between two points, whichever way the pointer went. */
export function rect(a: Point, b: Point): Box {
  return { left: Math.min(a.x, b.x), top: Math.min(a.y, b.y), right: Math.max(a.x, b.x), bottom: Math.max(a.y, b.y) };
}

/** Ids of the boxes the rectangle overlaps (touching an edge is not enough), in the boxes' order. */
export function hitTest(r: Box, boxes: readonly (Box & { id: string })[]): string[] {
  return boxes.filter((b) => b.left < r.right && b.right > r.left && b.top < r.bottom && b.bottom > r.top).map((b) => b.id);
}

/** A press only becomes a drag past this many pixels, so a slightly shaky click still clicks. */
export const DRAG_START = 6;

export function moved(a: Point, b: Point, min = DRAG_START): boolean {
  return Math.hypot(b.x - a.x, b.y - a.y) >= min;
}

/**
 * Auto-scroll while dragging near the window's top or bottom edge: pixels per frame, negative
 * upwards, faster the closer the pointer is to (or the further past) the edge.
 */
export function edgeSpeed(y: number, height: number, edge = 64, max = 28): number {
  if (y < edge) return -Math.round(max * Math.min(1, (edge - y) / edge));
  if (y > height - edge) return Math.round(max * Math.min(1, (y - (height - edge)) / edge));
  return 0;
}
