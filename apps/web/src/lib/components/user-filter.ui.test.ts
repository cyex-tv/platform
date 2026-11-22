import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import UserFilter from './UserFilter.svelte';
import type { Person } from './user-filter';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const person = (id: string, name: string, count?: number): Person => ({
  account: { id, displayName: name, avatar: null },
  identities: [{ platform: 'TWITCH', login: name.toLowerCase() }],
  ...(count === undefined ? {} : { count }),
});
const TOP = [person('A1', 'Ana', 9), person('B2', 'Bo', 4), person('C3', 'Cy', 2), person('D4', 'Di', 2), person('E5', 'Ed', 1), person('F6', 'Fay', 1)];

beforeEach(() => {
  gql.mockReset();
  // Names and looks the field asks for on its own (accountCosmetics, account): nothing to show.
  gql.mockResolvedValue({ accountCosmetics: [], account: null });
});
afterEach(() => cleanup());

const box = () => screen.getByRole('combobox') as HTMLInputElement;
const options = () => screen.queryAllByRole('option');
const names = () => options().map((o) => o.querySelector('.nm')?.textContent?.trim());
const active = () => box().getAttribute('aria-activedescendant');

function setup(props: Partial<Parameters<typeof UserFilter>[1]> = {}) {
  const onchange = vi.fn();
  const top = vi.fn(async () => TOP);
  const search = vi.fn(async (q: string) => [person('S1', `Sam${q}`), person('S2', `Sue${q}`)]);
  render(UserFilter, { label: 'By', value: '', onchange, top, search, ...props });
  return { onchange, top, search, user: userEvent.setup() };
}

describe('UserFilter', () => {
  it('shows nothing until it is focused', async () => {
    const { top } = setup();
    expect(box().getAttribute('aria-expanded')).toBe('false');
    expect(options()).toHaveLength(0);
    expect(top).not.toHaveBeenCalled();
  });

  it('focused and empty: the 5 most important people, pinned first, with their counts', async () => {
    const { user, top } = setup({ pinned: [person('Z9', 'Owner')] });
    await user.click(box());
    await vi.waitFor(() => expect(options()).toHaveLength(5));
    expect(top).toHaveBeenCalledTimes(1);
    expect(names()).toEqual(['Owner', 'Ana', 'Bo', 'Cy', 'Di']);
    expect(box().getAttribute('aria-expanded')).toBe('true');
    expect(screen.getByRole('listbox', { name: 'Most active here' })).toBeTruthy();
    expect(options()[1].textContent).toContain('9');
  });

  it('hides the list again when focus leaves, and asks for the top people once per key', async () => {
    const { user, top } = setup();
    await user.click(box());
    await vi.waitFor(() => expect(options()).toHaveLength(5));
    await user.tab();
    expect(box().getAttribute('aria-expanded')).toBe('false');
    await user.click(box());
    await vi.waitFor(() => expect(box().getAttribute('aria-expanded')).toBe('true'));
    expect(top).toHaveBeenCalledTimes(1);
  });

  it('falls back to the given people when nobody is counted', async () => {
    const { user } = setup({ top: vi.fn(async () => []), fallback: [person('E1', 'Editor')] });
    await user.click(box());
    await vi.waitFor(() => expect(names()).toEqual(['Editor']));
  });

  it('typing lists the matches after the pause, in one request', async () => {
    const { user, search } = setup();
    await user.click(box());
    await user.type(box(), 'ab');
    await vi.waitFor(() => expect(names()).toEqual(['Samab', 'Sueab']));
    expect(search).toHaveBeenCalledTimes(1);
    expect(search.mock.calls[0][0]).toBe('ab');
  });

  it('cancels a stale request: only the newest answer shows', async () => {
    const signals: AbortSignal[] = [];
    let release: (() => void) | undefined;
    const search = vi.fn((q: string, signal: AbortSignal) => {
      signals.push(signal);
      if (q === 'ab') return new Promise<Person[]>((ok) => (release = () => ok([person('OLD', 'Old')])));
      return Promise.resolve([person('NEW', 'New')]);
    });
    const { user } = setup({ search });
    await user.click(box());
    await user.type(box(), 'ab');
    await vi.waitFor(() => expect(search).toHaveBeenCalledTimes(1));
    await user.type(box(), 'c');
    await vi.waitFor(() => expect(names()).toEqual(['New']));
    expect(signals[0].aborted).toBe(true);
    release?.();
    await new Promise((r) => setTimeout(r, 20));
    expect(names()).toEqual(['New']);
  });

  it('arrows move the active row and wrap; Enter picks it; focus stays in the field', async () => {
    const { user, onchange } = setup();
    await user.click(box());
    await vi.waitFor(() => expect(options()).toHaveLength(5));
    const ids = options().map((o) => o.id);
    await user.keyboard('{ArrowDown}');
    expect(active()).toBe(ids[0]);
    await user.keyboard('{ArrowUp}');
    expect(active()).toBe(ids[4]);
    await user.keyboard('{ArrowDown}{ArrowDown}');
    expect(active()).toBe(ids[1]);
    expect(document.activeElement).toBe(box());
    await user.keyboard('{Enter}');
    expect(onchange).toHaveBeenCalledWith('B2', expect.objectContaining({ account: expect.objectContaining({ id: 'B2' }) }));
  });

  it('Escape closes the list, a second clears the text', async () => {
    const { user } = setup();
    await user.click(box());
    await user.type(box(), 'ab');
    await vi.waitFor(() => expect(options()).toHaveLength(2));
    await user.keyboard('{Escape}');
    expect(box().getAttribute('aria-expanded')).toBe('false');
    expect(box().value).toBe('ab');
    await user.keyboard('{Escape}');
    expect(box().value).toBe('');
  });

  it('a pasted account ID can be used as it is', async () => {
    const { user, onchange } = setup({ ids: (t: string) => (/^\d{6,}$/.test(t) ? t : null) });
    await user.click(box());
    await user.type(box(), '1234567');
    await user.keyboard('{Enter}');
    expect(onchange).toHaveBeenCalledWith('1234567', null);
  });

  it('the picked person is a chip: remove clears the filter, the name reopens the search', async () => {
    const { user, onchange } = setup({ value: 'A1', pinned: [person('A1', 'Ana')] });
    expect(screen.queryByRole('combobox')).toBeNull();
    const remove = screen.getByRole('button', { name: 'Remove by filter' });
    await user.click(screen.getByRole('button', { name: /By: Ana/ }));
    await vi.waitFor(() => expect(document.activeElement).toBe(box()));
    await user.keyboard('{Escape}{Escape}');
    expect(remove).toBeTruthy();
    cleanup();
    const second = setup({ value: 'A1', pinned: [person('A1', 'Ana')] });
    await second.user.click(screen.getByRole('button', { name: 'Remove by filter' }));
    expect(second.onchange).toHaveBeenCalledWith('', null);
    expect(onchange).not.toHaveBeenCalled();
  });
});
