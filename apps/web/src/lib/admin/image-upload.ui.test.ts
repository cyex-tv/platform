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
