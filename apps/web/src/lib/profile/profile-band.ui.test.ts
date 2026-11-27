import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { readFileSync } from 'node:fs';
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

// The profile page's head (Inspector layout): the person's roles as flags beside the name, the
// first three shown and the rest behind a count chip; no Copy link (owner, feedback round I:
// "remove the copy link button from a users profile and replace it with their roles as flags").
describe('profile head', () => {
  const roles = ['Moderator', 'Artist', 'Partner', 'Helper', 'Supporter'].map((name, i) => ({ id: `R${i}`, name }));
  const head = () =>
    render(ProfileHeader, {
      props: { account: { id: 'OTHER', displayName: 'quietoak' }, relation: NOBODY, profile: { ...profile, roles } as never, facts: false, flat: true },
    });

  it('shows the roles as flags next to the name, the rest behind a count', () => {
    head();
    const flags = screen.getByRole('list', { name: 'Roles' });
    expect([...flags.querySelectorAll(':scope > li.flag')].map((li) => li.textContent?.trim())).toEqual(['Moderator', 'Artist', 'Partner']);
    expect(within(flags).getByRole('button', { name: 'All roles, 2' })).toBeTruthy();
  });

  it('has no Copy link, on the head or anywhere on the profile page', () => {
    head();
    expect(screen.queryByRole('button', { name: /Copy link/ })).toBeNull();
    const page = readFileSync(`${process.cwd()}/src/routes/users/[id]/+page.svelte`, 'utf8');
    expect(page).not.toMatch(/CopyLink|Copy link/);
    // The flat head still gets its roles (the facts chips are what moved to the About panel).
    expect(page).toMatch(/<ProfileHeader [^>]*facts=\{false\} flat \/>/);
  });
});


// The ring round the avatar is the colour of the account's highest role that has one (the API
// resolves it: `roleColor`), never the paint or badge; without one it keeps the neutral ring.
describe('profile ring', () => {
  const ring = (container: HTMLElement) => container.querySelector<HTMLElement>('.ring')!;
  const draw = (extra: Record<string, unknown>) =>
    render(ProfileHeader, { props: { account: { id: 'OTHER', displayName: 'quietoak' }, relation: NOBODY, profile: { ...profile, ...extra } as never, flat: true } });

  it('uses the role colour', () => {
    const { container } = draw({ roleColor: '#3b82f6' });
    expect(ring(container).style.background).toBe('rgb(59, 130, 246)');
  });

  it('stays neutral without a coloured role, and ignores a malformed colour', () => {
    expect(ring(draw({ roleColor: null }).container).style.background).toBe('');
    expect(ring(draw({ roleColor: 'red' }).container).style.background).toBe('');
  });

  it('is not drawn from the paint', async () => {
    const { seedLook } = await import('$lib/cosmetics/store.svelte');
    seedLook('PAINTED', { badge: null, paint: { kind: 'SOLID', stops: [{ at: 0, color: '#ff00ff' }], repeat: false, angle: 0, shadows: [] } } as never);
    const { container } = render(ProfileHeader, {
      props: { account: { id: 'PAINTED', displayName: 'painter' }, relation: NOBODY, profile: { ...profile, roleColor: null } as never, flat: true },
    });
    expect(ring(container).style.background).toBe('');
  });

  it('shows a role colour as a dot on its flag, readable text unchanged', () => {
    const { container } = draw({ roles: [{ id: 'R1', name: 'Moderator', color: '#3b82f6' }, { id: 'R2', name: 'Artist', color: null }] });
    const flags = container.querySelectorAll('li.flag');
    expect((flags[0].querySelector('.cdot') as HTMLElement).style.background).toBe('rgb(59, 130, 246)');
    expect(flags[1].querySelector('.cdot')).toBeNull();
    expect(flags[0].textContent?.trim()).toBe('Moderator');
  });
});
