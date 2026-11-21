// The browse page's two shapes (redesign, 2026/11/21): signed out, a clean full-width grid with
// a short sign-in line and no set rail or tray; signed in, the rail and the tray around the grid,
// and the phone's "Adding to" button. Sorting and filters are there either way.
import { render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { session } from '$lib/tool/session.svelte';
import { parse } from '$lib/components/browse/params';
import Page from './+page.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/search'), state: {} }, navigating: { to: null } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), replaceState: vi.fn(), afterNavigate: vi.fn() }));

const items = [{ id: 'E1', name: 'fernYawn', host: 'https://cdn.example/1', animated: false, tags: [], files: [{ name: '1x.webp', width: 32, height: 32, isStatic: false }],
  owner: { id: 'X', displayName: 'kiwi_rae' }, flags: { zeroWidth: false } }];
const data = () => ({ view: parse(new URLSearchParams()), total: 1, items, pages: 1, error: null }) as never;

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(async (doc: string) => {
    if (doc.includes('query MySets')) return { mySets: [], editorSets: [], editorOf: [] };
    if (doc.includes('query MyChannels')) return { myChannels: [], mySets: [] };
    return new Promise(() => {});
  });
  localStorage.clear();
  document.documentElement.removeAttribute('data-signed');
});

describe('browse page', () => {
  it('signed out: no rail, no tray, no +, and a line offering sign in; sort and filters stay', () => {
    session.me = null;
    session.known = true;
    const { container } = render(Page, { data: data() });
    expect(container.querySelector('.ws')?.classList.contains('signed')).toBe(false);
    expect(container.querySelector('.rail-col')?.children.length).toBe(0);
    expect(container.querySelector('.tray-col')?.children.length).toBe(0);
    expect(screen.getByText(/to add emotes to your channel/).querySelector('a')?.getAttribute('href')).toBe('/signin?back=%2Fsearch');
    expect(container.querySelector('li[data-id] .add')).toBeNull();
    expect(screen.queryByRole('button', { name: /Adding to/ })).toBeNull();
    expect(screen.getByRole('button', { name: /Filters/ })).toBeTruthy();
    expect(screen.getByRole('radiogroup', { name: 'Sort' })).toBeTruthy();
  });

  it('signed in: the rail, the tray and the phone’s Adding to button', async () => {
    session.me = { id: 'ME-page', displayName: 'quietoak' } as unknown as typeof session.me;
    session.known = true;
    const { container } = render(Page, { data: data() });
    expect(container.querySelector('.ws')?.classList.contains('signed')).toBe(true);
    await waitFor(() => expect(container.querySelector('.rail-col .rail')).toBeTruthy());
    expect(container.querySelector('.tray-col')?.children.length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /Adding to/ })).toBeTruthy();
    expect(screen.queryByText(/to add emotes to your channel/)).toBeNull();
  });
});
