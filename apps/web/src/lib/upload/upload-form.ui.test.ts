import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import UploadDropHarness from '../test/UploadDropHarness.svelte';

const { guard } = vi.hoisted(() => ({ guard: { fn: null as null | ((n: unknown) => void) } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), beforeNavigate: (f: (n: unknown) => void) => (guard.fn = f) }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/me/upload') } }));
URL.createObjectURL = () => 'blob:x';
URL.revokeObjectURL = () => {};
HTMLDialogElement.prototype.showModal = function () {
  this.setAttribute('open', '');
};

const png = (name = 'cat.png') => new File([new Uint8Array(40)], name, { type: 'image/png', lastModified: 1 });
const drop = (el: Element, files: File[]) => fireEvent.drop(el, { dataTransfer: { files } });
const hint = () => document.querySelector('#drop-hint')!.textContent ?? '';

describe('upload form drops', () => {
  it('a file dropped on the drop zone is counted once, with no "already in the list" note', async () => {
    render(UploadDropHarness);
    await drop(document.querySelector('.drop')!, [png()]);
    expect(document.querySelectorAll('.queue .item')).toHaveLength(1);
    expect(hint()).not.toMatch(/already/);
  });

  it('a file dropped elsewhere on the dialog is added once too', async () => {
    render(UploadDropHarness);
    await drop(screen.getByTestId('outer'), [png()]);
    expect(document.querySelectorAll('.queue .item')).toHaveLength(1);
    expect(hint()).not.toMatch(/already/);
  });
});

describe('upload form leave guard', () => {
  const unload = () => {
    const e = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(e);
    return e.defaultPrevented;
  };

  it('asks before the tab closes while a file waits, not with an empty queue', async () => {
    render(UploadDropHarness);
    expect(unload()).toBe(false);
    await drop(document.querySelector('.drop')!, [png()]);
    expect(unload()).toBe(true);
  });

  it('on the page, leaving for another page asks; in the dialog the queue stays, so it does not', async () => {
    for (const [inDialog, asks] of [[false, true], [true, false]] as const) {
      const { unmount } = render(UploadDropHarness, { inDialog });
      await drop(document.querySelector('.drop')!, [png()]);
      const cancel = vi.fn();
      guard.fn!({ cancel, willUnload: false, to: { url: new URL('http://localhost/me') } });
      expect(cancel.mock.calls.length > 0).toBe(asks);
      unmount();
    }
  });
});
