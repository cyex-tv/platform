// The browse page's workspace (redesign, 2026/11/21): the set rail (groups that fold and stay
// folded per account, a filter past 8 sets, the target picked there, LIVE and how full), the
// tile's + (adds in place with Undo, refuses a full set, removes what the target holds), the tile
// menu's "Add to…" (every other set) and "Remove from" rows, the menu for a selection, dragging
// tiles onto a set in the rail, and the tray with what the target holds.
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { session } from '$lib/tool/session.svelte';
import { toasts } from '$lib/tool/notices.svelte';
import WorkspaceHarness from '$lib/test/WorkspaceHarness.svelte';
import SetRail from './SetRail.svelte';
import { target } from './target.svelte';
import { defaultShut, railKey } from './rail-state';
import { filterGroups, groupSets } from './set-filter';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/search') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn() }));

const ME = { id: 'ME', displayName: 'quietoak', avatar: null };
const NORA = { id: 'NORA', displayName: 'nora_byte', avatar: null };
const pic = (id: string, name: string) => ({ id, name, host: `https://cdn.example/${id}`, animated: false, files: [{ name: '1x.webp', width: 32, height: 32, isStatic: false }] });
const member = (id: string, name: string) => ({ name, zeroWidth: false, addedAt: '2026-11-01T00:00:00Z', emote: { ...pic(id, name), state: 'APPROVED' } });
const set = (id: string, name: string, owner: typeof ME, emotes: ReturnType<typeof member>[] = [], capacity = 1000) => ({
  id, name, kind: 'NORMAL', visibility: 'PUBLIC', capacity, owner, emotes,
});
let n = 0;
let calls: { op: string; setId: string; emoteId: string }[] = [];

/** `mine` of your own sets (the first is Main, shown on your channel), plus `theirs` of nora's. */
function answer(mine: number, theirs: string[] = [], over: { mainEmotes?: ReturnType<typeof member>[]; mainCap?: number; extra?: ReturnType<typeof set>[] } = {}) {
  const me = { ...ME };
  gql.mockImplementation(async (doc: string, vars: { setId: string; emoteId: string }) => {
    if (doc.includes('query MySets'))
      return {
        mySets: Array.from({ length: mine }, (_, i) =>
          i === 0 ? set('S0', 'Main', me, over.mainEmotes ?? [], over.mainCap ?? 1000) : set(`S${i}`, `Mine ${i}`, me),
        ).concat(over.extra ?? []),
        editorSets: theirs.map((name, i) => set(`N${i}`, name, NORA, i === 0 ? [member('E2', 'peaPaws')] : [])),
        editorOf: [{ account: NORA, permissions: ['EMOTES'] }],
      };
    if (doc.includes('query MyChannels'))
      return {
        myChannels: [
          { platform: 'TWITCH', platformChannelId: '1', login: 'quietoak_live', activeSetId: 'S0', allowPersonalEmotes: true, account: me },
          { platform: 'TWITCH', platformChannelId: '2', login: 'nora_live', activeSetId: 'N0', allowPersonalEmotes: true, account: NORA },
        ],
        mySets: [],
      };
    if (doc.includes('mutation AddSetEmote')) {
      calls.push({ op: 'add', ...vars });
      return { addSetEmote: { id: vars.setId } };
    }
    if (doc.includes('mutation RemoveSetEmote')) {
      calls.push({ op: 'remove', ...vars });
      return { removeSetEmote: { id: vars.setId } };
    }
    throw new Error(`unexpected ${doc.slice(0, 40)}`);
  });
}

const emotes = [pic('E1', 'fernYawn'), pic('E2', 'peaPaws'), pic('E3', 'toadLoaf')].map((e) => ({ ...e, owner: { id: 'X', displayName: 'kiwi_rae' } }));
const rows = () => [...document.querySelectorAll<HTMLButtonElement>('.rail .set')];
const rowNamed = (name: string) => rows().find((r) => r.querySelector('bdi')?.textContent === name)!;

beforeEach(() => {
  gql.mockReset();
  localStorage.clear();
  calls = [];
  toasts.splice(0, toasts.length);
  // A new account each test, so the shared target forgets the last one's sets.
  ME.id = `ME${++n}`;
  session.me = { ...ME } as unknown as typeof session.me;
  session.known = true;
});

async function rail(mine: number, theirs: string[] = [], over = {}) {
  answer(mine, theirs, over);
  render(SetRail);
  await waitFor(() => expect(rows().length).toBeGreaterThan(0));
}

