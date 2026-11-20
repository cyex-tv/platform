import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { forgetMe, session } from '$lib/tool/session.svelte';
import { barCalls } from '../../test/bar-calls';
import SelectBarHarness from '../../test/SelectBarHarness.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const calls = () => barCalls;
const ids = (n: number) => Array.from({ length: n }, (_, i) => `E${i}`);

beforeEach(() => {
  Object.assign(barCalls, { all: 0, clear: 0, exit: 0 });
  gql.mockReset();
  forgetMe();
  session.known = true;
});

describe('SelectBar', () => {
  it('counts the selection against the cap', () => {
    render(SelectBarHarness, { ids: ids(3), shown: 10 });
    const region = screen.getByRole('region', { name: 'Selection' });
    expect(region.textContent).toMatch(/3\s*\/ 100 selected/);
  });

  it('shows the page note in place of the count', () => {
    render(SelectBarHarness, { ids: ids(100), shown: 200, note: 'You can pick 100 at a time.' });
    const region = screen.getByRole('region', { name: 'Selection' });
    expect(region.textContent).toContain('You can pick 100 at a time.');
    expect(region.textContent).not.toContain('selected');
  });

  it('offers Select all shown only while more can be picked, and Clear only with a selection', async () => {
    const user = userEvent.setup();
    const { rerender } = render(SelectBarHarness, { ids: [], shown: 10 });
    expect(screen.queryByRole('button', { name: 'Clear' })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Select all shown' }));
    expect(calls().all).toBe(1);

    await rerender({ ids: ids(4), shown: 10 });
    await user.click(screen.getByRole('button', { name: 'Clear' }));
    expect(calls().clear).toBe(1);

    await rerender({ ids: ids(10), shown: 10 });
    expect(screen.queryByRole('button', { name: 'Select all shown' })).toBeNull();
    // At the cap nothing more can be picked, however many are shown.
    await rerender({ ids: ids(100), shown: 500 });
    expect(screen.queryByRole('button', { name: 'Select all shown' })).toBeNull();
  });

  it('Done leaves multi-select', async () => {
    const user = userEvent.setup();
    render(SelectBarHarness, { ids: ids(1), shown: 5 });
    await user.click(screen.getByRole('button', { name: 'Done' }));
    expect(calls().exit).toBe(1);
  });

  it('signed out, adding asks you to sign in', () => {
    render(SelectBarHarness, { ids: ids(2), shown: 5 });
    const link = screen.getByRole('link', { name: 'Sign in to add' });
    expect(link.getAttribute('href')).toBe('/sign-in?next=/search');
  });

  it('signed in with a set, the button names the set and how many go into it', async () => {
    session.me = { id: 'A1', createdAt: '', permissions: [], identities: [] } as unknown as typeof session.me;
    gql.mockImplementation(async (doc: unknown) =>
      String(doc).includes('myChannels')
        ? { myChannels: [] }
        : { mySets: [{ id: 'S1', name: 'Main', kind: 'CHANNEL', capacity: 10, emotes: [], owner: null }], editorSets: [], editorOf: [] },
    );
    render(SelectBarHarness, { ids: ids(3), shown: 5 });
    const add = await screen.findByRole('button', { name: /Add 3 to\s*Main/ });
    expect((add as HTMLButtonElement).disabled).toBe(false);
  });
});
