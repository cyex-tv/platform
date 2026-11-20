import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ beforeNavigate: () => {}, goto: vi.fn(), afterNavigate: () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/admin/shop/codes') } }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$lib/tool/session.svelte', async (orig) => ({ ...(await orig<typeof import('$lib/tool/session.svelte')>()), can: () => true }));
vi.mock('$lib/cosmetics/store.svelte', () => ({ loadCatalog: async () => ({ badges: new Map(), paints: new Map() }) }));

beforeEach(() => {
  gql.mockReset().mockImplementation(async (_q: unknown, vars: Record<string, unknown>) => {
    if (vars && 'input' in vars) return { createCodeBatch: { batch: { id: 'K', name: 'x', prefix: 'X' }, codes: ['X-1'] } };
    return { codeBatches: { items: [], total: 0 }, shopItems: [] };
  });
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
});

async function open() {
  render(Page);
  const [btn] = await screen.findAllByRole('button', { name: /New batch/ });
  await fireEvent.click(btn);
  return document.querySelectorAll('dialog')[0] as HTMLDialogElement;
}
const field = (d: HTMLElement, label: string) => [...d.querySelectorAll('label.ui-field')].find((l) => l.textContent?.startsWith(label))!.querySelector('input,select') as HTMLInputElement;

describe('make-a-batch form', () => {
  it('focuses the first invalid field on the first submit, and announces the errors', async () => {
    const d = await open();
    await fireEvent.submit(d.querySelector('form')!);
    const name = field(d, 'Name');
    await vi.waitFor(() => expect(document.activeElement).toBe(name));
    expect(d.querySelector('.ui-error')?.getAttribute('role')).toBe('alert');
  });

  it('accepts 1.000 and 1000 as a count, refuses 1.00 and 1001', async () => {
    const d = await open();
    const amount = field(d, 'How many codes');
    for (const [v, bad] of [['1.000', false], ['1000', false], ['1.00', true], ['1001', true]] as const) {
      await fireEvent.input(amount, { target: { value: v } });
      await fireEvent.submit(d.querySelector('form')!);
      const msg = [...d.querySelectorAll('.ui-error')].some((e) => /Make 1 to 1\.000 codes/.test(e.textContent ?? ''));
      expect(msg, v).toBe(bad);
    }
  });
});
