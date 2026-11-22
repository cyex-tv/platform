// The header's Upload is a button that opens the upload dialog where you are (owner, 2026/11/22:
// "/upload shouldnt even exist, only the upload button should work"): no link, no navigation.
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import AppHeader from './AppHeader.svelte';
import { session } from '$lib/tool/session.svelte';
import { closeUpload, upload } from '$lib/upload/upload.svelte';

const { goto } = vi.hoisted(() => ({ goto: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), beforeNavigate: vi.fn(), afterNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/search?q=cat'), data: {} }, navigating: { to: null } }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: () => new Promise(() => {}), onSignedOut: () => () => {} }));

beforeEach(() => {
  goto.mockReset();
  closeUpload();
  session.me = { id: 'ME', identities: [{ platform: 'TWITCH', displayName: 'me', login: 'me' }], permissions: [], roles: [] } as unknown as typeof session.me;
  session.known = true;
});

describe('the header’s Upload', () => {
  it('is a button, not a link to an upload page', () => {
    render(AppHeader);
    const up = screen.getByTestId('header-upload');
    expect(up.tagName).toBe('BUTTON');
    expect(up.getAttribute('href')).toBeNull();
    expect(document.querySelector('a[href="/me/upload"], a[href="/upload"]')).toBeNull();
  });

  it('opens the upload dialog on the page you are on', async () => {
    const user = userEvent.setup();
    render(AppHeader);
    expect(upload.open).toBe(false);
    await user.click(screen.getByTestId('header-upload'));
    expect(upload).toEqual({ open: true, for: null });
    expect(goto).not.toHaveBeenCalled();
  });
});
