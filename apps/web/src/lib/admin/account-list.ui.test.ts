import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import AccountList from './AccountList.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: async () => ({ accountCosmetics: [] }), onSignedOut: () => () => {} }));

const items = [
  { account: { id: 'A1', displayName: 'Oak', avatar: null }, identities: [{ platform: 'TWITCH', login: 'oak' }, { platform: 'KICK', login: 'oak_k' }] },
  { account: { id: 'A2', displayName: 'Nora', avatar: null }, identities: [{ platform: 'KICK', login: 'nora' }] },
];

describe('AccountList', () => {
  it('lists each account as a link to its admin page with its logins', () => {
    render(AccountList, { items });
    const links = screen.getAllByRole('link');
    expect(links).toHaveLength(2);
    expect(links[0].getAttribute('href')).toBe('/admin/users/A1');
    expect(links[0].textContent).toContain('oak on Twitch');
    expect(links[0].textContent).toContain('oak_k on Kick');
  });

  it('keeps one Tab stop over the rows', () => {
    render(AccountList, { items });
    const [a, b] = screen.getAllByRole('link');
    expect(a.tabIndex).toBe(0);
    expect(b.tabIndex).toBe(-1);
  });

  it('picks instead of opening when given onpick', async () => {
    const onpick = vi.fn();
    render(AccountList, { items, onpick });
    expect(screen.queryAllByRole('link')).toHaveLength(0);
    await fireEvent.click(screen.getByRole('button', { name: /Nora/ }));
    expect(onpick).toHaveBeenCalledWith(items[1].account);
  });
});
