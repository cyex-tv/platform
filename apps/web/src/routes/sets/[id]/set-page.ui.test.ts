// The public set page in the Inspector shape (redesign, 2026/11/21): the panel holds the actions,
// the facts and the set's editors (only for whoever the API answers: here the owner); no chat preview
// and no visibility row (every set is public). The grid is the browse grid, with its Select switch.
import { cleanup, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { session } from '$lib/tool/session.svelte';
import Page from './+page.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/sets/S1'), state: {} } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn(), replaceState: vi.fn() }));

const OWNER = { id: 'OWN', displayName: 'quietoak', avatar: null };
const pic = (id: string, name: string) => ({ id, name, host: `https://cdn.example/${id}`, animated: false, files: [{ name: '1x.webp', width: 32, height: 32, isStatic: false }], owner: OWNER, flags: 0, state: 'APPROVED' });
const member = (id: string, name: string) => ({ name, zeroWidth: false, addedAt: '2026-11-01T00:00:00Z', emote: pic(id, name) });
const SET = {
  id: 'S1', name: 'Main', kind: 'CHANNEL', visibility: 'LISTED', capacity: 1000, slotsUsed: 2, createdAt: '2026-11-10T10:00:00Z', owner: OWNER,
  emotes: [member('E1', 'Pog'), member('E2', 'Kappa')],
};
// The page's own load result, loosely typed for the test (enums as strings).
const data = { id: 'S1', set: SET, meta: {} } as unknown as import('./$types').PageData;

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(async (doc: string) => {
    if (doc.includes('query SetEditors')) return { setEditors: [{ id: 'NORA', displayName: 'nora_byte', avatar: null }] };
    if (doc.includes('query MySets')) return { mySets: [], editorSets: [], editorOf: [] };
    return {};
  });
  session.known = true;
});
afterEach(() => {
  cleanup();
  session.me = null;
});

describe('set page', () => {
  it('shows the actions and facts in the panel (no chat preview), the set menu button, and the grid with Select', async () => {
    session.me = null;
    render(Page, { data });
    const panel = screen.getByRole('complementary', { name: 'About Main' });
    expect(within(panel).getByRole('heading', { name: 'Actions' })).toBeTruthy();
    expect(within(panel).getAllByRole('link', { name: 'Sign in to clone' }).length).toBe(1);
    expect(within(panel).getByRole('heading', { name: 'Facts' })).toBeTruthy();
    expect(within(panel).getByText('Channel set')).toBeTruthy();
    expect(within(panel).queryByText('Listed')).toBeNull();
    expect(within(panel).getByRole('meter', { name: 'How full' })).toBeTruthy();
    expect(within(panel).queryByRole('heading', { name: 'Chat preview' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Set actions for Main' })).toBeTruthy();
    // Signed out: no editors block, and nobody is asked for them.
    expect(within(panel).queryByRole('heading', { name: /Editors/ })).toBeNull();
    expect(gql.mock.calls.some(([d]) => String(d).includes('query SetEditors'))).toBe(false);
    expect(screen.getByRole('switch', { name: 'Select' })).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Emotes in Main' })).toBeTruthy();
  });

  it('for the owner: Edit set and the set’s editors', async () => {
    session.me = { id: 'OWN', identities: [{ displayName: 'quietoak' }] } as unknown as typeof session.me;
    render(Page, { data });
    const panel = screen.getByRole('complementary', { name: 'About Main' });
    expect(within(panel).getByRole('link', { name: 'Edit set' }).getAttribute('href')).toBe('/me/sets/S1');
    await waitFor(() => expect(within(panel).getByRole('heading', { name: /Editors/ })).toBeTruthy());
    expect(within(panel).getAllByText('nora_byte').length).toBeGreaterThan(0);
    expect(within(panel).getByRole('link', { name: 'Manage' }).getAttribute('href')).toBe('/me/editors');
  });

  it('turning Select on shows the selection bar; Escape with nothing picked turns it off', async () => {
    const user = userEvent.setup();
    render(Page, { data });
    await user.click(screen.getByRole('switch', { name: 'Select' }));
    expect((screen.getByRole('switch', { name: 'Select' }) as HTMLInputElement).checked).toBe(true);
    await waitFor(() => expect(screen.getByText(/selected/)).toBeTruthy());
    (document.activeElement as HTMLElement).blur();
    await user.keyboard('{Escape}');
    expect((screen.getByRole('switch', { name: 'Select' }) as HTMLInputElement).checked).toBe(false);
  });

  it('search inside the set narrows the grid and says how many match', async () => {
    const user = userEvent.setup();
    render(Page, { data });
    await user.type(screen.getByRole('searchbox', { name: 'Search this set' }), 'kap');
    expect(screen.getByText(/1 of 2/)).toBeTruthy();
  });
});
