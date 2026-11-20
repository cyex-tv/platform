import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SetChat from './SetChat.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: vi.fn(async () => ({ accountCosmetics: [] })), onSignedOut: () => () => {} }));

const m = (name: string, w = 32, zeroWidth = false) => ({
  name, zeroWidth, emote: { host: 'https://cdn.test/e', animated: false, files: [{ name: '1x.webp', width: w, height: 32, isStatic: false }] },
});
const members = [m('Pog'), m('Kappa'), m('Wide', 96), m('Sparkle', 32, true), m('Hmm'), m('Lul'), m('Hype')];

afterEach(cleanup);

describe('SetChat', () => {
  it('shows a Twitch chat, switchable to Kick, with the set’s emotes (overlay stacked)', async () => {
    const user = userEvent.setup();
    render(SetChat, { members, owner: { id: 'U1', displayName: 'Ana' }, name: 'Main' });
    expect(screen.getByRole('heading', { name: 'Chat preview' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Twitch' }).getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByLabelText('Twitch chat with Main’s emotes')).toBeTruthy();
    expect(document.querySelectorAll('img[alt="Pog"]').length).toBeGreaterThan(0);
    expect(document.querySelector('.stack img.over')).toBeTruthy();
    await user.click(screen.getByRole('button', { name: 'Kick' }));
    expect(screen.getByLabelText('Kick chat with Main’s emotes')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Kick' }).getAttribute('aria-pressed')).toBe('true');
  });

  it('draws nothing for a set of only zero-width emotes', () => {
    render(SetChat, { members: [m('Over', 32, true)], owner: null, name: 'x' });
    expect(screen.queryByRole('heading', { name: 'Chat preview' })).toBeNull();
  });
});
