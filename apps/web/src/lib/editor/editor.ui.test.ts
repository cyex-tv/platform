// The upload dialog's two modes (owner, 2026/11/26): Quick upload and the emote editor. The
// editor's decoding and encoding (media.ts, a worker in the browser) are stood in for here; the
// real ones run in dev/e2e/emote-editor.mjs. What's checked: switching keeps both sides, a video
// in Quick upload offers "Open in editor", an MP4 opens as its frames with a seamless loop, the
// upload rules gate "Continue to upload" and their fixes work, and the result lands in the form.
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { sniff } from './sniff';
import { delaysAtFps } from './timing';

const { encodedSize, imports } = vi.hoisted(() => ({ encodedSize: { n: 120_000 }, imports: { n: 0 } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), beforeNavigate: () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/') } }));
vi.mock('$lib/api', async (orig) => ({
  ...(await orig<typeof import('$lib/api')>()),
  gql: () => new Promise(() => {}),
  onSignedOut: () => () => {},
}));
// The browser's decoders: an MP4 is 30 frames of 96 × 64 at 30 fps; a GIF 4 frames of 1200 × 300.
vi.mock('./media', () => ({
  Cancelled: class extends Error {},
  canEncodeWebp: async () => true,
  importFile: async (file: File) => {
    imports.n++;
    // jsdom's File can't be read; the fixture's bytes are checked with the real sniffer below.
    const kind = file.name.endsWith('.mp4') ? 'mp4' : 'gif';
    const [w, h, n, delays] = kind === 'mp4' ? [96, 64, 30, delaysAtFps(30, 30, 1000)] : [1200, 300, 4, [10, 10, 10, 10]];
    const bitmaps = Array.from({ length: n }, () => ({ width: w, height: h, close() {} }));
    return { kind, width: w, height: h, scaledWidth: w, scaledHeight: h, frames: n, bitmaps, delays };
  },
  encodeDoc: async () => new Blob([new Uint8Array(encodedSize.n)], { type: 'image/webp' }),
}));

URL.createObjectURL = () => 'blob:x';
URL.revokeObjectURL = () => {};
globalThis.createImageBitmap = (async () => ({ width: 64, height: 64, close() {} })) as unknown as typeof createImageBitmap;
HTMLDialogElement.prototype.showModal = function () {
  this.setAttribute('open', '');
};
// jsdom has no 2D canvas: one that takes every call and measures text as 10 px a letter.
const ctx = new Proxy({} as Record<string | symbol, unknown>, {
  get: (t, k) => (k === 'measureText' ? (s: string) => ({ width: s.length * 10 }) : k in t ? t[k] : () => ({})),
  set: (t, k, v) => ((t[k] = v), true),
});
HTMLCanvasElement.prototype.getContext = (() => ctx) as unknown as typeof HTMLCanvasElement.prototype.getContext;

const mp4 = () => new File([readFileSync(resolve('dev/e2e/fixtures/loop.mp4'))], 'loop.mp4', { type: 'video/mp4' });
const gif = () => new File([new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])], 'wide.gif', { type: 'image/gif' });

async function dialog() {
  const { upload } = await import('$lib/upload/upload.svelte');
  upload.open = true;
  const { default: UploadDialog } = await import('$lib/upload/UploadDialog.svelte');
  render(UploadDialog);
  return document.querySelector('dialog')!;
}
const mode = (name: string) => screen.getByRole('radio', { name });
const drop = (el: Element, files: File[]) => fireEvent.drop(el, { dataTransfer: { files } });

beforeEach(() => {
  encodedSize.n = 120_000;
  imports.n = 0;
});

