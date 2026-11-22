// The emote page's preview (owner, 2026/11/22): every size shows at once, at its real size, and
// none is a control; the largest is the focus. No Kind, Format or file-size facts, no NSFW flag,
// and no pause button (motion follows the reader's reduced-motion setting).
import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EmoteView from './EmoteView.svelte';
import type { PageEmote } from './emote-page';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/emotes/E1') } }));

const file = (scale: number, isStatic = false) => ({
  name: `${scale}x${isStatic ? '_static' : ''}.webp`, format: 'WEBP' as const, scale, width: 32 * scale, height: 32 * scale, size: 900 * scale, isStatic,
});
const emote = (over: Partial<PageEmote> = {}) =>
  ({
    id: 'E1', name: 'fernYawn', state: 'APPROVED', animated: true, tags: [], host: 'https://cdn.example/emotes/1',
    createdAt: '2026-11-17T10:00:00.000Z', channelCount: 2, visibility: 'PUBLIC',
    flags: { zeroWidth: false, private: false, wide: false },
    owner: { id: 'A1', displayName: 'nora_byte', avatar: null },
    files: [1, 2, 3, 4].flatMap((s) => [file(s), file(s, true)]),
    ...over,
  }) as unknown as PageEmote;
const view = (e = emote()) => render(EmoteView, { emote: e, more: [], channels: null, channelCount: 2, back: { href: '/search', label: 'All emotes', back: false } });

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(() => new Promise(() => {}));
});

describe('the emote page preview', () => {
  it('shows every size at once, smallest first, the largest (4x, 128 px) in focus', () => {
    view();
    const stage = screen.getByTestId('emote-stage');
    const sizes = within(stage).getAllByRole('listitem');
    expect(sizes.map((s) => s.dataset.scale)).toEqual(['1', '2', '3', '4']);
    expect(sizes.map((s) => s.textContent?.replace(/\s+/g, ' ').trim())).toEqual(['1x32 × 32', '2x64 × 64', '3x96 × 96', '4x128 × 128']);
    expect(sizes.filter((s) => s.classList.contains('focus')).map((s) => s.dataset.scale)).toEqual(['4']);
    const big = within(sizes[3]).getByRole('img', { name: 'fernYawn' });
    expect([big.getAttribute('width'), big.getAttribute('height')]).toEqual(['128', '128']);
    // Each at its own real size.
    expect(sizes.map((s) => s.querySelector('img')?.getAttribute('height'))).toEqual(['32', '64', '96', '128']);
  });

  it('has nothing to pick: no size buttons and no pause or play button', () => {
    view();
    const stage = screen.getByTestId('emote-stage');
    expect(within(stage).queryAllByRole('button')).toHaveLength(0);
    expect(screen.queryByRole('button', { name: /pause|play/i })).toBeNull();
    expect(screen.queryByRole('group', { name: 'Sizes' })).toBeNull();
  });

  it('keeps the useful facts and drops kind, formats and file sizes', () => {
    view();
    const facts = [...document.querySelectorAll('.facts dt')].map((d) => d.textContent);
    expect(facts).toEqual(['Channels', 'Uploaded', 'Review']);
    expect(document.body.textContent).not.toMatch(/Kind|Format|File at|KB\b|\d B\b/);
  });

  it('shows no NSFW flag', () => {
    view(emote({ flags: { zeroWidth: true, private: false, wide: false } } as Partial<PageEmote>));
    const flags = screen.getByRole('list', { name: 'Flags' });
    expect(flags.textContent).not.toMatch(/NSFW/);
  });
});
