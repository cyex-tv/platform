// Your profile → Account (moved from Settings, owner rule 2026/11/22): the name, connections,
// ONE session action (Sign out everywhere: every browser and app, this one too) and no list of
// sessions to revoke one by one (the API keeps mySessions/revokeSession; the site doesn't use them).
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

const gql = vi.hoisted(() => vi.fn(async (doc: unknown) => (String(doc).includes('signOutEverywhere') ? { signOutEverywhere: 3 } : {})));
const goto = vi.hoisted(() => vi.fn(async () => {}));

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto, invalidateAll: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql, onSignedOut: () => () => {} }));

import { session } from '$lib/tool/session.svelte';
import AccountSection from './sections/AccountSection.svelte';

const me = {
  id: 'A1',
  createdAt: '2026-11-10T12:00:00.000Z',
  permissions: [],
  roles: [],
  identities: [{ platform: 'TWITCH', login: 'lumen_fox', displayName: 'lumen_fox', avatar: null, platformUserId: '141981764' }],
};

afterEach(() => {
  vi.clearAllMocks();
  session.me = null;
});

describe('the Account tab', () => {
  it('has one session action, Sign out everywhere, and no sessions list', () => {
    session.me = me as unknown as typeof session.me;
    render(AccountSection);
    expect(screen.getAllByRole('button', { name: /sign out/i }).map((b) => b.textContent?.trim())).toEqual([expect.stringMatching(/^Sign out everywhere/)]);
    expect(screen.queryByRole('heading', { name: /sessions/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /revoke/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /everywhere else/i })).toBeNull();
    // Nothing asks the API for the sessions list.
    expect(gql.mock.calls.some((c) => /mySessions/.test(String((c as unknown[])[0])))).toBe(false);
    // The rest of the old Settings → Account is here.
    expect(screen.getByRole('heading', { name: /display name/i })).toBeTruthy();
    expect(screen.getByRole('heading', { name: /connections/i })).toBeTruthy();
  });

  it('signs out every session on the second click, then leaves', async () => {
    session.me = me as unknown as typeof session.me;
    const user = userEvent.setup();
    render(AccountSection);
    const btn = screen.getByRole('button', { name: /sign out everywhere/i });
    await user.click(btn);
    expect(gql).not.toHaveBeenCalled(); // armed, not yet sent
    await user.click(btn);
    expect(gql.mock.calls.map((c) => String((c as unknown[])[0])).filter((q) => /signOutEverywhere/.test(q))).toHaveLength(1);
    expect(goto).toHaveBeenCalledWith('/', { invalidateAll: true });
  });
});
