// The profile's sections are separate chunks: Uploaded emotes is part of the page, every other tab
// loads when it is asked for. Moving to Emote sets (the address changes, no new page load) loads the
// section and shows its content; the page's load fetches the one the address asks for.
import { cleanup, render, screen } from '@testing-library/svelte';
import { SvelteURL } from 'svelte/reactivity';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Page from './[id]/+page.svelte';
import { forgetProfile } from '$lib/profile/data.svelte';
import { lazySection, preloadFor } from '$lib/profile/sections/lazy.svelte';
import { session } from '$lib/tool/session.svelte';

const { gql, url } = vi.hoisted(() => ({ gql: vi.fn(), url: { current: null as unknown as URL } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { get url() { return url.current; }, data: {} } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));

const data = {
  id: 'U1',
  profile: { identities: [], activeSets: [], editors: [], roles: [], createdAt: '2026-11-10T09:00:00Z' },
  creator: { id: 'U1', displayName: 'nora', avatar: null },
  look: { badge: null, paint: null },
  items: [],
  total: 0,
  error: null,
  meta: {},
};

beforeEach(() => {
  forgetProfile();
  gql.mockReset();
  gql.mockImplementation(async (doc: unknown) => {
    const q = String(doc);
    if (q.includes('accountSets')) return { accountSets: [{ id: 'S1', name: 'Winter set', kind: 'CHANNEL', capacity: 100, slotsUsed: 3, active: false, preview: [] }] };
    return {};
  });
  url.current = new SvelteURL('http://x/users/U1');
});
afterEach(() => {
  cleanup();
  session.me = null;
});

describe('profile tabs', () => {
  it('a tab opened after the page is there shows its section', async () => {
    render(Page, { data: data as never });
    expect(screen.getByRole('link', { name: /Emote sets/ })).toBeTruthy();
    // What the router does when a tab link is followed: the address changes, the page stays.
    url.current.searchParams.set('s', 'sets');
    expect(await screen.findByText('Winter set')).toBeTruthy();
    expect(screen.getByRole('link', { name: /Emote sets/ }).getAttribute('aria-current')).toBe('page');
    expect(screen.queryByText('Loading…')).toBeNull();
  });

  it('the page’s load fetches the section the address asks for: public ones on the server, all in the browser', async () => {
    await preloadFor('activity', true);
    expect(lazySection('activity')).toBeDefined();
    await preloadFor('account', true);
    expect(lazySection('account')).toBeUndefined();
    await preloadFor('account', false);
    expect(lazySection('account')).toBeDefined();
  });

  it('the owner always has an Overview tab, whichever section they are on and wherever they came from', async () => {
    session.me = { id: 'U1', createdAt: '2026-11-10T12:00:00.000Z', permissions: [], roles: [], identities: [] } as unknown as typeof session.me;
    for (const s of [null, 'sets', 'cosmetics']) {
      url.current = new SvelteURL(`http://x/users/U1${s ? `?s=${s}` : ''}`);
      render(Page, { data: data as never });
      const nav = screen.getByRole('navigation', { name: /profile$/ });
      const first = nav.querySelector('a');
      expect(first?.getAttribute('aria-label')).toBe('Overview');
      expect(first?.getAttribute('href')).toBe('/users/U1?s=overview');
      expect(first?.getAttribute('aria-current')).toBeNull();
      cleanup();
    }
  });

  it('everyone else has no Overview tab', () => {
    render(Page, { data: data as never });
    expect(screen.queryByRole('link', { name: 'Overview' })).toBeNull();
  });
});
