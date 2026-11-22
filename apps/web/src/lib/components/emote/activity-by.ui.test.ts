// The emote page's Activity "By" filter keeps its state in the address (?by=): the feed is asked
// for that person's lines, and picking or removing someone rewrites the address.
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ActivityTab from './ActivityTab.svelte';

const { gql, goto, state } = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn(), state: { url: new URL('http://x/emotes/E1?tab=activity') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a) }));
vi.mock('$app/state', () => ({ page: state }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const ana = { account: { id: 'A1', displayName: 'Ana', avatar: null }, identities: [{ platform: 'TWITCH', login: 'ana', displayName: 'Ana' }], count: 3 };

beforeEach(() => {
  gql.mockReset();
  goto.mockReset();
  gql.mockImplementation(async (doc: unknown) => {
    const text = String(doc);
    if (text.includes('activityTopActors')) return { activityTopActors: [ana] };
    if (text.includes('emoteActivityPage')) return { emoteActivityPage: { entries: [], next: null } };
    return { accountCosmetics: [], account: null };
  });
});
afterEach(() => cleanup());

const feedCalls = () => gql.mock.calls.filter((c) => String(c[0]).includes('emoteActivityPage')).map((c) => c[1] as { actorId: string | null });

describe('emote Activity "By"', () => {
  it('asks for the lines of the person in ?by=', async () => {
    state.url = new URL('http://x/emotes/E1?tab=activity&by=A1');
    render(ActivityTab, { emoteId: 'E1', name: 'wave' });
    await vi.waitFor(() => expect(feedCalls().at(-1)?.actorId).toBe('A1'));
    expect(await screen.findByText('Nothing by them.')).toBeTruthy();
  });

  it('picking someone puts them in the address; removing takes them out', async () => {
    state.url = new URL('http://x/emotes/E1?tab=activity');
    const user = userEvent.setup();
    render(ActivityTab, { emoteId: 'E1', name: 'wave' });
    await vi.waitFor(() => expect(feedCalls().at(-1)?.actorId).toBeNull());
    await user.click(screen.getByRole('combobox', { name: 'By' }));
    await vi.waitFor(() => expect(screen.getAllByRole('option')).toHaveLength(1));
    await user.keyboard('{ArrowDown}{Enter}');
    const to = goto.mock.calls.at(-1)?.[0] as URL;
    expect(to.searchParams.get('by')).toBe('A1');
    expect(to.searchParams.get('tab')).toBe('activity');

    cleanup();
    state.url = new URL('http://x/emotes/E1?tab=activity&by=A1');
    render(ActivityTab, { emoteId: 'E1', name: 'wave', owner: { id: 'A1', displayName: 'Ana' } });
    await user.click(await screen.findByRole('button', { name: 'Remove by filter' }));
    expect((goto.mock.calls.at(-1)?.[0] as URL).searchParams.has('by')).toBe(false);
  });
});
