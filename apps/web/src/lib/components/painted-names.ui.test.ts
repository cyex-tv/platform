// Creator names under emote tiles and the names in an emote's channel list are drawn in each
// person's paint, loaded in batches: a grid asks once per 100 new people, never once per tile.
import { render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetLooks } from '$lib/cosmetics/store.svelte';
import TileGridHarness from '../test/TileGridHarness.svelte';
import ChannelsTab from './emote/ChannelsTab.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const sunset = { id: 'P1', name: 'Sunset', kind: 'LINEAR_GRADIENT', angle: 90, repeat: false, shape: 'CIRCLE', color: '#FF7A45',
  stops: [{ at: 0, color: '#FF7A45' }, { at: 1, color: '#8F5BFF' }], shadows: [], image: null };
const moving = { ...sunset, id: 'P2', kind: 'IMAGE', stops: [], image: { id: 'I1', kind: 'PAINT', host: 'https://cdn.cyex.tv/cosmetics/paints/p2', animated: true, ready: true,
  width: 64, height: 16, files: [{ name: '1x.webp', width: 64, height: 16 }, { name: '1x_static.webp', width: 64, height: 16 }] } };

/** Account ids each accountCosmetics call asked for. */
const asked = () => gql.mock.calls.filter(([doc]) => String(doc).includes('accountCosmetics')).map(([, v]) => (v as { accountIds: string[] }).accountIds);

const tile = (i: number, owner: number) => ({
  id: `E${i}`, name: `emote${i}`, host: 'https://cdn.example/e', animated: false,
  files: [{ name: '1x.webp', width: 32, height: 32, isStatic: true }],
  owner: { id: `A${owner}`, displayName: `maker${owner}` },
});

beforeEach(() => {
  forgetLooks();
  gql.mockReset();
  gql.mockImplementation(async (doc: string, v: { accountIds?: string[] }) => {
    if (doc.includes('accountCosmetics'))
      return { accountCosmetics: (v.accountIds ?? []).filter((id) => id === 'A1' || id === 'A7' || id === 'C2').map((id) => ({ accountId: id, badge: null, paint: id === 'A7' ? moving : sunset })) };
    throw new Error(`unexpected ${doc.slice(0, 40)}`);
  });
});

describe('painted creator names on tiles', () => {
  it('asks for a grid of 120 tiles in two requests, each creator once', async () => {
    // 120 tiles by 110 creators (some made several): 100 in the first request, 10 in the second.
    const items = Array.from({ length: 120 }, (_, i) => tile(i, i % 110));
    render(TileGridHarness, { items });
    await waitFor(() => expect(asked().length).toBe(2));
    const ids = asked().flat();
    expect(asked()[0]).toHaveLength(100);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toHaveLength(110);
  });

  it('asks nothing again for creators it already knows (a second grid, the next page)', async () => {
    const first = render(TileGridHarness, { items: [tile(1, 1), tile(2, 2)] });
    await waitFor(() => expect(asked()).toHaveLength(1));
    first.unmount();
    // Same creators on another page, plus one new: only the new one is asked for.
    render(TileGridHarness, { items: [tile(3, 1), tile(4, 2), tile(5, 3)] });
    await waitFor(() => expect(asked()).toHaveLength(2));
    expect(asked()[1]).toEqual(['A3']);
  });

  it('draws the creator in their paint, others plain, in the same box', async () => {
    const { container } = render(TileGridHarness, { items: [tile(1, 1), tile(2, 2)] });
    const name = () => container.querySelector('a[href="/emotes/E1"] .by bdi') as HTMLElement;
    const plain = container.querySelector('a[href="/emotes/E2"] .by bdi') as HTMLElement;
    // Before the paint arrives the name is already the painted-name element (no swap, no shift).
    expect(name().textContent).toBe('maker1');
    await waitFor(() => expect(name().getAttribute('style') ?? '').toContain('linear-gradient'));
    expect(name().classList.contains('painted')).toBe(true);
    expect(plain.getAttribute('style') ?? '').toBe('');
    expect(plain.textContent).toBe('maker2');
  });

  it('shows an image paint’s still frame under reduced motion', async () => {
    const { container } = render(TileGridHarness, { items: [tile(1, 7)] });
    const name = () => container.querySelector('.by bdi') as HTMLElement;
    await waitFor(() => expect(name().getAttribute('style') ?? '').toContain('1x_static.webp'));
  });

  it('asks nothing for a tile whose creator has no account id', async () => {
    // (TileGridHarness always shows creators; a tile without an owner id asks for nothing.)
    render(TileGridHarness, { items: [{ ...tile(1, 1), owner: { displayName: 'nobody' } }] });
    await new Promise((r) => setTimeout(r, 10));
    expect(asked()).toHaveLength(0);
  });
});

describe('painted names in an emote’s channel list', () => {
  const rows = [
    { platform: 'TWITCH' as const, login: 'c1', accountId: 'C1', displayName: 'chanOne', avatar: null },
    { platform: 'KICK' as const, login: 'c2', accountId: 'C2', displayName: 'chanTwo', avatar: null },
  ];

  it('asks for the page’s accounts once and paints the ones that wear a paint', async () => {
    render(ChannelsTab, { emoteId: 'E1', first: rows, total: 2 });
    const two = () => screen.getByText('chanTwo');
    await waitFor(() => expect(two().getAttribute('style') ?? '').toContain('linear-gradient'));
    expect(asked()).toEqual([['C1', 'C2']]);
    expect(screen.getByText('chanOne').getAttribute('style') ?? '').toBe('');
    expect(two().closest('a')?.getAttribute('href')).toBe('/channels/kick/c2');
  });
});
