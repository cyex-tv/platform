// Your profile → Account (moved from Settings, owner rule 2026/11/22): the name, connections,
// ONE session action (Sign out everywhere: every browser and app, this one too) and no list of
// sessions to revoke one by one (the API keeps mySessions/revokeSession; the site doesn't use them).
import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

const EXPIRES = new Date(2026, 10, 26, 14, 5).toISOString();
const gql = vi.hoisted(() =>
  vi.fn(async (doc: unknown): Promise<Record<string, unknown>> => (String(doc).includes('signOutEverywhere') ? { signOutEverywhere: 3 } : {})),
);
const makeToken = vi.hoisted(() => vi.fn(async () => ({ token: 'tok_new_one', expiresAt: new Date(2026, 10, 29, 9, 30).toISOString() })));
const goto = vi.hoisted(() => vi.fn(async () => {}));

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto, invalidateAll: vi.fn() }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql, onSignedOut: () => () => {} }));
vi.mock('$lib/account/console-token', async (orig) => ({ ...(await orig<typeof import('$lib/account/console-token')>()), makeToken }));

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
    // Armed, not yet sent (the API token section reads its expiry on its own).
    expect(gql.mock.calls.some((c) => /signOutEverywhere/.test(String((c as unknown[])[0])))).toBe(false);
    await user.click(btn);
    expect(gql.mock.calls.map((c) => String((c as unknown[])[0])).filter((q) => /signOutEverywhere/.test(q))).toHaveLength(1);
    expect(goto).toHaveBeenCalledWith('/', { invalidateAll: true });
  });

  it('shows the API token’s expiry, and a new token with Copy once made', async () => {
    session.me = me as unknown as typeof session.me;
    gql.mockImplementation(async (doc: unknown) => (String(doc).includes('myConsoleToken') ? { myConsoleToken: { id: 'S1', createdAt: EXPIRES, expiresAt: EXPIRES } } : {}));
    const user = userEvent.setup();
    render(AccountSection);
    expect(screen.getByRole('heading', { name: /api token/i })).toBeTruthy();
    await waitFor(() => expect(screen.getByText('Expires 2026/11/26, 14:05')).toBeTruthy());
    expect(screen.queryByRole('textbox', { name: /api token/i })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'New token' }));
    expect(makeToken).toHaveBeenCalledTimes(1);
    await waitFor(() => expect((screen.getByRole('textbox', { name: /api token/i }) as HTMLInputElement).value).toBe('tok_new_one'));
    expect(screen.getByText('Expires 2026/11/29, 09:30')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Copy' })).toBeTruthy();
  });

  it('says None when there is no live token', async () => {
    session.me = me as unknown as typeof session.me;
    gql.mockImplementation(async (doc: unknown) => (String(doc).includes('myConsoleToken') ? { myConsoleToken: null } : {}));
    render(AccountSection);
    await waitFor(() => expect(screen.getByText('None')).toBeTruthy());
    expect(screen.getByRole('button', { name: 'Make token' })).toBeTruthy();
  });
});
