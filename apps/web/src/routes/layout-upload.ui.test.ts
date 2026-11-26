// The root layout builds the upload dialog and the shortcut sheet on first use, not with every
// page: `?upload=` (and the /upload redirect that sends you there) still opens the dialog, the
// Upload button and `?` still open theirs, and neither is on the page before it is asked for.
import { cleanup, render, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { createRawSnippet } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Layout from './+layout.svelte';
import { session } from '$lib/tool/session.svelte';
import { closeUpload, upload } from '$lib/upload/upload.svelte';
import { shortcuts } from '$lib/shell/shortcuts.svelte';

const { goto, url } = vi.hoisted(() => ({ goto: vi.fn(), url: { current: new URL('http://localhost/search') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), beforeNavigate: vi.fn(), afterNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { get url() { return url.current; }, data: {} }, navigating: { to: null } }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: () => new Promise(() => {}), onSignedOut: () => () => {} }));
vi.mock('$lib/tool/session.svelte', async (orig) => {
  const m = await orig<typeof import('$lib/tool/session.svelte')>();
  return { ...m, loadMe: () => new Promise(() => {}) };
});

const children = createRawSnippet(() => ({ render: () => '<p>page</p>' }));
const uploadDialog = () => document.querySelector<HTMLDialogElement>('dialog.upload-dialog');
const sheet = () => document.querySelector<HTMLDialogElement>('dialog.keys');

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
  };
  goto.mockReset();
  closeUpload();
  shortcuts.open = false;
  url.current = new URL('http://localhost/search');
  session.me = { id: 'ME', identities: [{ platform: 'TWITCH', displayName: 'me', login: 'me' }], permissions: [], roles: [] } as unknown as typeof session.me;
  session.known = true;
});
afterEach(cleanup);

describe('root layout, lazy dialogs', () => {
  it('ships neither dialog until one is asked for', async () => {
    render(Layout, { children });
    await new Promise((r) => setTimeout(r, 30));
    expect(uploadDialog()).toBeNull();
    expect(sheet()).toBeNull();
  });

  it('?upload= opens the upload dialog and takes the parameter off the address', async () => {
    url.current = new URL('http://localhost/search?upload=1&q=cat');
    render(Layout, { children });
    await waitFor(() => expect(uploadDialog()?.hasAttribute('open')).toBe(true));
    expect(upload.open).toBe(true);
    expect(goto).toHaveBeenCalledWith('/search?q=cat', { replaceState: true, noScroll: true, keepFocus: true });
  });

  it('signed out, ?upload= goes to sign in and comes back', async () => {
    session.me = null;
    url.current = new URL('http://localhost/search?upload=1');
    render(Layout, { children });
    await waitFor(() => expect(goto).toHaveBeenCalledWith('/signin?back=%2Fsearch%3Fupload%3D1', { replaceState: true }));
    expect(uploadDialog()).toBeNull();
  });

  it('? opens the shortcut sheet, and again closes it', async () => {
    const user = userEvent.setup();
    render(Layout, { children });
    await user.keyboard('?');
    await waitFor(() => expect(sheet()?.hasAttribute('open')).toBe(true));
    await user.keyboard('?');
    await waitFor(() => expect(sheet()?.hasAttribute('open')).toBe(false));
  });
});
