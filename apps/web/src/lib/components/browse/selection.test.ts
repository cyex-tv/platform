import { describe, expect, it } from 'vitest';
import { all, addMany, CAP, cancelDrag, drag, dragMode, EMPTY, edgeSpeed, hitTest, moved, range, rect, toggle, type Selection } from './selection';

const ids = (n: number, from = 0) => Array.from({ length: n }, (_, i) => `e${i + from}`);
const order = ids(10);

describe('toggle', () => {
  it('picks, unpicks and moves the anchor', () => {
    let s = toggle(EMPTY, 'e1').sel;
    expect(s).toEqual({ ids: ['e1'], anchor: 'e1' });
    s = toggle(s, 'e3').sel;
    expect(s.ids).toEqual(['e1', 'e3']);
    s = toggle(s, 'e1').sel;
    expect(s).toEqual({ ids: ['e3'], anchor: 'e1' });
  });
  it('refuses a new one at the cap but still unpicks', () => {
    const full: Selection = { ids: ids(CAP), anchor: null };
    const r = toggle(full, 'new');
    expect(r.refused).toBe(1);
    expect(r.sel).toBe(full);
    expect(toggle(full, 'e5').sel.ids).toHaveLength(CAP - 1);
  });
});

describe('range', () => {
  it('picks from the anchor to the target, both ways, keeping earlier picks', () => {
    const s = toggle(toggle(EMPTY, 'e9').sel, 'e2').sel; // anchor e2
    const down = range(s, order, 'e5').sel;
    expect(down.ids).toEqual(['e9', 'e2', 'e3', 'e4', 'e5']);
    expect(down.anchor).toBe('e2');
    const up = range({ ids: [], anchor: 'e5' }, order, 'e3').sel;
    expect(up.ids).toEqual(['e5', 'e4', 'e3']);
  });
  it('unpicks the whole range when the clicked tile is already picked, keeping the anchor', () => {
    const s: Selection = { ids: ['e0', 'e2', 'e3', 'e4', 'e5', 'e8'], anchor: 'e2' };
    const r = range(s, order, 'e5');
    expect(r.sel).toEqual({ ids: ['e0', 'e8'], anchor: 'e2' });
    expect(r.refused).toBe(0);
    // Upwards, and over tiles that were not picked: they stay unpicked.
    const up = range({ ids: ['e1', 'e3', 'e6'], anchor: 'e6' }, order, 'e3').sel;
    expect(up).toEqual({ ids: ['e1'], anchor: 'e6' });
  });
  it('picks the range when the clicked tile is not picked, even if the anchor was unpicked', () => {
    // A plain click unpicked e2 (it is still the anchor); Shift+click on e4 picks e2..e4.
    const s = toggle({ ids: ['e2', 'e7'], anchor: 'e7' }, 'e2').sel;
    expect(s).toEqual({ ids: ['e7'], anchor: 'e2' });
    expect(range(s, order, 'e4').sel).toEqual({ ids: ['e7', 'e2', 'e3', 'e4'], anchor: 'e2' });
  });
  it('is a toggle without an anchor, or when the anchor is no longer shown', () => {
    expect(range(EMPTY, order, 'e4').sel).toEqual({ ids: ['e4'], anchor: 'e4' });
    expect(range({ ids: [], anchor: 'gone' }, order, 'e4').sel.ids).toEqual(['e4']);
  });
  it('ignores a target that is not shown', () => {
    const s = { ids: ['e1'], anchor: 'e1' };
    expect(range(s, order, 'nope').sel).toBe(s);
  });
  it('stops at the cap and says how many it left out', () => {
    const big = ids(150);
    const r = range({ ids: [], anchor: 'e0' }, big, 'e149');
    expect(r.sel.ids).toHaveLength(CAP);
    expect(r.sel.ids.at(-1)).toBe('e99');
    expect(r.refused).toBe(50);
  });
});

describe('all and addMany', () => {
  it('adds what is shown up to the cap, skipping repeats', () => {
    const r = all({ ids: ['e3'], anchor: null }, order, 5);
    expect(r.sel.ids).toEqual(['e3', 'e0', 'e1', 'e2', 'e4']);
    expect(r.refused).toBe(5);
    expect(addMany(EMPTY, ['a', 'a', 'b']).sel.ids).toEqual(['a', 'b']);
  });
  it('refuses everything when already full', () => {
    const r = all({ ids: ids(CAP), anchor: null }, ids(5, 500));
    expect(r.refused).toBe(5);
    expect(r.sel.ids).toHaveLength(CAP);
  });
});

describe('drag', () => {
  it('is the base plus the hits, recomputed from the base', () => {
    const base = { ids: ['e0'], anchor: 'e0' };
    expect(drag(base, ['e1', 'e2']).sel).toEqual({ ids: ['e0', 'e1', 'e2'], anchor: 'e0' });
    // The rectangle shrinks again: e2 drops out.
    expect(drag(base, ['e1']).sel.ids).toEqual(['e0', 'e1']);
  });
  it('caps the hits', () => {
    const r = drag({ ids: ids(98), anchor: null }, ['x', 'y', 'z']);
    expect(r.sel.ids.slice(-2)).toEqual(['x', 'y']);
    expect(r.refused).toBe(1);
  });
});

