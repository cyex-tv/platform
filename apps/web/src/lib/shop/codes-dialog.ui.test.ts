import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import CodesDialog from './CodesDialog.svelte';

const { copy } = vi.hoisted(() => ({ copy: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/clipboard.svelte', () => ({ copy: (...a: unknown[]) => copy(...a) }));

beforeEach(() => {
  copy.mockReset().mockResolvedValue(true);
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
  URL.createObjectURL = () => 'blob:x';
  URL.revokeObjectURL = () => {};
});

async function shown() {
  const r = render(CodesDialog);
  (r.component as unknown as { show: (n: string, c: string[]) => void }).show('Launch', ['LAUNCH-AAAA', 'LAUNCH-BBBB']);
  await screen.findByText(/You won’t see these again/);
  return document.querySelector('dialog')!;
}
const esc = (d: HTMLDialogElement) => {
  const e = new Event('cancel', { cancelable: true });
  d.dispatchEvent(e);
  return e.defaultPrevented;
};

describe('codes shown once', () => {
  it('Esc is refused until the codes were copied or downloaded', async () => {
    const d = await shown();
    expect(esc(d)).toBe(true);
    expect(d.hasAttribute('open')).toBe(true);
    expect(await screen.findByText(/Close without saving\? These codes can’t be shown again/)).toBeTruthy();
  });

  it('"Close without saving" asks, Go back keeps them, Close anyway closes', async () => {
    const d = await shown();
    await fireEvent.click(screen.getByRole('button', { name: 'Close without saving' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Go back' }));
    expect(d.hasAttribute('open')).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Close without saving' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Close anyway' }));
    expect(d.hasAttribute('open')).toBe(false);
  });

  it('after Copy all, Esc closes normally and the way out is Done', async () => {
    const d = await shown();
    await fireEvent.click(screen.getByRole('button', { name: /Copy all/ }));
    await screen.findByRole('button', { name: 'Done' });
    expect(esc(d)).toBe(false);
    expect(screen.queryByRole('button', { name: 'Close without saving' })).toBeNull();
  });

  it('a download counts as saved too', async () => {
    const d = await shown();
    await fireEvent.click(screen.getByRole('button', { name: /Download .txt/ }));
    expect(esc(d)).toBe(false);
  });

  it('a failed copy does not count as saved', async () => {
    copy.mockResolvedValue(false);
    document.execCommand = () => false;
    const d = await shown();
    await fireEvent.click(screen.getByRole('button', { name: /Copy all/ }));
    await screen.findByText(/Couldn’t copy/);
    expect(esc(d)).toBe(true);
  });

  it('asks the browser before the tab closes, until saved', async () => {
    await shown();
    const before = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(before);
    expect(before.defaultPrevented).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: /Download .txt/ }));
    const after = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(after);
    expect(after.defaultPrevented).toBe(false);
  });
});
