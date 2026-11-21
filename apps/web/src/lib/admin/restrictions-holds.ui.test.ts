import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Restrictions from './Restrictions.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$lib/tool/session.svelte', async (orig) => ({ ...(await orig<typeof import('$lib/tool/session.svelte')>()), can: () => true }));

const off = { uploadsBlocked: false, ticketsBlocked: false, blocked: false };
const held = { ...off, uploadsBlocked: true, uploadsBlockedUntil: '2026-12-01T10:00:00Z' };

describe('Restrictions as settings rows', () => {
  beforeEach(() => {
    gql.mockReset();
    gql.mockResolvedValue({ accountRestrictions: [] });
    HTMLDialogElement.prototype.showModal = vi.fn();
  });

  it('shows each restriction as a switch that is on while it holds', async () => {
    render(Restrictions, { accountId: 'A', status: held });
    await screen.findByText('Never restricted.');
    expect((screen.getByRole('switch', { name: 'Uploads' }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole('switch', { name: 'Whole account' }) as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText(/Uploads stopped until/)).toBeTruthy();
  });

  it('turning a switch on asks for a reason and a length instead of restricting at once', async () => {
    render(Restrictions, { accountId: 'A', status: off });
    await screen.findByText('Never restricted.');
    await fireEvent.click(screen.getByRole('switch', { name: 'Uploads' }));
    expect(HTMLDialogElement.prototype.showModal).toHaveBeenCalledOnce();
    expect(gql).toHaveBeenCalledTimes(1); // only the history load
  });

  it('turning a switch off lifts the restriction', async () => {
    gql.mockImplementation(async (q: { definition?: unknown }, v: { kind?: string }) => (v?.kind ? { liftRestriction: off } : { accountRestrictions: [] }));
    render(Restrictions, { accountId: 'A', status: held });
    await screen.findByText('Never restricted.');
    await fireEvent.click(screen.getByRole('switch', { name: 'Uploads' }));
    expect(gql.mock.calls.some((c) => c[1]?.kind === 'UPLOADS' && c[1]?.accountId === 'A')).toBe(true);
  });

  it('keeps the switches off-limits on your own account', async () => {
    render(Restrictions, { accountId: 'A', status: off, self: true });
    await screen.findByText('Never restricted.');
    expect((screen.getByRole('switch', { name: 'Uploads' }) as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText('You can’t restrict your own account.')).toBeTruthy();
  });
});