describe('set rail', () => {
  it('groups your sets first, then each account you edit for, with LIVE and how full', async () => {
    await rail(2, ['Hype'], { mainEmotes: [member('E1', 'fernYawn')] });
    const heads = screen.getAllByRole('button', { expanded: true }).map((b) => b.textContent?.replace(/\s+/g, ' ').trim());
    expect(heads).toEqual(['Your sets 2', 'nora_byte · editor 1']);
    const main = rowNamed('Main');
    expect(main.textContent).toMatch(/Live/);
    expect(main.textContent).toMatch(/1\/1\.000/);
    // The account you edit for: its channel shows Hype, so Hype is LIVE too.
    expect(rowNamed('Hype').textContent).toMatch(/Live/);
    // Starts on your channel's active set, which is bold with the accent tick (aria-pressed).
    expect(main.getAttribute('aria-pressed')).toBe('true');
  });

  it('picks the target with a click, and remembers it on this device', async () => {
    const user = userEvent.setup();
    await rail(3);
    await user.click(rowNamed('Mine 2'));
    expect(target.current?.name).toBe('Mine 2');
    expect(rowNamed('Mine 2').getAttribute('aria-pressed')).toBe('true');
    expect(localStorage.getItem('cyex.browse.target')).toBe('S2');
  });

  it('folds a group, and remembers it per account', async () => {
    const user = userEvent.setup();
    await rail(2, ['Hype', 'Subs']);
    await user.click(screen.getByRole('button', { name: /nora_byte/ }));
    expect(screen.getByRole('button', { name: /nora_byte/ }).getAttribute('aria-expanded')).toBe('false');
    expect(rows().map((r) => r.dataset.set)).toEqual(['S0', 'S1']);
    expect(JSON.parse(localStorage.getItem(railKey(ME.id))!)).toEqual(['NORA']);
  });

  it('opens folded the way it was left', async () => {
    localStorage.setItem(`cyex.browse.rail.ME${n + 1}`, JSON.stringify(['mine']));
    ME.id = `ME${n + 1}`;
    session.me = { ...ME } as unknown as typeof session.me;
    answer(2, ['Hype']);
    render(SetRail);
    await waitFor(() => expect(screen.getByRole('button', { name: /Your sets/ }).getAttribute('aria-expanded')).toBe('false'));
    expect(rows().map((r) => r.dataset.set)).toEqual(['N0']);
  });

  it('has no filter with 8 sets or fewer', async () => {
    await rail(8);
    expect(screen.queryByRole('searchbox', { name: 'Find a set' })).toBeNull();
  });

  it('past 8 sets filters by set, account and channel', async () => {
    const user = userEvent.setup();
    await rail(7, ['Hype', 'Subs only']);
    const find = screen.getByRole('searchbox', { name: 'Find a set' });
    await user.type(find, 'subs');
    expect(rows().map((r) => r.dataset.set)).toEqual(['N1']);
    await user.clear(find);
    await user.type(find, 'quietoak_live main');
    expect(rows().map((r) => r.dataset.set)).toEqual(['S0']);
    await user.clear(find);
    await user.type(find, 'zzz');
    expect(screen.getByText('No sets match')).toBeTruthy();
    await user.clear(find);
    await user.type(find, 'hype{Enter}');
    expect(target.current?.name).toBe('Hype');
  });

  it('walks with the arrows', async () => {
    const user = userEvent.setup();
    await rail(3);
    screen.getByRole('button', { name: /Your sets/ }).focus();
    await user.keyboard('{ArrowDown}{ArrowDown}');
    expect((document.activeElement as HTMLElement).dataset.set).toBe('S1');
    await user.keyboard('{End}');
    expect((document.activeElement as HTMLElement).dataset.set).toBe('S2');
    await user.keyboard('{Enter}');
    expect(target.current?.id).toBe('S2');
  });
});

describe('rail state', () => {
  const g = (key: string, sets: number) => ({ key, sets: Array(sets).fill(0) });
  it('starts a small rail open and a big one folded but for your sets and the target’s group', () => {
    expect([...defaultShut([g('mine', 3), g('A', 2)], 'mine')]).toEqual([]);
    expect([...defaultShut([g('mine', 10), g('A', 10), g('B', 10)], 'B')]).toEqual(['A']);
    expect([...defaultShut([g('mine', 1), g('A', 1), g('B', 1), g('C', 1), g('D', 1)], 'mine')]).toEqual(['A', 'B', 'C', 'D']);
  });
  it('groups and filters like before', () => {
    const s = (id: string, name: string, mine: boolean, ownerName = 'x', channels: string[] = []) => ({ id, name, ownerId: mine ? 'me' : ownerName, ownerName, mine, channels });
    const groups = groupSets([s('1', 'Café Hype', true, 'me', ['mylive']), s('2', 'a', false, 'zed'), s('3', 'c', false, 'amy')]);
    expect(groups.map((x) => x.label)).toEqual(['Your sets', 'amy', 'zed']);
    expect(filterGroups(groups, 'CAFE mylive')[0].sets.map((x) => x.id)).toEqual(['1']);
  });
});

