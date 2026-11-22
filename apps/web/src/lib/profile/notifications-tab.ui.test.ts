// Your profile → Notifications (owner rule 2026/11/22: it was /me/notifications): the filter is in
// the address (?f=), the panel counts what is kept and unread and shows the picked one (?n=).
import { cleanup, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Section from './sections/NotificationsSection.svelte';
import Panel from './NotificationsPanel.svelte';

const { gql, goto, url } = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn(), url: { current: new URL('http://x/users/A1?s=notifications&f=editors') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { get url() { return url.current; } } }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));

const note = (id: string, kind: string, read = false) => ({ id, kind, payload: JSON.stringify({ emote_name: 'Pog', owner_name: 'ana', editor_name: 'nora' }), createdAt: '2026-11-20T10:00:00Z', read });
const NOTES = [note('1', 'emote.approved'), note('2', 'editor.added'), note('3', 'editor.removed', true), note('4', 'cosmetic.granted', true)];

afterEach(() => {
  cleanup();
  goto.mockReset();
  url.current = new URL('http://x/users/A1?s=notifications&f=editors');
});

describe('the Notifications tab', () => {
  it('opens on the filter the address names, and a chip changes the address (the tab stays)', async () => {
    gql.mockImplementation(async (doc: string) => (doc.includes('myNotifications') ? { myNotifications: NOTES } : { unreadNotificationCount: 2 }));
    const user = userEvent.setup();
    render(Section);
    const list = await screen.findByRole('region', { name: 'Notifications' });
    await waitFor(() => expect(within(list).getAllByRole('listitem').length).toBe(2));
    expect(screen.getByRole('radio', { name: /Editors/ }).getAttribute('aria-checked')).toBe('true');
    await user.click(screen.getByRole('radio', { name: /Cosmetics/ }));
    expect(goto).toHaveBeenCalledWith('/users/A1?s=notifications&f=cosmetics', expect.objectContaining({ replaceState: true }));
    await user.click(screen.getByRole('radio', { name: /^All/ }));
    expect(goto).toHaveBeenLastCalledWith('/users/A1?s=notifications', expect.anything());
  });

  it('counts what is kept in the panel and links to the settings', async () => {
    gql.mockImplementation(async (doc: string) => (doc.includes('myNotifications') ? { myNotifications: NOTES } : { unreadNotificationCount: 2 }));
    render(Section);
    render(Panel);
    await waitFor(() => expect(screen.getByText('Kept').nextElementSibling?.textContent?.trim()).toBe('4'));
    expect(screen.getByRole('link', { name: /Notification settings/ }).getAttribute('href')).toBe('/me/settings/notifications');
  });

  it('shows the picked one (?n=) in the panel, and Close drops the pick', async () => {
    url.current = new URL('http://x/users/A1?s=notifications&n=2');
    gql.mockImplementation(async (doc: string) => (doc.includes('myNotifications') ? { myNotifications: NOTES } : { unreadNotificationCount: 2 }));
    const user = userEvent.setup();
    render(Section);
    const { container } = render(Panel);
    await waitFor(() => expect(container.querySelector('.pick')).not.toBeNull());
    await user.click(within(container).getByRole('button', { name: 'Close' }));
    expect(goto).toHaveBeenLastCalledWith('/users/A1?s=notifications', expect.anything());
  });
});
