// The target-set picker: a filter field once there are more than 8 sets (by set or channel name),
// groups per owner that fold and stay folded, arrows and Enter to pick, Escape clears then closes.
import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { session } from '$lib/tool/session.svelte';
import TargetSet from './TargetSet.svelte';
import { target } from './target.svelte';
import { filterGroups, groupSets } from './set-filter';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));

const ME = { id: 'ME', displayName: 'quietoak', avatar: null };
const NORA = { id: 'NORA', displayName: 'nora_byte', avatar: null };
const set = (id: string, name: string, owner: { id: string; displayName: string; avatar: null }) => ({
  id, name, kind: 'NORMAL', visibility: 'PUBLIC', capacity: 1000, owner, emotes: [],
});
let n = 0;

function answer(mine: number, theirs: string[] = []) {
  const me = { ...ME };
  gql.mockImplementation(async (doc: string) => {
    if (doc.includes('query MySets'))
      return {
        mySets: Array.from({ length: mine }, (_, i) => set(`S${i}`, i === 0 ? 'Main' : `Mine ${i}`, me)),
        editorSets: theirs.map((name, i) => set(`N${i}`, name, NORA)),
        editorOf: [{ account: NORA, permissions: ['EMOTES'] }],
      };
    if (doc.includes('query MyChannels'))
      return { myChannels: [{ platform: 'TWITCH', platformChannelId: '1', login: 'quietoak_live', activeSetId: 'S0', allowPersonalEmotes: true, account: me }], mySets: [] };
    throw new Error('unexpected');
  });
}

async function openPicker() {
  const user = userEvent.setup();
  render(TargetSet);
  await waitFor(() => expect(target.sets).not.toBeNull());
  await user.click(screen.getByRole('button', { name: /Adding to/ }));
  return user;
}

const radios = () => screen.queryAllByRole('radio') as HTMLInputElement[];

beforeEach(() => {
  gql.mockReset();
  localStorage.clear();
  // A new account each test, so the shared target forgets the last one's sets.
  ME.id = `ME${++n}`;
  session.me = { ...ME } as unknown as typeof session.me;
  session.known = true;
});

describe('target set picker', () => {
  it('has no filter with 8 sets or fewer', async () => {
    answer(8);
    await openPicker();
    expect(screen.queryByRole('searchbox', { name: 'Find a set' })).toBeNull();
    expect(radios()).toHaveLength(8);
  });

  it('with more than 8 sets, focuses the filter and filters by set and by channel name', async () => {
    answer(7, ['Hype', 'Subs only']);
    const user = await openPicker();
    const find = screen.getByRole('searchbox', { name: 'Find a set' });
    expect(document.activeElement).toBe(find);
    await user.type(find, 'subs');
    expect(radios().map((r) => r.value)).toEqual(['N1']);
    // The account you edit for finds its sets, your channel finds yours.
    await user.clear(find);
    await user.type(find, 'nora');
    expect(radios().map((r) => r.value)).toEqual(['N0', 'N1']);
    await user.clear(find);
    await user.type(find, 'quietoak_live main');
    expect(radios().map((r) => r.value)).toEqual(['S0']);
  });

  it('says so when nothing matches', async () => {
    answer(9);
    const user = await openPicker();
    await user.type(screen.getByRole('searchbox', { name: 'Find a set' }), 'zzz');
    expect(screen.getByText('No sets match')).toBeTruthy();
    expect(radios()).toHaveLength(0);
  });

  it('picks the first match with Enter in the field', async () => {
    answer(7, ['Hype', 'Subs only']);
    const user = await openPicker();
    await user.type(screen.getByRole('searchbox', { name: 'Find a set' }), 'hype{Enter}');
    expect(target.current?.name).toBe('Hype');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('walks with the arrows without picking, and picks with Enter', async () => {
    answer(9);
    const user = await openPicker();
    const was = target.chosenId;
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}');
    const focused = document.activeElement as HTMLInputElement;
    expect(focused.type).toBe('radio');
    expect(focused.value).toBe('S2');
    expect(target.chosenId).toBe(was);
    await user.keyboard('{ArrowUp}');
    expect((document.activeElement as HTMLInputElement).value).toBe('S1');
    await user.keyboard('{Enter}');
    expect(target.chosenId).toBe('S1');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('Escape clears the filter first, then closes', async () => {
    answer(9);
    const user = await openPicker();
    const find = screen.getByRole('searchbox', { name: 'Find a set' }) as HTMLInputElement;
    await user.type(find, 'mine');
    await user.keyboard('{Escape}');
    expect(find.value).toBe('');
    expect(screen.getByRole('dialog')).toBeTruthy();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('folds a group and keeps it folded when opened again', async () => {
    answer(3, ['Hype']);
    const user = await openPicker();
    const head = screen.getByRole('button', { name: /nora_byte/ });
    expect(head.getAttribute('aria-expanded')).toBe('true');
    await user.click(head);
    expect(radios()).toHaveLength(3);
    await user.keyboard('{Escape}');
    await user.click(screen.getByRole('button', { name: /Adding to/ }));
    expect(screen.getByRole('button', { name: /nora_byte/ }).getAttribute('aria-expanded')).toBe('false');
    expect(radios()).toHaveLength(3);
  });
});

describe('set-filter', () => {
  const s = (id: string, name: string, mine: boolean, ownerName = 'x', channels: string[] = []) => ({ id, name, ownerId: mine ? 'me' : ownerName, ownerName, mine, channels });
  it('puts your sets first, then each account by name', () => {
    const g = groupSets([s('1', 'a', false, 'zed'), s('2', 'b', true), s('3', 'c', false, 'amy')]);
    expect(g.map((x) => x.label)).toEqual(['Your sets', 'amy', 'zed']);
  });
  it('ignores case and accents, needs every word', () => {
    const g = groupSets([s('1', 'Café Hype', true, 'me', ['mylive']), s('2', 'Other', true)]);
    expect(filterGroups(g, 'CAFE')[0].sets.map((x) => x.id)).toEqual(['1']);
    expect(filterGroups(g, 'hype mylive')[0].sets.map((x) => x.id)).toEqual(['1']);
    expect(filterGroups(g, 'hype nope')).toEqual([]);
  });
});
