import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Dialog from './Dialog.svelte';

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
});

const text = (t: string) => createRawSnippet(() => ({ render: () => `<p>${t}</p>` }));
const esc = (d: HTMLElement) => {
  const e = new Event('cancel', { cancelable: true });
  d.dispatchEvent(e);
  return e.defaultPrevented;
};

function setup(props: Record<string, unknown> = {}) {
  const opener = document.createElement('button');
  document.body.append(opener);
  const r = render(Dialog, { props: { title: 'Rename', children: text('Body'), ...props } });
  const api = r.component as unknown as { open: () => void; close: () => void; isOpen: () => boolean };
  const dialog = [...document.querySelectorAll('dialog')].at(-1)!;
  opener.focus();
  api.open();
  return { api, dialog, opener };
}

describe('Dialog', () => {
  it('opens as a modal named by its title, with the body inside .body', () => {
    const { dialog } = setup();
    expect(dialog.hasAttribute('open')).toBe(true);
    expect(screen.getByRole('heading', { name: 'Rename' }).id).toBe(dialog.getAttribute('aria-labelledby'));
    expect(dialog.querySelector('.body p')?.textContent).toBe('Body');
  });

  it('Escape closes it and focus goes back to the opener', () => {
    const onclose = vi.fn();
    const { dialog, opener } = setup({ onclose });
    (dialog.querySelector('h2') as HTMLElement).focus();
    expect(esc(dialog)).toBe(false);
    dialog.close();
    expect(onclose).toHaveBeenCalledOnce();
    expect(document.activeElement).toBe(opener);
  });

  it('Escape does nothing while busy, and the caller can refuse it too', () => {
    const { dialog } = setup({ busy: true });
    expect(esc(dialog)).toBe(true);
    expect(dialog.hasAttribute('open')).toBe(true);
    const own = setup({ oncancel: (e: Event) => e.preventDefault() });
    expect(esc(own.dialog)).toBe(true);
  });

  it('a click on the backdrop closes it only when asked to, and never while busy', async () => {
    const plain = setup();
    await fireEvent.click(plain.dialog);
    expect(plain.dialog.hasAttribute('open')).toBe(true);
    plain.api.close();

    const dismiss = setup({ backdrop: true });
    await fireEvent.click(dismiss.dialog.querySelector('.body')!);
    expect(dismiss.dialog.hasAttribute('open')).toBe(true);
    await fireEvent.click(dismiss.dialog);
    expect(dismiss.dialog.hasAttribute('open')).toBe(false);

    const busy = setup({ backdrop: true, busy: true });
    await fireEvent.click(busy.dialog);
    expect(busy.dialog.hasAttribute('open')).toBe(true);
  });

  it('wraps the content in a form when given onsubmit, with the footer inside', async () => {
    const onsubmit = vi.fn((e: SubmitEvent) => e.preventDefault());
    const footer = createRawSnippet(() => ({ render: () => '<button type="submit">Save</button>' }));
    setup({ onsubmit, footer });
    await fireEvent.click(screen.getByRole('button', { name: 'Save', hidden: true }));
    expect(onsubmit).toHaveBeenCalledOnce();
    expect(document.querySelector('dialog form footer')).toBeTruthy();
  });
});
