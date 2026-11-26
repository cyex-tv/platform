// The announcement bar comes from the root layout's server load, so it is in the first paint and
// nothing moves: it asks the API itself only on a prerendered page (no answer there), and closing
// it is remembered in a cookie the server reads next time.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const gql = vi.fn();
const data: { announcement?: unknown; announceClosed?: string[] } = {};
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), gqlPublic: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { get data() { return data; }, url: new URL('http://x/') } }));

const live = {
  id: 'ANN1', kind: 'CAMPAIGN', text: 'Winter paints are in the shop.', linkUrl: '/shop', linkLabel: 'See them', badgeId: null,
  startsAt: null, endsAt: null, endedAt: null, createdAt: '2026-11-20T10:00:00.000Z', updatedAt: '2026-11-20T10:00:00.000Z', live: true, emote: null,
};

// A fresh page load: the bar's store forgets what the browser asked and read.
async function fresh() {
  const { banner } = await import('./announcement.svelte');
  banner.current = undefined;
  banner.closed = undefined;
  return (await import('./AnnouncementBar.svelte')).default;
}

beforeEach(() => {
  gql.mockReset();
  gql.mockResolvedValue({ activeAnnouncement: live });
  localStorage.clear();
  document.cookie = 'cyexAnnounceClosed=; Max-Age=0; Path=/';
  delete data.announcement;
  data.announceClosed = [];
});

describe('the announcement bar', () => {
  it('shows the server’s announcement at once, without asking the API', async () => {
    data.announcement = live;
    render(await fresh());
    expect(screen.getByRole('complementary', { name: 'Announcement' }).textContent).toContain('Winter paints are in the shop.');
    expect(gql).not.toHaveBeenCalled();
  });

  it('shows nothing when the server says none, or this device closed it', async () => {
    data.announcement = null;
    const { unmount } = render(await fresh());
    expect(screen.queryByRole('complementary', { name: 'Announcement' })).toBeNull();
    unmount();
    data.announcement = live;
    data.announceClosed = ['ANN1'];
    render(await fresh());
    expect(screen.queryByRole('complementary', { name: 'Announcement' })).toBeNull();
    expect(gql).not.toHaveBeenCalled();
  });

  it('asks from the browser on a prerendered page, and remembers it for the next first paint', async () => {
    render(await fresh());
    expect(await screen.findByRole('complementary', { name: 'Announcement' })).toBeTruthy();
    expect(gql).toHaveBeenCalledOnce();
    expect(localStorage.getItem('cyexAnnounce')).toBe('ANN1');
  });

  it('remembers a close in the cookie the server reads', async () => {
    data.announcement = live;
    const user = userEvent.setup();
    render(await fresh());
    await user.click(screen.getByRole('button', { name: 'Close the announcement' }));
    expect(document.cookie).toContain('cyexAnnounceClosed=ANN1');
    expect(JSON.parse(localStorage.getItem('cyex.announcement.closed') ?? '[]')).toEqual(['ANN1']);
  });

  it('carries a close made before the cookie existed into it', async () => {
    localStorage.setItem('cyex.announcement.closed', JSON.stringify(['OLD1']));
    data.announcement = live;
    render(await fresh());
    expect(document.cookie).toContain('cyexAnnounceClosed=OLD1');
  });
});