describe('dragMode', () => {
  const base: Selection = { ids: ['e1', 'e2'], anchor: 'e2' };
  it('adds from an empty spot or an unpicked tile, removes from a picked one', () => {
    expect(dragMode(base, null)).toBe('add');
    expect(dragMode(base, 'e5')).toBe('add');
    expect(dragMode(base, 'e1')).toBe('remove');
  });
  it('toggles with Ctrl/Cmd held, wherever it starts', () => {
    expect(dragMode(base, null, true)).toBe('toggle');
    expect(dragMode(base, 'e1', true)).toBe('toggle');
    expect(dragMode(base, 'e5', true)).toBe('toggle');
  });
});

describe('drag that unpicks or toggles', () => {
  const base: Selection = { ids: ['e0', 'e1', 'e2', 'e3', 'e7'], anchor: 'e3' };
  it('a drag from a picked tile unpicks what it covers, and only that', () => {
    const from = 'e1';
    const mode = dragMode(base, from);
    const r = drag(base, ['e1', 'e2', 'e5'], mode);
    expect(r.sel).toEqual({ ids: ['e0', 'e3', 'e7'], anchor: 'e3' });
    expect(r.refused).toBe(0);
    // The rectangle shrinks back: e2 is picked again, as before the drag.
    expect(drag(base, ['e1'], mode).sel.ids).toEqual(['e0', 'e2', 'e3', 'e7']);
  });
  it('Ctrl+drag flips each covered tile', () => {
    const r = drag(base, ['e2', 'e3', 'e4', 'e5'], 'toggle');
    expect(r.sel).toEqual({ ids: ['e0', 'e1', 'e7', 'e4', 'e5'], anchor: 'e3' });
    expect(drag(base, [], 'toggle').sel.ids).toEqual(base.ids);
  });
  it('Ctrl+drag keeps the cap for the tiles it picks, never for the ones it unpicks', () => {
    const full: Selection = { ids: ids(CAP), anchor: null };
    const r = drag(full, ['e0', 'e1', 'x', 'y', 'z'], 'toggle');
    expect(r.sel.ids).toHaveLength(CAP);
    expect(r.sel.ids).not.toContain('e0');
    expect(r.sel.ids.slice(-2)).toEqual(['x', 'y']);
    expect(r.refused).toBe(1);
  });
  it('an unpicking drag never hits the cap', () => {
    const full: Selection = { ids: ids(CAP), anchor: null };
    const r = drag(full, ['e5', 'nope'], 'remove');
    expect(r.sel.ids).toHaveLength(CAP - 1);
    expect(r.refused).toBe(0);
  });
  it('Esc puts back exactly what was picked before the drag', () => {
    const mid = drag(base, ['e0', 'e1'], 'remove').sel;
    expect(mid.ids).toEqual(['e2', 'e3', 'e7']);
    expect(cancelDrag(base)).toEqual({ sel: base, refused: 0 });
  });
});

describe('rect and hitTest', () => {
  // A 3 × 2 grid of 100px tiles with 10px gaps.
  const boxes = ids(6).map((id, i) => {
    const col = i % 3;
    const row = Math.floor(i / 3);
    return { id, left: col * 110, top: row * 110, right: col * 110 + 100, bottom: row * 110 + 100 };
  });
  it('normalises the corners whichever way the pointer went', () => {
    expect(rect({ x: 50, y: 80 }, { x: 10, y: 20 })).toEqual({ left: 10, top: 20, right: 50, bottom: 80 });
  });
  it('finds overlapped tiles in grid order', () => {
    expect(hitTest(rect({ x: 50, y: 50 }, { x: 160, y: 160 }), boxes)).toEqual(['e0', 'e1', 'e3', 'e4']);
    expect(hitTest(rect({ x: 330, y: 230 }, { x: 0, y: 0 }), boxes)).toEqual(ids(6));
  });
  it('a rectangle inside a gap, or only touching an edge, hits nothing', () => {
    expect(hitTest(rect({ x: 101, y: 0 }, { x: 109, y: 300 }), boxes)).toEqual([]);
    expect(hitTest(rect({ x: 100, y: 0 }, { x: 110, y: 100 }), boxes)).toEqual([]);
  });
});

describe('moved and edgeSpeed', () => {
  it('a press becomes a drag past the threshold', () => {
    expect(moved({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(false);
    expect(moved({ x: 0, y: 0 }, { x: 6, y: 0 })).toBe(true);
  });
  it('scrolls faster nearer the edge, not at all in the middle', () => {
    expect(edgeSpeed(400, 800)).toBe(0);
    expect(edgeSpeed(32, 800)).toBe(-14);
    expect(edgeSpeed(-50, 800)).toBe(-28);
    expect(edgeSpeed(800, 800)).toBe(28);
    expect(edgeSpeed(768, 800)).toBe(14);
  });
});
