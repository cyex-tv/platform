// The upload dialog's form (owner, 2026/11/22): one image, a name and tags, the two flags a
// creator sets (Overlaying and Private; no Unlisted, no NSFW), the rules box, Discard and Upload
// on the left; a large drop zone with Browse files and our limits on the right.
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import UploadDropHarness from '../test/UploadDropHarness.svelte';
import { session } from '$lib/tool/session.svelte';

const { guard, uploadEmote, goto } = vi.hoisted(() => ({
  guard: { fn: null as null | ((n: unknown) => void) },
  uploadEmote: vi.fn(),
  goto: vi.fn(),
}));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), beforeNavigate: (f: (n: unknown) => void) => (guard.fn = f) }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/search') } }));
vi.mock('$lib/api', async (orig) => ({
  ...(await orig<typeof import('$lib/api')>()),
  gql: () => new Promise(() => {}),
  uploadEmote: (...a: unknown[]) => uploadEmote(...a),
  onSignedOut: () => () => {},
}));
URL.createObjectURL = () => 'blob:x';
URL.revokeObjectURL = () => {};
// jsdom has no image decoding: every picked image reads as 64 × 64.
globalThis.createImageBitmap = (async () => ({ width: 64, height: 64, close() {} })) as unknown as typeof createImageBitmap;
HTMLDialogElement.prototype.showModal = function () {
  this.setAttribute('open', '');
};

const png = (name = 'cat.png') => new File([new Uint8Array(40)], name, { type: 'image/png', lastModified: 1 });
const drop = (el: Element, files: File[]) => fireEvent.drop(el, { dataTransfer: { files } });
const name = () => screen.getByRole('textbox', { name: 'Name' }) as HTMLInputElement;

beforeEach(() => {
  uploadEmote.mockReset();
  goto.mockReset();
  session.me = null;
});

describe('upload form fields', () => {
  it('has name, tags, Overlaying and Private, the rules box, Discard, Upload, Browse files and the limits', () => {
    render(UploadDropHarness);
    expect(name()).toBeTruthy();
    expect(screen.getByLabelText('Tags')).toBeTruthy();
    expect(screen.getByRole('checkbox', { name: /Overlaying/ })).toBeTruthy();
    expect(screen.getByRole('checkbox', { name: /Private/ })).toBeTruthy();
    const rules = screen.getByRole('link', { name: /rules and guidelines/ });
    expect([rules.getAttribute('href'), rules.getAttribute('target')]).toEqual(['/guidelines', '_blank']);
    expect(screen.getByRole('checkbox', { name: /I accept the rules and guidelines/ })).toBeTruthy();
    for (const b of ['Discard', 'Upload', 'Browse files']) expect(screen.getByRole('button', { name: b })).toBeTruthy();
    const limits = screen.getByRole('list', { name: 'Limits' }).textContent ?? '';
    expect(limits).toMatch(/8 MB/);
    expect(limits).toMatch(/1000 × 1000 px/);
    expect(limits).toMatch(/128 px tall/);
  });

  it('offers no NSFW and no Unlisted', () => {
    render(UploadDropHarness);
    expect(document.body.textContent).not.toMatch(/NSFW|Unlisted|Visibility/);
    expect(screen.queryByRole('radiogroup')).toBeNull();
  });
});

describe('upload form drops', () => {
  it('a dropped image goes in once, with its name from the file', async () => {
    render(UploadDropHarness);
    await drop(document.querySelector('.drop')!, [png('fern_wave.png')]);
    expect(name().value).toBe('fern_wave');
    expect(screen.getByRole('button', { name: 'Replace image' })).toBeTruthy();
  });

  it('an image dropped elsewhere on the dialog goes in too; of several, the first', async () => {
    render(UploadDropHarness);
    await drop(screen.getByTestId('outer'), [png('one.png'), png('two.png')]);
    expect(name().value).toBe('one');
    expect(document.body.textContent).toMatch(/One image at a time/);
  });

  it('a typed name stays when another image replaces the first', async () => {
    render(UploadDropHarness);
    await drop(document.querySelector('.drop')!, [png('a.png')]);
    await fireEvent.input(name(), { target: { value: 'myName' } });
    await drop(document.querySelector('.drop')!, [png('b.png')]);
    expect(name().value).toBe('myName');
  });

  it('Discard empties the form', async () => {
    render(UploadDropHarness);
    await drop(document.querySelector('.drop')!, [png()]);
    await fireEvent.click(screen.getByRole('button', { name: 'Discard' }));
    expect(name().value).toBe('');
    expect(screen.getByRole('button', { name: 'Browse files' })).toBeTruthy();
  });
});

describe('upload form sends', () => {
  it('sends Overlaying and Private as the flags, never NSFW or unlisted', async () => {
    session.me = { id: 'ME', identities: [{ displayName: 'me' }], permissions: [], roles: [] } as unknown as typeof session.me;
    uploadEmote.mockResolvedValue({ id: 'E9', name: 'cat', state: 'PENDING' });
    const user = userEvent.setup();
    render(UploadDropHarness);
    await drop(document.querySelector('.drop')!, [png()]);
    await user.click(screen.getByRole('checkbox', { name: /Overlaying/ }));
    await user.click(screen.getByRole('checkbox', { name: /Private/ }));
    await user.click(screen.getByRole('checkbox', { name: /I accept/ }));
    await user.click(screen.getByRole('button', { name: 'Upload' }));
    await waitFor(() => expect(uploadEmote).toHaveBeenCalled());
    const [, sentName, tags, flags] = uploadEmote.mock.calls[0];
    expect([sentName, tags, flags]).toEqual(['cat', [], { zeroWidth: true, visibility: 'PRIVATE' }]);
    await waitFor(() => expect(goto).toHaveBeenCalledWith('/me/emotes/E9'));
  });

  it('asks for the rules box before uploading', async () => {
    const user = userEvent.setup();
    render(UploadDropHarness);
    await drop(document.querySelector('.drop')!, [png()]);
    await user.click(screen.getByRole('button', { name: 'Upload' }));
    expect(screen.getByText('Tick the box to upload.')).toBeTruthy();
    expect(uploadEmote).not.toHaveBeenCalled();
  });
});

describe('upload form leave guard', () => {
  const unload = () => {
    const e = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(e);
    return e.defaultPrevented;
  };

  it('asks before the tab closes while an image waits, not when empty; moving between pages keeps it', async () => {
    render(UploadDropHarness);
    expect(unload()).toBe(false);
    await drop(document.querySelector('.drop')!, [png()]);
    expect(unload()).toBe(true);
    const cancel = vi.fn();
    guard.fn!({ cancel, willUnload: false, to: { url: new URL('http://localhost/me') } });
    expect(cancel).not.toHaveBeenCalled();
  });
});
