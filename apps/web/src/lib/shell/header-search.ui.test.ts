import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import HeaderSearch from './HeaderSearch.svelte';
import { headerSearch } from './search.svelte';
import { session } from '$lib/tool/session.svelte';
import { EDITABLE_CHANNELS } from '$lib/queries';

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
  session.me = null;
  session.known = false;
});

const box = () => screen.getByRole('combobox', { name: 'Search' }) as HTMLInputElement;
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

describe('HeaderSearch: the field is just "Search"', () => {
  it('is named and labelled Search, still finding emotes and people', async () => {
    const user = userEvent.setup();
    await search(user);
    expect(box().getAttribute('placeholder')).toBe('Search');
    expect(screen.getAllByRole('option').map((o) => o.getAttribute('href'))).toContain('/users/U1');
  });
});

describe('HeaderSearch: no recent searches', () => {
  beforeEach(() => localStorage.clear());

  it('forgets the old recent searches key on load and never lists or stores searches', async () => {
    localStorage.setItem('cyex.search.recent', JSON.stringify(['kappa', 'pog']));
    const user = userEvent.setup();
    await search(user);
    expect(localStorage.getItem('cyex.search.recent')).toBeNull();
    await user.keyboard('{Enter}');
    expect(goto).toHaveBeenCalledWith('/search?q=pog');
    expect(localStorage.getItem('cyex.search.recent')).toBeNull();
    cleanup();
    render(HeaderSearch);
    await user.click(box());
    expect(screen.queryAllByRole('option')).toHaveLength(0);
    expect(screen.queryByText(/Recent searches/)).toBeNull();
    expect(screen.getByText(/Type to search emotes by name, or people by login/)).toBeTruthy();
  });
});

describe('HeaderSearch: the empty field lists the channels you edit', () => {
  const chan = (role: string, login: string, platform = 'TWITCH') => ({ role, channel: { platform, login, account: { id: `A-${login}`, displayName: login, avatar: null } } });

  it('by role (owner, editor, set editor), then name A to Z, every one of them, in a capped list', async () => {
    session.me = { id: 'ME', identities: [], permissions: [], roles: [] } as never;
    session.known = true;
    const many = Array.from({ length: 40 }, (_, i) => chan('SET_EDITOR', `zz${String(i).padStart(2, '0')}`));
    gql.mockImplementation(async (doc: unknown) =>
      doc === EDITABLE_CHANNELS
        ? { myEditableChannels: [chan('SET_EDITOR', 'bob'), chan('ACCOUNT_EDITOR', 'Zed'), chan('OWNER', 'me'), chan('ACCOUNT_EDITOR', 'amy'), ...many] }
        : { quickSearch: { emotes: [], users: [] } },
    );
    const user = userEvent.setup();
    render(HeaderSearch);
    await user.click(box());
    await vi.waitFor(() => expect(screen.getAllByRole('option').length).toBe(44));
    const logins = screen.getAllByRole('option').map((o) => o.getAttribute('href')!.split('/').pop());
    expect(logins.slice(0, 5)).toEqual(['me', 'amy', 'Zed', 'bob', 'zz00']);
    expect(screen.queryByText(/Recent searches/)).toBeNull();
    expect(screen.getByRole('listbox').getAttribute('aria-label')).toBe('Channels you edit');
    // 44 rows never grow the page: the list scrolls inside its capped height.
    expect(screen.getByRole('listbox').classList.contains('list')).toBe(true);
  });
});
