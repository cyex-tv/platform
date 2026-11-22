// The browse grid's multi-select picks and unpicks, file-manager style: a drag that starts on a
// picked tile unpicks what it covers (otherwise it picks), Ctrl/⌘+drag flips each tile, Esc
// mid-drag puts the picks back, Shift+click gives the range the clicked tile's new state, and a
// long-press on a picked tile starts an unpicking drag on touch. The cap still holds.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetMe, session } from '$lib/tool/session.svelte';
import SelectGridHarness from '$lib/test/SelectGridHarness.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: vi.fn(async () => ({})), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/search') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn() }));

const N = 9;
const items = Array.from({ length: N }, (_, i) => ({
  id: `e${i}`,
  name: `emote${i}`,
  host: `https://cdn.example/e${i}`,
  animated: false,
  files: [{ name: '1x.webp', width: 32, height: 32, isStatic: false }],
  owner: { id: 'X', displayName: 'kiwi_rae' },
}));

// jsdom has no layout: three columns of 100px tiles with 10px gaps, from y = 100.
const box = (i: number) => ({ left: (i % 3) * 110, top: 100 + Math.floor(i / 3) * 110 });
const mid = (i: number) => ({ x: box(i).left + 50, y: box(i).top + 50 });
const rectOf = HTMLElement.prototype.getBoundingClientRect;
beforeEach(() => {
  forgetMe();
  session.known = true;
  HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
    if (!this.matches('li[data-id]')) return rectOf.call(this);
    const { left, top } = box(Number(this.dataset.id!.slice(1)));
    return { left, top, right: left + 100, bottom: top + 100, width: 100, height: 100, x: left, y: top, toJSON() {} } as DOMRect;
  };
});
afterEach(() => {
  HTMLElement.prototype.getBoundingClientRect = rectOf;
  vi.useRealTimers();
});

const picks = () => screen.getByTestId('picks').textContent!.split(',').filter(Boolean);
const tile = (i: number) => document.querySelector<HTMLElement>(`li[data-id="e${i}"] .tile`)!;

type Init = { ctrlKey?: boolean; metaKey?: boolean; pointerType?: string };
function pointer(el: EventTarget, type: string, at: { x: number; y: number }, init: Init = {}) {
  const ev = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: at.x, clientY: at.y, button: 0, ctrlKey: !!init.ctrlKey, metaKey: !!init.metaKey });
  Object.defineProperty(ev, 'pointerType', { value: init.pointerType ?? 'mouse' });
  el.dispatchEvent(ev);
}
/** Press on tile `from`, move over the tiles' centres in `over`, and (unless `hold`) let go. */
async function dragTiles(from: number, over: number[], init: Init = {}, hold = false) {
  pointer(tile(from), 'pointerdown', mid(from), init);
  for (const i of over) pointer(window, 'pointermove', mid(i), init);
  if (!hold) pointer(window, 'pointerup', mid(over.at(-1) ?? from), init);
  await Promise.resolve();
}

