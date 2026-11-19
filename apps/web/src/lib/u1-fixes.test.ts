import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import { addState, plan } from './components/browse/bulk';
import BrowseGrid from './components/browse/BrowseGrid.svelte';
import { canReport, transferLine } from './components/emote/emote-page';
import LocalPreview from './tool/LocalPreview.svelte';

describe('Add button state in the select bar', () => {
  const held = new Set(['a', 'b']);
  it('says add while something will go in', () => {
    expect(addState(2, plan(['a', 'c'], held, 5))).toBe('add');
  });
  it('says already when every pick is in the set (never "Add 0")', () => {
    expect(addState(2, plan(['a', 'b'], held, 5))).toBe('already');
  });
  it('says full when nothing fits', () => {
    expect(addState(1, plan(['z'], held, 0))).toBe('full');
  });
  it('says none with nothing picked or no set', () => {
    expect(addState(0, plan([], held, 5))).toBe('none');
    expect(addState(2, null)).toBe('none');
  });
});

describe('Report', () => {
  it('is offered on approved emotes that are not yours', () => {
    expect(canReport({ state: 'APPROVED', owner: { id: 'U2' } } as never, 'U1')).toBe(true);
    expect(canReport({ state: 'APPROVED', owner: { id: 'U2' } } as never, null)).toBe(true);
  });
  it('is not offered on your own emote, or one that is not approved', () => {
    expect(canReport({ state: 'APPROVED', owner: { id: 'U1' } } as never, 'U1')).toBe(false);
    expect(canReport({ state: 'PENDING', owner: { id: 'U2' } } as never, 'U1')).toBe(false);
  });
});

describe('transfer request wording', () => {
  const acct = (id: string, displayName: string) => ({ id, displayName });
  it('says themselves when they ask to give it to their own account', () => {
    const r = { requestedBy: acct('P', 'pixelmoth'), to: acct('P', 'pixelmoth') };
    expect(transferLine(r, 'ME')).toEqual({ asker: { name: 'pixelmoth' }, target: 'themselves' });
  });
  it('says yourself when you ask to give it to yourself', () => {
    const r = { requestedBy: acct('ME', 'me'), to: acct('ME', 'me') };
    expect(transferLine(r, 'ME')).toEqual({ asker: 'you', target: 'yourself' });
  });
  it('names the other account, or you', () => {
    expect(transferLine({ requestedBy: acct('P', 'pixelmoth'), to: acct('ME', 'me') }, 'ME').target).toBe('you');
    expect(transferLine({ requestedBy: acct('ME', 'me'), to: acct('Q', 'quill') }, 'ME').target).toEqual({ name: 'quill' });
  });
});

describe('upload preview', () => {
  it('shows 1x, 2x, 3x and 4x', () => {
    const html = render(LocalPreview, { props: { src: 'blob:x', width: 32, height: 32, name: 'a' } }).body;
    for (const k of ['1x', '2x', '3x', '4x']) expect(html).toContain(`>${k}<`);
  });
});

describe('browse grid preloading', () => {
  const props = (selecting: boolean) => ({ items: [], label: 'Emotes', selecting, sel: { ids: [], anchor: null }, onstep: () => {}, onstart: () => {} });
  it('loads a tile’s page on tap, not on hover', () => {
    expect(render(BrowseGrid, { props: props(false) }).body).toContain('data-sveltekit-preload-data="tap"');
  });
  it('loads nothing in Select mode (a drag would fetch dozens)', () => {
    expect(render(BrowseGrid, { props: props(true) }).body).toContain('data-sveltekit-preload-data="off"');
  });
});
