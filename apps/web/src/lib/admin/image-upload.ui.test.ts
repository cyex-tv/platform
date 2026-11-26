import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ImageUpload from './ImageUpload.svelte';

const { upload, gql } = vi.hoisted(() => ({ upload: vi.fn(), gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), uploadCosmeticImage: (...a: unknown[]) => upload(...a) }));

// Decoding takes a moment; each file's size comes back when the test lets it.
const sizes = new Map<File, (v: { width: number; height: number; close(): void }) => void>();
beforeEach(() => {
  upload.mockReset().mockImplementation(() => new Promise(() => {}));
  gql.mockReset();
  sizes.clear();
  vi.stubGlobal('createImageBitmap', (f: File) => new Promise((r) => sizes.set(f, r)));
  URL.createObjectURL = () => 'blob:x';
  URL.revokeObjectURL = () => {};
});

const png = (name: string) => new File(['x'], name, { type: 'image/png' });
const pick = (f: File) => fireEvent.change(document.querySelector('input.file') as HTMLInputElement, { target: { files: [f] } });
const decoded = (f: File) => sizes.get(f)!({ width: 100, height: 100, close() {} });

describe('ImageUpload in the decode window', () => {
  it('says it is busy while it reads the file', async () => {
    render(ImageUpload, { kind: 'badge', label: 'Picture' });
    await pick(png('a.png'));
    expect(screen.getByRole('status').textContent).toMatch(/Checking/);
  });

  it('uploads only the newest pick when two come before the first is read', async () => {
    render(ImageUpload, { kind: 'badge', label: 'Picture' });
    const a = png('a.png');
    const b = png('b.png');
    await pick(a);
    await pick(b);
    decoded(b);
    decoded(a); // the older one is read last: it must not start an upload
    await new Promise((r) => setTimeout(r, 10));
    expect(upload).toHaveBeenCalledTimes(1);
    expect((upload.mock.calls[0][0] as File).name).toBe('b.png');
  });
});

describe('ImageUpload checks badges like the API', () => {
  // A GIF of `frames` 1×1 frames, 50 ms each: only what the frame reader looks at.
  const gif = (frames: number) => {
    const out = [71, 73, 70, 56, 57, 97, 1, 0, 1, 0, 0x80, 0, 0, 0, 0, 0, 255, 255, 255];
    for (let i = 0; i < frames; i++) out.push(0x21, 0xf9, 4, 0, 5, 0, 0, 0, 0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 0x01, 0);
    out.push(0x3b);
    return new File([new Uint8Array(out)], 'a.gif', { type: 'image/gif' });
  };

  it('refuses an animation over 120 frames before uploading it', async () => {
    render(ImageUpload, { kind: 'badge', label: 'Picture' });
    const f = gif(121);
    await pick(f);
    decoded(f);
    await vi.waitFor(() => expect(screen.getByRole('alert').textContent).toMatch(/at most 120 frames \(this one has 121\)/));
    expect(upload).not.toHaveBeenCalled();
  });

  it('takes a JPEG badge and a 120-frame one', async () => {
    render(ImageUpload, { kind: 'badge', label: 'Picture' });
    const f = new File(['x'], 'a.jpg', { type: 'image/jpeg' });
    await pick(f);
    decoded(f);
    await vi.waitFor(() => expect(upload).toHaveBeenCalledTimes(1));
    const g = gif(120);
    await pick(g);
    decoded(g);
    await vi.waitFor(() => expect(upload).toHaveBeenCalledTimes(2));
  });

  it('says the badge rules and shows the made badge at 72 px', async () => {
    const made = { id: 'img2', kind: 'BADGE', host: 'https://cdn.cyex.tv/cosmetics/badges/def', animated: false, ready: true, width: 512, height: 512,
      files: ['1x', '2x', '4x', '128', '256', '512'].map((s) => ({ name: `${s}.webp`, width: 0, height: 0, isStatic: false })) };
    render(ImageUpload, { kind: 'badge', label: 'Picture', current: made as never });
    expect(screen.getByRole('status').textContent).toMatch(/PNG, JPEG, GIF or WebP, up to 8 MB; animated up to 120 frames and 10 seconds/);
    expect((document.querySelector('input.file') as HTMLInputElement).accept).toContain('image/jpeg');
    // Before the page knows the screen, 72 px at 1x.
    expect((document.querySelector('.prev img') as HTMLImageElement).getAttribute('src')).toBe('https://cdn.cyex.tv/cosmetics/badges/def/4x.webp');
  });
});

describe('ImageUpload when the made file won’t load', () => {
  const made = { id: 'img1', kind: 'BADGE', host: 'https://cdn.cyex.tv/cosmetics/badges/abc', animated: false, ready: true, width: 72, height: 72, files: [] };

  it('says the CDN refused it and keeps showing the picked file', async () => {
    upload.mockResolvedValue({ id: 'img1' });
    gql.mockResolvedValue({ cosmeticImage: made });
    render(ImageUpload, { kind: 'badge', label: 'Picture' });
    const f = png('a.png');
    await pick(f);
    decoded(f);
    await vi.waitFor(() => expect(screen.getByRole('status').textContent).toMatch(/Ready/));
    const img = document.querySelector('.prev img') as HTMLImageElement;
    expect(img.getAttribute('src')).toBe('https://cdn.cyex.tv/cosmetics/badges/abc/4x.webp');
    await fireEvent.error(img);
    expect(screen.getByRole('alert').textContent).toMatch(/cdn\.cyex\.tv won’t send it/);
    expect((document.querySelector('.prev img') as HTMLImageElement).getAttribute('src')).toBe('blob:x');
  });

  it('warns for a saved picture that won’t load too', async () => {
    render(ImageUpload, { kind: 'badge', label: 'Picture', current: made as never });
    await fireEvent.error(document.querySelector('.prev img') as HTMLImageElement);
    expect(screen.getByRole('alert').textContent).toMatch(/firewall rule is blocking \/cosmetics\//);
  });
});

describe('ImageUpload previews a badge at its original size', () => {
  it('shows original.webp at its real pixels, not a ladder size', () => {
    const files = ['1x', '2x', '4x', '128', '256', '512', 'original'].map((s) => ({ name: `${s}.webp`, width: s === 'original' ? 300 : 512, height: s === 'original' ? 300 : 512, isStatic: false }));
    const made = { id: 'img3', kind: 'BADGE', host: 'https://cdn.cyex.tv/cosmetics/badges/ghi', animated: false, ready: true, width: 300, height: 300, files };
    render(ImageUpload, { kind: 'badge', label: 'Picture', current: made as never });
    const img = document.querySelector('.prev img') as HTMLImageElement;
    expect(img.getAttribute('src')).toBe('https://cdn.cyex.tv/cosmetics/badges/ghi/original.webp');
    expect([img.getAttribute('width'), img.getAttribute('height')]).toEqual(['300', '300']);
    expect(document.querySelector('.prev.real')).not.toBeNull();
  });
});
