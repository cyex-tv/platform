// The emote page's Activity tab has no filters (owner rule, 2026/11/22: "emote page activity
// doesn't need a search field"): no "By" field, the feed is everyone's lines, and an old ?by=
// in the address is ignored.
import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ActivityTab from './ActivityTab.svelte';

const { gql, state } = vi.hoisted(() => ({ gql: vi.fn(), state: { url: new URL('http://x/emotes/E1?tab=activity') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: state }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const line = { id: 'L1', kind: 'SET_EMOTE_ADDED', actor: { id: 'A1', displayName: 'Ana', avatar: null }, set: { id: 'S1', name: 'Main' }, emote: null,
  fromName: null, toName: null, badge: null, paint: null, editor: null, createdAt: '2026-11-20T10:00:00Z' };

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(async (doc: unknown) => {
    const text = String(doc);
    if (text.includes('emoteActivityPage')) return { emoteActivityPage: { entries: [line], next: null } };
    return { accountCosmetics: [], account: null };
  });
});
afterEach(() => cleanup());

const feedCalls = () => gql.mock.calls.filter((c) => String(c[0]).includes('emoteActivityPage')).map((c) => c[1] as { actorId: string | null });

describe('emote Activity', () => {
  it('shows the lines with no "By" field or other filter', async () => {
    render(ActivityTab, { emoteId: 'E1', name: 'wave', owner: { id: 'A1', displayName: 'Ana' } });
    await vi.waitFor(() => expect(feedCalls().length).toBe(1));
    await vi.waitFor(() => expect(screen.getByText(/Main/)).toBeTruthy());
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByText('By')).toBeNull();
    expect(screen.queryByRole('group', { name: 'Filter activity' })).toBeNull();
    expect(gql.mock.calls.some((c) => String(c[0]).includes('activityTopActors'))).toBe(false);
  });

  it('ignores an old ?by= in the address', async () => {
    state.url = new URL('http://x/emotes/E1?tab=activity&by=A1');
    render(ActivityTab, { emoteId: 'E1', name: 'wave' });
    await vi.waitFor(() => expect(feedCalls().length).toBe(1));
    expect(feedCalls()[0].actorId).toBeNull();
  });
});
