import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import PaintStudio from './PaintStudio.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ beforeNavigate: () => {}, goto: vi.fn(), afterNavigate: () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/me/paints') } }));

let decode: (f: File) => Promise<unknown>;
beforeEach(() => {
  decode = async () => ({ width: 128, height: 128, close() {} });
  vi.stubGlobal('createImageBitmap', (f: File) => decode(f));
  URL.createObjectURL = () => 'blob:x';
  URL.revokeObjectURL = () => {};
  HTMLCanvasElement.prototype.getContext = (() => ({ drawImage() {} })) as never;
  HTMLCanvasElement.prototype.toBlob = function (cb: BlobCallback) {
    cb(new Blob(['x'], { type: 'image/png' }));
  };
});

const png = (name: string) => new File(['x'], name, { type: 'image/png' });

describe('paint picture file', () => {
  it('forgets the earlier picture when the next file can’t be read', async () => {
    render(PaintStudio, { backend: null });
    await fireEvent.click(await screen.findByRole('radio', { name: /Animated or picture/ }));
    await fireEvent.click(await screen.findByRole('radio', { name: /Upload a file/ }));
    const input = (await screen.findByLabelText(/GIF, PNG, APNG or WebP/)) as HTMLInputElement;
    await fireEvent.change(input, { target: { files: [png('a.png')] } });
    await screen.findByText(/128 × 128 px/);
    decode = async () => {
      throw new Error('bad');
    };
    await fireEvent.change(input, { target: { files: [png('b.png')] } });
    await screen.findAllByText(/Couldn’t read that picture/);
    expect(screen.queryByText(/128 × 128 px/)).toBeNull();
  });
});
