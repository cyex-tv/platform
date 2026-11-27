// The emote page's tabs follow the address: `?tab=activity` opens Activity (a link or a reload),
// a click goes to the tab's address (the first tab's is clean) without scrolling, arrows replace
// the entry instead of adding one, and an unknown tab is tidied to the clean address.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EmoteView from './EmoteView.svelte';
import type { PageEmote } from './emote-page';

const { gql, goto, here } = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn(), here: { url: new URL('http://x/emotes/E1') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: here }));

const file = (scale: number, isStatic = false) => ({
  name: `${scale}x${isStatic ? '_static' : ''}.webp`, format: 'WEBP' as const, scale, width: 32 * scale, height: 32 * scale, size: 900, isStatic,
});
const emote = {
  id: 'E1', name: 'fernYawn', state: 'APPROVED', animated: false, tags: [], host: 'https://cdn.example/emotes/1',
  createdAt: '2026-11-17T10:00:00.000Z', channelCount: 2, visibility: 'PUBLIC',
  flags: { zeroWidth: false, private: false, wide: false },
  owner: { id: 'A1', displayName: 'nora_byte', avatar: null },
  files: [1, 2, 3, 4].flatMap((s) => [file(s), file(s, true)]),
} as unknown as PageEmote;
const channels = [{ platform: 'TWITCH' as const, login: 'c1', accountId: 'C1', displayName: 'chanOne', avatar: null }];
const view = () =>
  render(EmoteView, { emote, more: [], channels, channelCount: 1, back: { href: '/search', label: 'All emotes', back: false } });
const selected = () => screen.getAllByRole('tab').find((t) => t.getAttribute('aria-selected') === 'true')?.id;

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(() => new Promise(() => {}));
  goto.mockReset();
});

describe('emote page tabs in the address', () => {
  it('opens the first tab on a clean address and asks for no change', () => {
    here.url = new URL('http://x/emotes/E1');
    view();
    expect(selected()).toBe('tab-channels');
    expect(goto).not.toHaveBeenCalled();
  });

  it('opens the tab the address names (a link or a reload)', () => {
    here.url = new URL('http://x/emotes/E1?tab=activity');
    view();
    expect(selected()).toBe('tab-activity');
    expect(screen.getByRole('tabpanel', { name: /Activity/ })).toBeTruthy();
    expect(goto).not.toHaveBeenCalled();
  });

  it('a click goes to the tab’s address, keeping scroll and focus; the first tab’s is clean', async () => {
    here.url = new URL('http://x/emotes/E1');
    const user = userEvent.setup();
    view();
    await user.click(screen.getByRole('tab', { name: /Activity/ }));
    expect(goto).toHaveBeenLastCalledWith('/emotes/E1?tab=activity', { noScroll: true, keepFocus: true, replaceState: false });
    goto.mockReset();
    here.url = new URL('http://x/emotes/E1?tab=activity');
    view();
    await user.click(screen.getAllByRole('tab', { name: /Channels/ }).at(-1)!);
    expect(goto).toHaveBeenLastCalledWith('/emotes/E1', { noScroll: true, keepFocus: true, replaceState: false });
  });

  it('arrow keys replace the entry instead of adding one per step', async () => {
    here.url = new URL('http://x/emotes/E1');
    const user = userEvent.setup();
    view();
    screen.getByRole('tab', { name: /Channels/ }).focus();
    await user.keyboard('{ArrowRight}');
    expect(goto).toHaveBeenLastCalledWith('/emotes/E1?tab=activity', { noScroll: true, keepFocus: true, replaceState: true });
  });

  it('falls back to the first tab for an unknown one and tidies the address', () => {
    here.url = new URL('http://x/emotes/E1?tab=nope');
    view();
    expect(selected()).toBe('tab-channels');
    expect(goto).toHaveBeenCalledWith('/emotes/E1', { noScroll: true, keepFocus: true, replaceState: true });
  });
});

// The Inspector layout (redesign, 2026/11/21): the preview column and the facts column.
describe('emote page layout', () => {
  it('shows one neutral stage with every size (the largest in focus) and a chat line', () => {
    here.url = new URL('http://x/emotes/E1');
    const { container } = view();
    const stage = container.querySelector('[data-testid="emote-stage"]')!;
    expect(stage.querySelector('.focus img')?.getAttribute('src')).toBe('https://cdn.example/emotes/1/4x.webp');
    expect(stage.querySelectorAll('img')).toHaveLength(4);
    expect(container.querySelector('.chatline img')?.getAttribute('alt')).toBe('fernYawn');
  });

  it('puts the name, the creator, the facts and the tabs in the right column', () => {
    here.url = new URL('http://x/emotes/E1');
    const { container } = view();
    const right = container.querySelector('.right')!;
    expect(right.querySelector('h1')?.textContent).toBe('fernYawn');
    expect(right.querySelector('a.creator')?.getAttribute('href')).toBe('/users/A1');
    const facts = [...right.querySelectorAll('.facts dt')].map((d) => d.textContent);
    expect(facts).toEqual(['Channels', 'Uploaded', 'Public listing', 'Personal use']);
    expect(right.querySelector('[role="tablist"]')).toBeTruthy();
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' }).textContent).toMatch(/All emotes\s*\/\s*fernYawn/);
  });
});
