import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import ProfileHeader from './ProfileHeader.svelte';
import { NOBODY } from './rows';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: vi.fn(async () => ({ accountCosmetics: [] })) }));

const editors = Array.from({ length: 50 }, (_, i) => ({ id: `E${i}`, displayName: `editor_${String(i).padStart(2, '0')}`, avatar: null }));
const profile = {
  identities: [{ platform: 'TWITCH', login: 'lumen_fox' }, { platform: 'KICK', login: 'lumenfox_kick' }],
  activeSets: [{ platform: 'TWITCH', login: 'lumen_fox', setId: 'S1', setName: 'Main' }],
  editors,
  roles: [],
  createdAt: '2026-11-10T09:00:00Z',
};
const band = () => render(ProfileHeader, { props: { account: { id: 'ME', displayName: 'lumen_fox' }, relation: NOBODY, profile: profile as never } });

describe('profile band disclosures', () => {
  it('editors open on purpose, ten at a time with a count, and Escape closes back to the chip', async () => {
    const user = userEvent.setup();
    band();
    const chip = screen.getByRole('button', { name: 'Editors, 50' });
    expect(chip.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByText('editor_00')).toBeNull();
    await user.click(chip);
    const panel = await screen.findByRole('dialog', { name: 'Editors' });
    expect(chip.getAttribute('aria-expanded')).toBe('true');
    expect(within(panel).getAllByRole('listitem')).toHaveLength(10);
    expect(within(panel).getByText('Showing 10 of 50')).toBeTruthy();
    await user.click(within(panel).getByRole('button', { name: 'Show 10 more' }));
    expect(within(panel).getAllByRole('listitem')).toHaveLength(20);
    // Focus is inside the panel; Escape closes it and gives focus back to the chip.
    expect(panel.contains(document.activeElement)).toBe(true);
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(chip);
  });

  it('connections list each login with the set its channel shows', async () => {
    const user = userEvent.setup();
    band();
    await user.click(screen.getByRole('button', { name: 'Connections, 2' }));
    const panel = await screen.findByRole('dialog', { name: 'Connections' });
    const rows = within(panel).getAllByRole('listitem');
    expect(rows).toHaveLength(2);
    expect(rows[0].textContent).toContain('lumen_fox');
    expect(within(rows[0]).getByRole('link', { name: 'Main' }).getAttribute('href')).toBe('/sets/S1');
    expect(within(rows[1]).getByRole('link', { name: /lumenfox_kick/ }).getAttribute('target')).toBe('_blank');
  });
});