describe('drag-select', () => {
  it('a drag that starts on a picked tile unpicks every tile it covers, and keeps it on release', async () => {
    render(SelectGridHarness, { items, picked: ['e0', 'e1', 'e2', 'e4'], start: 'e4' });
    // From e1 down to e4: the rectangle covers e1 and e4.
    await dragTiles(1, [4], {}, true);
    expect(picks()).toEqual(['e0', 'e2']);
    // Back up over e1 only: e4 comes back, as it was before the drag.
    pointer(window, 'pointermove', { x: mid(1).x + 5, y: mid(1).y + 5 });
    await Promise.resolve();
    expect(picks()).toEqual(['e0', 'e2', 'e4']);
    pointer(window, 'pointermove', mid(4));
    pointer(window, 'pointerup', mid(4));
    await Promise.resolve();
    expect(picks()).toEqual(['e0', 'e2']);
    // The click that ends the drag does not toggle the tile under it.
    await fireEvent.click(tile(4));
    expect(picks()).toEqual(['e0', 'e2']);
    expect(screen.getByTestId('anchor').textContent).toBe('e4');
  });

  it('a drag that starts on an unpicked tile picks what it covers', async () => {
    render(SelectGridHarness, { items, picked: ['e0'] });
    await dragTiles(4, [5]);
    expect(picks()).toEqual(['e0', 'e4', 'e5']);
  });

  it('Ctrl/⌘+drag flips each tile it covers', async () => {
    render(SelectGridHarness, { items, picked: ['e0', 'e1', 'e7'] });
    await dragTiles(0, [2], { ctrlKey: true });
    expect(picks()).toEqual(['e7', 'e2']);
    await dragTiles(6, [7], { metaKey: true });
    expect(picks()).toEqual(['e2', 'e6']);
  });

  it('Esc mid-drag puts the picks back as before it (and the page’s Esc does not also clear them)', async () => {
    render(SelectGridHarness, { items, picked: ['e0', 'e1'] });
    await dragTiles(0, [2], {}, true);
    expect(picks()).toEqual([]);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(picks()).toEqual(['e0', 'e1']);
    expect(screen.getByTestId('mode').textContent).toBe('on');
    // Moving on and letting go after Esc changes nothing, nor does the release's click.
    pointer(window, 'pointermove', mid(5));
    pointer(window, 'pointerup', mid(0));
    await fireEvent.click(tile(0));
    expect(picks()).toEqual(['e0', 'e1']);
    // A second Esc is the page's again: it clears.
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(picks()).toEqual([]);
  });

  it('a drag from an empty spot turns Select mode on, and Esc turns it off again', async () => {
    render(SelectGridHarness, { items, on: false });
    const area = document.querySelector('.pick-area')!;
    pointer(area, 'pointerdown', { x: 340, y: 120 });
    pointer(window, 'pointermove', { x: 150, y: 140 });
    await Promise.resolve();
    expect(screen.getByTestId('mode').textContent).toBe('on');
    expect(picks()).toEqual(['e1', 'e2']);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.getByTestId('mode').textContent).toBe('off');
    expect(picks()).toEqual([]);
    pointer(window, 'pointerup', { x: 150, y: 140 });
  });

  it('keeps the cap and says so', async () => {
    const many = Array.from({ length: 98 }, (_, i) => `x${i}`);
    render(SelectGridHarness, { items, picked: many });
    await dragTiles(0, [2]);
    expect(picks()).toHaveLength(100);
    expect(picks().slice(-2)).toEqual(['e0', 'e1']);
    expect(screen.getByTestId('note').textContent).toMatch(/100/);
  });
});

describe('clicks', () => {
  it('Shift+click on a picked tile unpicks the range from the anchor; on an unpicked one picks it', async () => {
    render(SelectGridHarness, { items, picked: ['e1', 'e2', 'e3', 'e5'], start: 'e1' });
    await fireEvent.click(tile(3), { shiftKey: true });
    expect(picks()).toEqual(['e5']);
    expect(screen.getByTestId('anchor').textContent).toBe('e1');
    await fireEvent.click(tile(4), { shiftKey: true });
    expect(picks()).toEqual(['e5', 'e1', 'e2', 'e3', 'e4']);
  });

  it('a plain or Ctrl/⌘+click toggles one tile and moves the anchor', async () => {
    render(SelectGridHarness, { items, picked: ['e1'], start: 'e1' });
    await fireEvent.click(tile(6), { ctrlKey: true });
    expect(picks()).toEqual(['e1', 'e6']);
    expect(screen.getByTestId('anchor').textContent).toBe('e6');
    await fireEvent.click(tile(1));
    expect(picks()).toEqual(['e6']);
    expect(screen.getByTestId('anchor').textContent).toBe('e1');
  });
});

describe('touch', () => {
  it('a long-press on a picked tile unpicks it and the tiles the drag goes on over', async () => {
    vi.useFakeTimers();
    render(SelectGridHarness, { items, picked: ['e1', 'e4', 'e7'] });
    pointer(tile(1), 'pointerdown', mid(1), { pointerType: 'touch' });
    vi.advanceTimersByTime(500);
    await Promise.resolve();
    expect(picks()).toEqual(['e4', 'e7']);
    pointer(window, 'pointermove', mid(4), { pointerType: 'touch' });
    await Promise.resolve();
    expect(picks()).toEqual(['e7']);
    pointer(window, 'pointerup', mid(4), { pointerType: 'touch' });
    await Promise.resolve();
    expect(picks()).toEqual(['e7']);
  });

  it('a long-press on an unpicked tile picks it and goes on picking', async () => {
    vi.useFakeTimers();
    render(SelectGridHarness, { items, picked: ['e7'] });
    pointer(tile(0), 'pointerdown', mid(0), { pointerType: 'touch' });
    vi.advanceTimersByTime(500);
    pointer(window, 'pointermove', mid(1), { pointerType: 'touch' });
    pointer(window, 'pointerup', mid(1), { pointerType: 'touch' });
    await Promise.resolve();
    expect(picks()).toEqual(['e7', 'e0', 'e1']);
  });
});
