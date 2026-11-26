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

const SETS = [
  { id: 's1', name: 'Main' },
  { id: 's2', name: 'Subs' },
  { id: 's3', name: 'Seasonal' },
];
const ALL = ['EMOTES', 'SETS', 'CHANNELS', 'EMOTE_UPLOAD', 'EMOTE_MANAGE', 'PROFILE', 'BILLING_PAYMENT', 'BILLING_SUBSCRIPTION', 'EDITORS'];

function openDialog(props: Record<string, unknown> = {}, start: Record<string, unknown> = {}) {
  const r = render(EditorDialog, { held: new Set(ALL) as never, sets: SETS, onsave: async () => {}, describe: () => 'no', ...props });
  (r.component as unknown as { open: (...a: unknown[]) => void }).open(
    { id: 'U', displayName: 'Ana' },
    { permissions: ['EMOTES'], scope: { all: true }, ...start },
    'edit',
  );
  return r;
}
const trigger = () => screen.getByRole('button', { name: /Sets they can edit/ });
const panel = () => screen.queryByRole('dialog', { name: 'Sets they can edit' });
const setBox = (name: string) => screen.getByRole('checkbox', { name }) as HTMLInputElement;

describe('the sets dropdown', () => {
  it('stays open while ticking two sets, and the trigger reads "2 sets"', async () => {
    openDialog();
    await fireEvent.click(trigger());
    expect(panel()).not.toBeNull();
    await fireEvent.click(setBox('Main'));
    await fireEvent.click(setBox('Subs'));
    expect(panel()).not.toBeNull();
    expect(setBox('Main').checked).toBe(true);
    expect(setBox('Subs').checked).toBe(true);
    expect(trigger().textContent).toContain('2 sets');
  });

  it('closes on Confirm, focus back on the trigger, dialog still open', async () => {
    openDialog();
    await fireEvent.click(trigger());
    await fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    await vi.waitFor(() => expect(panel()).toBeNull());
    expect(document.activeElement).toBe(trigger());
    expect(document.querySelector('dialog')!.hasAttribute('open')).toBe(true);
  });

  it('closes on a pointerdown outside it', async () => {
    openDialog();
    await fireEvent.click(trigger());
    expect(panel()).not.toBeNull();
    await fireEvent.pointerDown(screen.getByText('Everything'));
    await vi.waitFor(() => expect(panel()).toBeNull());
    expect(document.querySelector('dialog')!.hasAttribute('open')).toBe(true);
  });

  it('closes on Esc without closing the dialog, and focus goes back to the trigger', async () => {
    openDialog();
    await fireEvent.click(trigger());
    const filter = screen.getByRole('searchbox', { name: 'Filter sets' });
    const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
    filter.dispatchEvent(esc);
    // A cancelled Esc keydown is what keeps the browser from closing the dialog.
    expect(esc.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(panel()).toBeNull());
    expect(document.activeElement).toBe(trigger());
    const cancel = new Event('cancel', { cancelable: true });
    document.querySelector('dialog')!.dispatchEvent(cancel);
    expect(document.querySelector('dialog')!.hasAttribute('open')).toBe(true);
  });

  it('opens with ArrowDown, focuses the filter, and the filter narrows the rows', async () => {
    openDialog();
    await fireEvent.keyDown(trigger(), { key: 'ArrowDown' });
    const filter = (await screen.findByRole('searchbox', { name: 'Filter sets' })) as HTMLInputElement;
    await vi.waitFor(() => expect(document.activeElement).toBe(filter));
    expect(screen.getAllByRole('checkbox', { name: /Main|Subs|Seasonal/ })).toHaveLength(3);
    await fireEvent.input(filter, { target: { value: 'su' } });
    expect(screen.queryByRole('checkbox', { name: 'Main' })).toBeNull();
    expect(screen.getByRole('checkbox', { name: 'Subs' })).toBeTruthy();
    // ↓ moves from the filter to the first row.
    await fireEvent.keyDown(filter, { key: 'ArrowDown' });
    expect(document.activeElement).toBe(screen.getByRole('checkbox', { name: /^All sets/ }));
  });

  it('a manager limited to some sets sees only theirs, never All sets', async () => {
    openDialog({ mine: { all: false, ids: ['s2'] } }, { scope: { all: false, ids: ['s2'] } });
    await fireEvent.click(trigger());
    expect(screen.queryByRole('checkbox', { name: /^All sets/ })).toBeNull();
    expect(screen.queryByRole('checkbox', { name: 'Main' })).toBeNull();
    await fireEvent.click(setBox('Subs'));
    expect(trigger().textContent).toContain('Pick sets');
  });
});

describe('the permissions', () => {
  const box = (name: string) => screen.getByRole('checkbox', { name }) as HTMLInputElement;

  it('ticks Sets and Channel settings independently', async () => {
    openDialog({}, { permissions: [] });
    await fireEvent.click(box('Create and manage sets'));
    expect(box('Create and manage sets').checked).toBe(true);
    expect(box('Channel settings').checked).toBe(false);
    await fireEvent.click(box('Channel settings'));
    expect(box('Channel settings').checked).toBe(true);
    expect(box('Channel settings').disabled).toBe(false);
    await fireEvent.click(box('Channel settings'));
    expect(box('Channel settings').checked).toBe(false);
    expect(box('Create and manage sets').checked).toBe(true);
  });

  it('never says channel settings come with sets, and gives each box its line as a tooltip', () => {
    openDialog({}, { permissions: ['SETS'] });
    expect(document.body.textContent).not.toMatch(/Comes with managing sets/);
    const sets = box('Create and manage sets');
    const tip = document.getElementById(sets.getAttribute('aria-describedby')!)!;
    expect(tip.getAttribute('role')).toBe('tooltip');
    expect(tip.textContent).toMatch(/resize/);
  });
});
