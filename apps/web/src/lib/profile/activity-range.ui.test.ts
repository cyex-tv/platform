// The profile's Activity feed (owner rule, 2026/11/22): visitors read the latest 100 lines, all
// time, with no time control ("Latest 100", paging stops there); the owner and staff page through
// everything and pick a date range (?from= ?to=) instead of Day or Month.
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ActivityFeed from './sections/ActivityFeed.svelte';
import { lastDays, rangeOf } from './range';

const { gql, goto, state } = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn(), state: { url: new URL('http://x/users/A1?s=activity') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), invalidateAll: vi.fn() }));
vi.mock('$app/state', () => ({ page: state }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));

let n = 0;
const line = () => ({ id: `L${String(1000 - ++n).padStart(4, '0')}`, kind: 'SET_EMOTE_ADDED', actor: { id: 'A1', displayName: 'Ana', avatar: null },
  set: { id: 'S1', name: 'Main' }, emote: null, fromName: null, toName: null, badge: null, paint: null, editor: null, createdAt: '2026-11-20T10:00:00Z' });

beforeEach(() => {
  n = 0;
  gql.mockReset();
  goto.mockReset();
  // An endless feed: every page is full and says there is more.
  gql.mockImplementation(async (doc: unknown, vars: { limit?: number }) => {
    const text = String(doc);
    if (text.includes('accountActivityPage')) {
      const entries = Array.from({ length: vars.limit ?? 30 }, line);
      return { accountActivityPage: { entries, next: entries.at(-1)!.id } };
    }
    if (text.includes('activityTopActors')) return { activityTopActors: [] };
    return {};
  });
});
afterEach(() => cleanup());

const feedCalls = () => gql.mock.calls.filter((c) => String(c[0]).includes('accountActivityPage')).map((c) => c[1] as { filter: { since: string | null; until: string | null } });

describe('profile Activity for a visitor', () => {
  it('shows the latest 100, all time, with no date control, and stops paging there', async () => {
    state.url = new URL('http://x/users/A1?s=activity&from=2026-11-01&d=2026-11-19');
    const user = userEvent.setup();
    render(ActivityFeed, { accountId: 'A1', name: 'Ana' });
    expect(await screen.findByTestId('activity-cap')).toBeTruthy();
    expect(screen.getByTestId('activity-cap').textContent).toBe('Latest 100');
    expect(screen.queryByTestId('activity-range')).toBeNull();
    expect(screen.queryByRole('radiogroup', { name: 'Timeframe' })).toBeNull();
    // A range in the address is not theirs: all time is asked for.
    await vi.waitFor(() => expect(feedCalls().length).toBe(1));
    expect(feedCalls()[0].filter).toMatchObject({ since: null, until: null });
    for (let i = 0; i < 5; i++) {
      const more = screen.queryByRole('button', { name: /show more|load more/i });
      if (!more) break;
      await user.click(more);
    }
    await vi.waitFor(() => expect(screen.getByText(/latest 100\. Older changes/)).toBeTruthy());
    expect(screen.queryByRole('button', { name: /show more|load more/i })).toBeNull();
    expect(document.querySelectorAll('ol.feed > li.item').length).toBe(100);
    expect(feedCalls().length).toBe(4); // 30 + 30 + 30 + 10 shown of the fourth
  });
});

describe('profile Activity for the owner', () => {
  it('pages on past 100 and has a date range instead of Day / Month', async () => {
    state.url = new URL('http://x/users/A1?s=activity');
    const user = userEvent.setup();
    render(ActivityFeed, { accountId: 'A1', name: 'Ana', full: true });
    const trigger = await screen.findByTestId('activity-range');
    expect(trigger.textContent).toContain('All time');
    expect(screen.queryByTestId('activity-cap')).toBeNull();
    expect(screen.queryByRole('radio', { name: 'Day' })).toBeNull();
    expect(screen.queryByRole('radio', { name: 'Month' })).toBeNull();
    for (let i = 0; i < 4; i++) await user.click(await screen.findByRole('button', { name: /show more|load more/i }));
    expect(screen.getByRole('button', { name: /show more|load more/i })).toBeTruthy();
    expect(feedCalls().length).toBe(5);

    // A shortcut writes the range into the address.
    await user.click(trigger);
    await user.click(await screen.findByRole('button', { name: 'Last 7 days' }));
    const to = goto.mock.calls.at(-1)?.[0] as URL;
    const want = lastDays(7);
    expect(to.searchParams.get('from')).toBe(want.from);
    expect(to.searchParams.get('to')).toBe(want.to);
  });

  it('asks the API for the range in the address', async () => {
    state.url = new URL('http://x/users/A1?s=activity&from=2026-11-01&to=2026-11-10');
    render(ActivityFeed, { accountId: 'A1', name: 'Ana', full: true });
    await vi.waitFor(() => expect(feedCalls().length).toBe(1));
    expect(feedCalls()[0].filter).toMatchObject(rangeOf({ from: '2026-11-01', to: '2026-11-10' }));
    expect((await screen.findByTestId('activity-range')).textContent).toContain('2026/11/01 – 2026/11/10');
  });

  it('refuses a backwards range in words and keeps the address', async () => {
    state.url = new URL('http://x/users/A1?s=activity');
    const user = userEvent.setup();
    render(ActivityFeed, { accountId: 'A1', name: 'Ana', full: true });
    await user.click(await screen.findByTestId('activity-range'));
    await user.type(await screen.findByLabelText('From'), '2026/11/10');
    await user.type(screen.getByLabelText('To'), '2026/11/01');
    await user.click(screen.getByRole('button', { name: 'Apply' }));
    expect((await screen.findByRole('alert')).textContent).toMatch(/after the end/);
    expect(goto).not.toHaveBeenCalled();
  });
});
