import { render, screen } from '@testing-library/svelte';
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