async function workspace(mine = 2, theirs = ['Hype'], over = {}) {
  answer(mine, theirs, over);
  render(WorkspaceHarness, { items: emotes });
  await waitFor(() => expect(rows().length).toBeGreaterThan(0));
}
const tile = (name: string) => [...document.querySelectorAll<HTMLElement>('li[data-id]')].find((li) => li.textContent?.includes(name))!;
const lastToast = () => toasts.at(-1);
// jsdom has no DataTransfer: a small one that keeps strings by type.
class FakeTransfer {
  #data = new Map<string, string>();
  dropEffect = 'none';
  effectAllowed = 'all';
  get types() {
    return [...this.#data.keys()];
  }
  setData(type: string, value: string) {
    this.#data.set(type, value);
  }
  getData(type: string) {
    return this.#data.get(type) ?? '';
  }
  setDragImage() {}
}

describe('the tile’s +', () => {
  it('adds to the target in place, says so with Undo, and Undo takes it out again', async () => {
    const user = userEvent.setup();
    await workspace();
    await user.click(within(tile('fernYawn')).getByRole('button', { name: 'Add fernYawn to Main' }));
    await waitFor(() => expect(calls).toEqual([{ op: 'add', setId: 'S0', emoteId: 'E1' }]));
    expect(lastToast()?.text).toBe('Added fernYawn to Main.');
    // The tile now says it is in Main, and its button removes it.
    expect(within(tile('fernYawn')).getByRole('button', { name: 'Remove fernYawn from Main' }).getAttribute('aria-pressed')).toBe('true');
    expect(tile('fernYawn').textContent).toMatch(/In\s*Main/);
    // The tray lists it, newest first.
    expect(screen.getByRole('list', { name: 'Emotes in Main' }).textContent).toMatch(/fernYawn/);
    lastToast()!.action!.run!();
    await waitFor(() => expect(calls.at(-1)).toEqual({ op: 'remove', setId: 'S0', emoteId: 'E1' }));
    await waitFor(() => expect(within(tile('fernYawn')).getByRole('button', { name: 'Add fernYawn to Main' })).toBeTruthy());
  });

  it('refuses a full set without asking the API', async () => {
    const user = userEvent.setup();
    await workspace(2, [], { mainEmotes: [member('E9', 'other')], mainCap: 1 });
    await user.click(within(tile('fernYawn')).getByRole('button', { name: 'Add fernYawn to Main' }));
    expect(calls).toEqual([]);
    expect(lastToast()?.text).toMatch(/Main is full \(1\/1\)/);
    expect(lastToast()?.error).toBe(true);
  });
});

describe('the tile menu', () => {
  it('lists every other set under "Add to…" and adds there', async () => {
    const user = userEvent.setup();
    await workspace(3, ['Hype']);
    await fireEvent.contextMenu(tile('fernYawn').querySelector('.tile')!, { clientX: 40, clientY: 40 });
    const menu = await screen.findByRole('menu');
    await user.click(within(menu).getByRole('menuitem', { name: /Add to…/ }));
    const sets = within(screen.getByRole('menu')).getAllByRole('menuitem').map((i) => i.textContent?.replace(/\s+/g, ' ').trim());
    expect(sets[0]).toMatch(/Add fernYawn to…/);
    expect(sets.slice(1).map((s) => s?.split(' ')[0])).toEqual(['Mine', 'Mine', 'HypeLive']);
    await user.click(within(screen.getByRole('menu')).getByRole('menuitem', { name: /^Hype/ }));
    await waitFor(() => expect(calls).toEqual([{ op: 'add', setId: 'N0', emoteId: 'E1' }]));
    expect(lastToast()?.text).toBe('Added fernYawn to nora_byte · Hype.');
  });

  it('offers Remove from each other set that holds the emote', async () => {
    const user = userEvent.setup();
    await workspace(2, ['Hype']);
    await fireEvent.contextMenu(tile('peaPaws').querySelector('.tile')!, { clientX: 40, clientY: 40 });
    const menu = await screen.findByRole('menu');
    await user.click(within(menu).getByRole('menuitem', { name: /Remove from nora_byte · Hype/ }));
    await waitFor(() => expect(calls).toEqual([{ op: 'remove', setId: 'N0', emoteId: 'E2' }]));
  });

  it('opens "Add to…" with the right arrow and comes back with the left', async () => {
    const user = userEvent.setup();
    await workspace(2, ['Hype']);
    await fireEvent.contextMenu(tile('fernYawn').querySelector('.tile')!, { clientX: 40, clientY: 40 });
    await screen.findByRole('menu');
    const sub = within(screen.getByRole('menu')).getByRole('menuitem', { name: /Add to…/ });
    sub.focus();
    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('menu').getAttribute('aria-label')).toBe('Add fernYawn to');
    await user.keyboard('{ArrowLeft}');
    expect(screen.getByRole('menu').getAttribute('aria-label')).toBe('fernYawn');
    expect(document.activeElement?.textContent).toMatch(/Add to…/);
  });

  it('acts on every picked emote when a picked tile is right-clicked in Select mode', async () => {
    const user = userEvent.setup();
    await workspace(2, ['Hype']);
    await fireEvent.contextMenu(tile('fernYawn').querySelector('.tile')!, { clientX: 40, clientY: 40 });
    await user.click(within(await screen.findByRole('menu')).getByRole('menuitem', { name: 'Select' }));
    await user.click(tile('toadLoaf').querySelector('.tile')!);
    await fireEvent.contextMenu(tile('toadLoaf').querySelector('.tile')!, { clientX: 40, clientY: 40 });
    const menu = await screen.findByRole('menu');
    expect(menu.getAttribute('aria-label')).toBe('2 emotes');
    await user.click(within(menu).getByRole('menuitem', { name: /Add 2 to Main/ }));
    await waitFor(() => expect(calls.map((c) => c.emoteId)).toEqual(['E1', 'E3']));
    expect(lastToast()?.text).toBe('Added 2 to Main.');
  });
});