describe('the upload dialog’s modes', () => {
  it('switches to the editor and back, keeping the form', async () => {
    await dialog();
    expect(mode('Quick upload').getAttribute('aria-checked')).toBe('true');
    const name = screen.getByRole('textbox', { name: 'Name' }) as HTMLInputElement;
    await fireEvent.input(name, { target: { value: 'catJAM' } });
    await fireEvent.click(mode('Editor'));
    expect(await screen.findByText('Drop a picture or a video here')).toBeTruthy();
    expect(document.querySelector('dialog')!.classList.contains('wide')).toBe(true);
    await fireEvent.click(mode('Quick upload'));
    expect((screen.getByRole('textbox', { name: 'Name' }) as HTMLInputElement).value).toBe('catJAM');
    // The editor stays mounted (hidden), so its work is kept.
    expect(document.querySelector('.ed')).toBeTruthy();
  });

  it('knows the MP4 fixture and a GIF from their bytes', () => {
    expect(sniff(new Uint8Array(readFileSync(resolve('dev/e2e/fixtures/loop.mp4'))))).toBe('mp4');
    expect(sniff(new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0, 0, 0, 0, 0, 0]))).toBe('gif');
  });

  it('offers Open in editor for an MP4, which opens as 30 frames looping in exactly 1 s', async () => {
    const d = await dialog();
    await drop(d, [mp4()]);
    const open = await screen.findByRole('button', { name: 'Open in editor' });
    expect(open.classList.contains('primary')).toBe(true);
    await fireEvent.click(open);
    await waitFor(() => expect(document.querySelector('.tl .pos')?.textContent?.trim()).toBe('1 / 30'));
    expect(mode('Editor').getAttribute('aria-checked')).toBe('true');
    expect(document.querySelector('.tl .len')?.textContent?.trim()).toBe('1 s');
    const delays = [...document.querySelectorAll('.tl .item .d')].map((x) => Number(x.textContent));
    expect(delays.length).toBeGreaterThan(2); // only the frames in view are drawn
    expect(delays.every((x) => x === 33 || x === 34)).toBe(true);
  });

  it('gates Continue on the rules, fixes them in one click, and hands the result to the form', async () => {
    const d = await dialog();
    await fireEvent.click(mode('Editor'));
    await screen.findByText('Drop a picture or a video here');
    await drop(d, [gif()]);
    await waitFor(() => expect(document.querySelector('.tl .pos')?.textContent?.trim()).toBe('1 / 4'));
    const cont = () => screen.getByTestId('editor-continue') as HTMLButtonElement;
    const rule = (id: string) => document.querySelector(`.rule[data-rule="${id}"]`)!;
    // 1200 × 300 at 10 ms a frame: too big a side and too fast.
    expect(rule('side').classList.contains('fail')).toBe(true);
    // Only the five rules are listed: the pixel budget is checked but not shown.
    expect(document.querySelector('.rule[data-rule="pixels"]')).toBeNull();
    expect([...document.querySelectorAll('.rule .name')].map((n) => n.firstChild?.textContent)).toEqual(['Size', 'Shape', 'Frames', 'Frame rate', 'File size']);
    expect(rule('fps').classList.contains('fail')).toBe(true);
    expect(cont().disabled).toBe(true);
    await fireEvent.click(within(rule('side') as HTMLElement).getByRole('button', { name: 'Fit to 1.000 px' }));
    await fireEvent.click(within(rule('fps') as HTMLElement).getByRole('button', { name: 'Limit to 50 fps' }));
    expect(rule('side').classList.contains('pass')).toBe(true);
    expect(rule('side').textContent).toContain('1.000 × 250 px');
    expect(rule('fps').classList.contains('pass')).toBe(true);
    // The size is the encoded file's: pending until it is encoded, then it passes.
    await waitFor(() => expect(rule('bytes').classList.contains('pass')).toBe(true), { timeout: 3000 });
    expect(cont().disabled).toBe(false);
    // Undo brings the failure back (and the gate with it).
    await fireEvent.click(screen.getByRole('button', { name: /^Undo Limit to 50 fps/ }));
    expect(rule('fps').classList.contains('fail')).toBe(true);
    expect(cont().disabled).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: /^Redo Limit to 50 fps/ }));
    await waitFor(() => expect(cont().disabled).toBe(false), { timeout: 3000 });
    await fireEvent.click(cont());
    await waitFor(() => expect(mode('Quick upload').getAttribute('aria-checked')).toBe('true'));
    await waitFor(() => expect(document.querySelector('.meta')?.textContent).toMatch(/wide\.webp/));
  });

  it('fails the size rule on a file over 8 MB', async () => {
    encodedSize.n = 9 * 1024 * 1024;
    const d = await dialog();
    await fireEvent.click(mode('Editor'));
    await screen.findByText('Drop a picture or a video here');
    await drop(d, [mp4()]);
    await waitFor(() => expect(document.querySelector('.rule[data-rule="bytes"]')?.classList.contains('fail')).toBe(true), { timeout: 3000 });
    expect(document.querySelector('.rule[data-rule="bytes"]')?.textContent).toContain('Scale to 75%');
    expect((screen.getByTestId('editor-continue') as HTMLButtonElement).disabled).toBe(true);
  });

  it('asks before another file replaces the edits', async () => {
    const d = await dialog();
    await fireEvent.click(mode('Editor'));
    await screen.findByText('Drop a picture or a video here');
    await drop(d, [gif()]);
    await waitFor(() => expect(document.querySelector('.tl .pos')).toBeTruthy());
    await fireEvent.click(screen.getByRole('button', { name: 'Reverse' }));
    await drop(d, [mp4()]);
    expect(await screen.findByRole('alertdialog')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Keep editing' }));
    expect(document.querySelector('.tl .pos')?.textContent?.trim()).toBe('1 / 4');
    expect(imports.n).toBe(1);
  });
});
