// An upload whose processing ran out of attempts (`processing: FAILED`): the emote page, the tile
// and the helper say it failed, instead of "still being made" for good. One action: delete it.
import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { failed, ready } from '$lib/emote';
import EmoteTile from '$lib/components/EmoteTile.svelte';
import EmoteView from './EmoteView.svelte';
import type { PageEmote } from './emote-page';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/emotes/E1') } }));
vi.mock('$lib/tool/session.svelte', () => ({ session: { me: { id: 'A1' } }, can: () => false }));

const emote = (over: Partial<PageEmote> = {}) =>
  ({
    id: 'E1', name: 'BrokenUpload', state: 'PENDING', processing: 'FAILED', animated: false, tags: [], host: 'https://cdn.example/emotes/1',
    createdAt: '2026-11-17T10:00:00.000Z', channelCount: 0, visibility: 'PUBLIC',
    flags: { zeroWidth: false, private: false, wide: false },
    owner: { id: 'A1', displayName: 'nora_byte', avatar: null },
    files: [],
    ...over,
  }) as unknown as PageEmote;

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(() => new Promise(() => {}));
});

describe('a failed upload', () => {
  it('is failed only while it has no files', () => {
    expect(failed(emote())).toBe(true);
    expect(ready(emote())).toBe(false);
    expect(failed(emote({ processing: 'PROCESSING' } as Partial<PageEmote>))).toBe(false);
    const made = emote({ files: [{ name: '1x.webp', format: 'WEBP', scale: 1, width: 32, height: 32, size: 900, isStatic: false }] } as Partial<PageEmote>);
    expect(failed(made)).toBe(false);
  });

  it('says so on the emote page, with one action: delete', () => {
    render(EmoteView, { emote: emote(), more: [], channels: null, channelCount: 0, back: { href: '/search', label: 'All emotes', back: false } });
    const box = within(screen.getByTestId('emote-stage')).getByTestId('emote-failed');
    expect(box.textContent).toMatch(/couldn’t make this emote’s image/);
    expect(screen.queryByText(/still being made/)).toBeNull();
    const links = within(box).getAllByRole('link');
    expect(links.map((l) => l.textContent)).toEqual(['Delete emote']);
    expect(links[0].getAttribute('href')).toBe('/me/emotes/E1');
  });

  it('still says processing while the job is running', () => {
    render(EmoteView, { emote: emote({ processing: 'PROCESSING' } as Partial<PageEmote>), more: [], channels: null, channelCount: 0, back: { href: '/search', label: 'All emotes', back: false } });
    expect(screen.queryByTestId('emote-failed')).toBeNull();
    expect(screen.getByText(/still being made/)).toBeTruthy();
  });

  it('shows Failed on the tile instead of Processing', () => {
    const { unmount } = render(EmoteTile, { emote: emote() });
    expect(document.querySelector('.processing')?.textContent).toBe('Failed');
    unmount();
    render(EmoteTile, { emote: emote({ processing: 'PROCESSING' } as Partial<PageEmote>) });
    expect(document.querySelector('.processing')?.textContent).toBe('Processing');
  });
});