describe('dragging onto the rail', () => {
  it('adds a dragged tile to the set it is dropped on', async () => {
    await workspace(3);
    const dt = new FakeTransfer() as unknown as DataTransfer;
    await fireEvent.dragStart(tile('toadLoaf').querySelector('.tile')!, { dataTransfer: dt });
    const row = rowNamed('Mine 2');
    await fireEvent.dragOver(row, { dataTransfer: dt });
    expect(row.classList.contains('over')).toBe(true);
    await fireEvent.drop(row, { dataTransfer: dt });
    await waitFor(() => expect(calls).toEqual([{ op: 'add', setId: 'S2', emoteId: 'E3' }]));
    expect(row.textContent).toMatch(/1\/1\.000/);
  });

  it('ignores drags that are not emotes', async () => {
    await workspace(2);
    const dt = new FakeTransfer() as unknown as DataTransfer;
    dt.setData('text/plain', 'hello');
    const row = rowNamed('Mine 1');
    const over = new Event('dragover', { bubbles: true, cancelable: true }) as DragEvent;
    Object.defineProperty(over, 'dataTransfer', { value: dt });
    row.dispatchEvent(over);
    expect(over.defaultPrevented).toBe(false);
  });
});

describe('the tray', () => {
  it('lists what the target holds, newest first, and removes with Undo', async () => {
    const user = userEvent.setup();
    await workspace(2, [], { mainEmotes: [member('E1', 'fernYawn'), { ...member('E3', 'toadLoaf'), addedAt: '2026-11-05T00:00:00Z' }] });
    const list = screen.getByRole('list', { name: 'Emotes in Main' });
    expect([...list.querySelectorAll('a')].map((a) => a.textContent)).toEqual(['toadLoaf', 'fernYawn']);
    await user.click(within(list).getByRole('button', { name: 'Remove toadLoaf from Main' }));
    await waitFor(() => expect(calls).toEqual([{ op: 'remove', setId: 'S0', emoteId: 'E3' }]));
    expect([...list.querySelectorAll('a')].map((a) => a.textContent)).toEqual(['fernYawn']);
    expect(lastToast()?.text).toBe('Removed toadLoaf from Main.');
    lastToast()!.action!.run!();
    await waitFor(() => expect(calls.at(-1)).toEqual({ op: 'add', setId: 'S0', emoteId: 'E3' }));
  });

  it('folds to a strip and opens again', async () => {
    const user = userEvent.setup();
    await workspace();
    await user.click(screen.getByRole('button', { name: 'Hide the set' }));
    expect(screen.queryByRole('list', { name: 'Emotes in Main' })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Show Main' }));
    expect(screen.getByText(/free slots/)).toBeTruthy();
  });
});
