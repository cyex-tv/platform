// The profile's emote panel: a picked tile's emote shows in place of About, with a way to its
// full page and a way back (the close button; the page also takes Escape and back).
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import EmoteInspector from './EmoteInspector.svelte';
import type { PageEmote } from '$lib/components/emote/emote-page';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: vi.fn(() => new Promise(() => {})), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/users/A1?e=E1') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

const file = (scale: number) => ({ name: `${scale}x.webp`, format: 'WEBP', scale, width: 32 * scale, height: 32 * scale, size: 1200 * scale, isStatic: false });
const emote = {
  id: 'E1', name: 'beanSus', state: 'APPROVED', animated: false, tags: ['meme'], host: 'https://cdn.example/e/1',
  createdAt: '2026-11-10T10:00:00.000Z', channelCount: 820, visibility: 'PUBLIC',
  flags: { zeroWidth: false, private: false, wide: false },
  owner: { id: 'A1', displayName: 'emberlynx', avatar: null },
  files: [1, 2, 3, 4].map(file),
} as unknown as PageEmote;

describe('profile emote panel', () => {
  it('shows the picked emote with its sizes, facts and the way to its page', () => {
    render(EmoteInspector, { emote, onclose: () => {} });
    expect(screen.getByText('beanSus')).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Sizes' }).querySelectorAll('li')).toHaveLength(4);
    expect(screen.getAllByRole('link', { name: /Open full page/ }).map((a) => a.getAttribute('href'))).toEqual(['/emotes/E1', '/emotes/E1']);
    expect(document.querySelector('dl.kv')?.textContent).toMatch(/Channels\s*820/);
  });

  it('closes from its button', async () => {
    const onclose = vi.fn();
    const user = userEvent.setup();
    render(EmoteInspector, { emote, onclose });
    await user.click(screen.getByRole('button', { name: 'Close' }));
    expect(onclose).toHaveBeenCalledOnce();
  });

  it('leaves its head to the page’s Inspector panel when asked', () => {
    render(EmoteInspector, { emote, onclose: () => {}, head: false });
    expect(screen.queryByRole('button', { name: 'Close' })).toBeNull();
    expect(screen.queryByText('Selected emote')).toBeNull();
    expect(screen.getByText('beanSus')).toBeTruthy();
  });

  it('holds its place while the emote loads, and says so when it can’t', () => {
    const { container, unmount } = render(EmoteInspector, { emote: null, onclose: () => {} });
    expect(container.querySelector('[aria-busy="true"]')).toBeTruthy();
    unmount();
    render(EmoteInspector, { emote: null, failed: true, onclose: () => {} });
    expect(screen.getByText('Couldn’t load this emote.')).toBeTruthy();
  });
});
