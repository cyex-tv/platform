import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { session } from '$lib/tool/session.svelte';

const { gql, goto } = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/users/ME?s=sets') } }));
vi.mock('$app/navigation', () => ({ goto, invalidateAll: vi.fn(), afterNavigate: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

import SetActions, { forgetSetMenus } from './SetActions.svelte';

const SET = { id: 'S1', name: 'Main', kind: 'CHANNEL', ownerId: 'ME', capacity: 1000 };
const RANGE = { min: 3, max: 1000, ownerMax: 1000, used: 3, staff: false };
const MENU = {
  setCapacityRange: RANGE,
  editorOf: [],
  myChannels: [{ platform: 'TWITCH', platformChannelId: '1', login: 'lumen_fox', activeSetId: null, account: { id: 'ME' } }],
  editorChannels: [],
};

type Actions = { open: (s: typeof SET, at: MouseEvent | HTMLElement) => void };
const opName = (doc: unknown) => String(doc).match(/(?:query|mutation)\s+(\w+)/)?.[1];

function setup(props: Record<string, unknown> = {}) {
  const onrename = vi.fn();
  const onchanged = vi.fn();
  const r = render(SetActions, { onrename, onchanged, ...props });
  const button = document.createElement('button');
  button.textContent = 'menu';
  document.body.append(button);
  const open = (set = SET) => (r.component as unknown as Actions).open(set, button);
  return { onrename, onchanged, open, button };
}

beforeEach(() => {
  forgetSetMenus();
  gql.mockReset();
  goto.mockReset();
  gql.mockImplementation(async (doc: unknown, vars: Record<string, unknown>) => {
    switch (opName(doc)) {
      case 'SetMenu':
        return MENU;
      case 'SetCapacityRange':
        return { setCapacityRange: RANGE };
      case 'UpdateSet':
        return { updateSet: { id: 'S1', name: 'Main', visibility: 'LISTED', capacity: (vars.input as { capacity: number }).capacity } };
      case 'DeleteSet':
        return { deleteSet: true };
      case 'UpdateChannel':
        return { updateChannelSettings: { activeSetId: 'S1', allowPersonalEmotes: true } };
    }
    throw new Error(`unexpected ${opName(doc)}`);
  });
  session.me = { id: 'ME', permissions: [] } as unknown as typeof session.me;
  session.known = true;
  HTMLElement.prototype.showPopover = function () {};
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
});

const items = () => screen.getAllByRole('menuitem').map((el) => el.textContent?.replace(/\s+/g, ' ').trim());

describe('set menu', () => {
  it('offers rename, link, set active, capacity, open and delete to the owner', async () => {
    const { open } = setup();
    open();
    await waitFor(() => expect(items()).toContain('Delete…'));
    expect(items()).toEqual([
      'Rename',
      'Copy link',
      'Set active on Twitch lumen_fox',
      'Change capacity…',
      'Open set page',
      'Open in editor',
      'Delete…',
    ]);
    expect(gql).toHaveBeenCalledWith(expect.stringContaining('query SetMenu'), { id: 'S1' });
  });

  it('shows only link and open to someone who may do nothing with it', async () => {
    MENU.setCapacityRange = null as never;
    const { open } = setup();
    open({ ...SET, ownerId: 'SOMEONE' });
    try {
      await waitFor(() => expect(screen.queryByText('Checking what you can do…')).toBeNull());
      expect(items()).toEqual(['Copy link', 'Open set page']);
    } finally {
      MENU.setCapacityRange = RANGE;
    }
  });

  it('hands Rename back to the page, which renames in place', async () => {
    const { open, onrename } = setup();
    open();
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'Rename' }));
    expect(onrename).toHaveBeenCalledWith('S1');
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('sets the set active on a channel', async () => {
    const { open, onchanged } = setup();
    open();
    await fireEvent.click(await screen.findByRole('menuitem', { name: /Set active on Twitch lumen_fox/ }));
    await waitFor(() => expect(onchanged).toHaveBeenCalledWith({ kind: 'active', id: 'S1' }));
    expect(gql).toHaveBeenCalledWith(expect.stringContaining('mutation UpdateChannel'), {
      platform: 'TWITCH', id: '1', input: { activeSetId: 'S1' },
    });
  });

  it('changes the capacity in a dialog that shows the allowed range and refuses what is outside it', async () => {
    const user = userEvent.setup();
    const { open, onchanged } = setup();
    open();
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'Change capacity…' }));
    const field = await screen.findByLabelText('Capacity');
    expect(screen.getByText('3 to 1.000 slots: it holds 3.')).toBeTruthy();
    await user.clear(field);
    await user.type(field, '1001');
    expect(await screen.findByText('Up to 1.000.')).toBeTruthy();
    await user.clear(field);
    await user.type(field, '2');
    expect(await screen.findByText(/holds 3 emotes/)).toBeTruthy();
    expect(gql).not.toHaveBeenCalledWith(expect.stringContaining('mutation UpdateSet'), expect.anything());
    await user.clear(field);
    await user.type(field, '500');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(onchanged).toHaveBeenCalledWith({ kind: 'capacity', id: 'S1', capacity: 500 }));
    expect(gql).toHaveBeenCalledWith(expect.stringContaining('mutation UpdateSet'), { id: 'S1', input: { capacity: 500 } });
    expect(document.querySelector('dialog[open]')).toBeNull();
  });

  it('deletes only after the confirmation that says it can’t be undone', async () => {
    const { open, onchanged } = setup();
    open();
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'Delete…' }));
    expect(screen.getByText(/can’t be undone/)).toBeTruthy();
    expect(gql).not.toHaveBeenCalledWith(expect.stringContaining('mutation DeleteSet'), expect.anything());
    await fireEvent.click(screen.getByRole('button', { name: 'Delete set' }));
    await waitFor(() => expect(onchanged).toHaveBeenCalledWith({ kind: 'deleted', id: 'S1' }));
  });

  it('opens the set page and the editor', async () => {
    const { open } = setup();
    open();
    await fireEvent.click(await screen.findByRole('menuitem', { name: 'Open in editor' }));
    expect(goto).toHaveBeenCalledWith('/me/sets/S1');
  });

  it('moves with the arrows and closes on Escape, focus back on the button', async () => {
    const { open, button } = setup();
    open();
    await screen.findByRole('menuitem', { name: 'Delete…' });
    await tick();
    const menu = screen.getByRole('menu');
    await fireEvent.keyDown(menu, { key: 'End' });
    expect(document.activeElement?.textContent).toMatch('Delete');
    await fireEvent.keyDown(menu, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(button);
  });
});
