// The profile's About panel (the Inspector profile, redesign 2026/11/21): public facts, the same
// for every viewer. Connections and Editors start collapsed and open on purpose; editors show ten
// at a time; the numbers are the public ones.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import ProfileAbout from './ProfileAbout.svelte';
import type { PublicProfile } from './data.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: vi.fn(() => new Promise(() => {})), onSignedOut: () => () => {} }));

const editors = Array.from({ length: 23 }, (_, i) => ({ id: `ED${i}`, displayName: `editor_${i}`, avatar: null }));
const profile = {
  account: { id: 'A1', displayName: 'quietoak', avatar: null },
  identities: [{ platform: 'TWITCH', login: 'quietoak', displayName: 'quietoak' }],
  roles: [],
  activeSets: [
    { platform: 'TWITCH', login: 'quietoak', setId: 'S1', setName: 'Main' },
    { platform: 'KICK', login: 'oak_kick', setId: 'S2', setName: 'Kick set' },
  ],
  editors,
  createdAt: '2026-11-08T10:00:00.000Z',
} as unknown as PublicProfile;

describe('profile About panel', () => {
  it('starts with connections and editors collapsed, with their counts', () => {
    render(ProfileAbout, { accountId: 'A1', profile, uploads: 1234 });
    const conn = screen.getByRole('button', { name: /Connections/ });
    const eds = screen.getByRole('button', { name: /Editors/ });
    expect(conn.getAttribute('aria-expanded')).toBe('false');
    expect(eds.getAttribute('aria-expanded')).toBe('false');
    expect(conn.textContent).toMatch(/2/); // the Twitch login, and the Kick channel showing a set
    expect(eds.textContent).toMatch(/23/);
    expect(screen.queryByRole('list', { name: 'Editors' })).toBeNull();
  });

  it('opens connections with the set each channel shows', async () => {
    const user = userEvent.setup();
    render(ProfileAbout, { accountId: 'A1', profile, uploads: 0 });
    await user.click(screen.getByRole('button', { name: /Connections/ }));
    const list = screen.getByRole('list', { name: 'Connections' });
    expect(list.querySelectorAll('li')).toHaveLength(2);
    expect(list.textContent).toMatch(/Shows\s*Main/);
    expect(list.textContent).toMatch(/oak_kick/);
  });

  it('shows editors ten at a time', async () => {
    const user = userEvent.setup();
    render(ProfileAbout, { accountId: 'A1', profile, uploads: 0 });
    await user.click(screen.getByRole('button', { name: /Editors/ }));
    const rows = () => screen.getByRole('list', { name: 'Editors' }).querySelectorAll('li');
    expect(rows()).toHaveLength(10);
    expect(screen.getByText('Showing 10 of 23')).toBeTruthy();
    await user.click(screen.getByRole('button', { name: 'Show 10 more' }));
    expect(rows()).toHaveLength(20);
    await user.click(screen.getByRole('button', { name: 'Show 3 more' }));
    expect(rows()).toHaveLength(23);
  });

  it('lists the public numbers', () => {
    render(ProfileAbout, { accountId: 'A1', profile, uploads: 1234 });
    const dl = document.querySelector('dl.kv')!;
    const pairs = [...dl.querySelectorAll('dt')].map((dt) => `${dt.textContent}=${dt.nextElementSibling?.textContent}`);
    expect(pairs).toEqual(['Public uploads=1.234', 'Channels showing a set=2', 'Connections=2', 'Editors=23', 'Member since=2026/11/08']);
  });

  it('says so when nobody else edits', () => {
    render(ProfileAbout, { accountId: 'A1', profile: { ...profile, editors: [] } as PublicProfile, uploads: 0 });
    expect(screen.getByText('Nobody else edits this channel.')).toBeTruthy();
  });
});
