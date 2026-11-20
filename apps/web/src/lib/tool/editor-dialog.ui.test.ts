import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EditorDialog from './EditorDialog.svelte';

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
});

describe('editor dialog while a save is on its way', () => {
  it('ignores Esc, Close and Cancel until the answer is in', async () => {
    let finish!: () => void;
    const onsave = vi.fn(() => new Promise<void>((r) => (finish = r)));
    const r = render(EditorDialog, { held: new Set(['SETS', 'CHANNELS', 'EMOTES']) as never, sets: [], onsave, describe: () => 'no' });
    (r.component as unknown as { open: (...a: unknown[]) => void }).open({ id: 'U', displayName: 'Ana' }, { permissions: ['EMOTES'], scope: { all: true, ids: [] } }, 'add');
    const d = document.querySelector('dialog')!;
    await fireEvent.submit(d.querySelector('form')!);
    expect(onsave).toHaveBeenCalled();
    await screen.findByText('Saving…');
    const esc = new Event('cancel', { cancelable: true });
    d.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(true);
    expect((screen.getByRole('button', { name: 'Close' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
    finish();
    await vi.waitFor(() => expect(d.hasAttribute('open')).toBe(false));
  });

  it('closes on Esc when nothing is saving', async () => {
    const r = render(EditorDialog, { held: new Set(['EMOTES']) as never, sets: [], onsave: async () => {}, describe: () => 'no' });
    (r.component as unknown as { open: (...a: unknown[]) => void }).open({ id: 'U', displayName: 'Ana' }, { permissions: ['EMOTES'], scope: { all: true, ids: [] } }, 'add');
    const esc = new Event('cancel', { cancelable: true });
    document.querySelector('dialog')!.dispatchEvent(esc);
    expect(esc.defaultPrevented).toBe(false);
  });
});
