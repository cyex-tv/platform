import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import HeaderSearch from './HeaderSearch.svelte';
import { headerSearch } from './search.svelte';

const { goto, gql } = vi.hoisted(() => ({ goto: vi.fn(), gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a) }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const emote = (id: string, name: string) => ({ id, name, host: '', files: [], animated: false, owner: { displayName: 'Ana' } });
const person = { account: { id: 'U1', displayName: 'Ana', avatar: null }, identities: [{ platform: 'TWITCH', login: 'ana', displayName: 'Ana' }] };

beforeEach(() => {
  goto.mockReset();
  gql.mockReset();
  gql.mockResolvedValue({ quickSearch: { emotes: [emote('E1', 'Pog'), emote('E2', 'PogChamp')], users: [person] } });
  headerSearch.query = '';
});

const box = () => screen.getByRole('combobox', { name: 'Search emotes and people' }) as HTMLInputElement;
const active = () => box().getAttribute('aria-activedescendant');

async function search(user: ReturnType<typeof userEvent.setup>, text = 'pog') {
  render(HeaderSearch);
  await user.type(box(), text);
  await vi.waitFor(() => expect(screen.getAllByRole('option').length).toBe(4), { timeout: 2000 });
}

describe('HeaderSearch keyboard', () => {
  it('lists emotes, people and the all-results row, after the typing pause', async () => {
    const user = userEvent.setup();
    await search(user);
    expect(box().getAttribute('aria-expanded')).toBe('true');
    expect(gql).toHaveBeenCalledTimes(1);
    expect(screen.getAllByRole('option').map((o) => o.getAttribute('href'))).toEqual(['/emotes/E1', '/emotes/E2', '/users/U1', '/search?q=pog']);
  });

  it('arrows move the active row (and wrap), focus stays in the field', async () => {
    const user = userEvent.setup();
    await search(user);
    const ids = screen.getAllByRole('option').map((o) => o.id);
    expect(active()).toBeNull();
    await user.keyboard('{ArrowDown}');
    expect(active()).toBe(ids[0]);
    await user.keyboard('{ArrowDown}{ArrowDown}');
    expect(active()).toBe(ids[2]);
    await user.keyboard('{ArrowUp}');
    expect(active()).toBe(ids[1]);
    await user.keyboard('{ArrowUp}{ArrowUp}');
    expect(active()).toBe(ids[3]); // wrapped to the last row
    expect(document.activeElement).toBe(box());
    expect(screen.getAllByRole('option').filter((o) => o.getAttribute('aria-selected') === 'true')).toHaveLength(1);
  });

  it('Enter opens the active row', async () => {
    const user = userEvent.setup();
    await search(user);
    await user.keyboard('{ArrowDown}{ArrowDown}{Enter}');
    expect(goto).toHaveBeenCalledWith('/emotes/E2');
    expect(headerSearch.query).toBe('');
  });

  it('Enter with no active row searches for the text', async () => {
    const user = userEvent.setup();
    await search(user);
    await user.keyboard('{Enter}');
    expect(goto).toHaveBeenCalledWith('/search?q=pog');
  });

  it('Escape closes the list and keeps the text; a second Escape clears it', async () => {
    const user = userEvent.setup();
    await search(user);
    await user.keyboard('{Escape}');
    expect(box().getAttribute('aria-expanded')).toBe('false');
    expect(box().value).toBe('pog');
    await user.keyboard('{Escape}');
    expect(box().value).toBe('');
    expect(headerSearch.query).toBe('');
  });

  it('typing again after Escape opens the list again', async () => {
    const user = userEvent.setup();
    await search(user);
    await user.keyboard('{Escape}');
    await user.keyboard('x');
    expect(box().getAttribute('aria-expanded')).toBe('true');
  });

  it('asks nothing for a blank field', async () => {
    const user = userEvent.setup();
    render(HeaderSearch);
    await user.type(box(), '   ');
    await new Promise((r) => setTimeout(r, 300));
    expect(gql).not.toHaveBeenCalled();
  });
});

describe('HeaderSearch errors', () => {
  it('a new request clears the last one’s error line while it is pending', async () => {
    const user = userEvent.setup();
    gql.mockRejectedValueOnce(new Error('down'));
    render(HeaderSearch);
    await user.type(box(), 'p');
    await vi.waitFor(() => expect(screen.getByText(/Search isn.t available right now/)).toBeTruthy(), { timeout: 2000 });
    gql.mockReturnValueOnce(new Promise(() => {})); // the next request hangs
    await user.type(box(), 'o');
    expect(screen.queryByText(/Search isn.t available right now/)).toBeNull();
    expect(screen.getByText('Searching…')).toBeTruthy();
  });
});

describe('HeaderSearch recents', () => {
  beforeEach(() => localStorage.clear());

  it('remembers what was searched and lists it when the empty field is focused', async () => {
    const user = userEvent.setup();
    await search(user);
    await user.keyboard('{Enter}');
    expect(JSON.parse(localStorage.getItem('cyex.search.recent')!)).toEqual(['pog']);
    cleanup();
    render(HeaderSearch);
    await user.click(box());
    const opts = screen.getAllByRole('option');
    expect(opts.map((o) => o.textContent?.trim())).toEqual(['pog', 'Clear recent searches']);
    expect(opts[0].getAttribute('href')).toBe('/search?q=pog');
  });

  it('is reachable with the arrow keys, opens a recent one and clears them all', async () => {
    localStorage.setItem('cyex.search.recent', JSON.stringify(['kappa', 'pog']));
    const user = userEvent.setup();
    render(HeaderSearch);
    await user.click(box());
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}{Enter}'); // the clear row
    expect(screen.queryAllByRole('option')).toHaveLength(0);
    expect(localStorage.getItem('cyex.search.recent')).toBeNull();
    expect(goto).not.toHaveBeenCalled();
    cleanup();
    localStorage.setItem('cyex.search.recent', JSON.stringify(['kappa', 'pog']));
    render(HeaderSearch);
    await user.click(box());
    await user.keyboard('{ArrowDown}{ArrowDown}{Enter}');
    expect(goto).toHaveBeenCalledWith('/search?q=pog');
    expect(JSON.parse(localStorage.getItem('cyex.search.recent')!)[0]).toBe('pog');
  });

  it('stores nothing when the browser refuses, and still searches', async () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('denied'); });
    const user = userEvent.setup();
    await search(user);
    await user.keyboard('{Enter}');
    expect(goto).toHaveBeenCalledWith('/search?q=pog');
    vi.restoreAllMocks();
    expect(localStorage.getItem('cyex.search.recent')).toBeNull();
  });
});
