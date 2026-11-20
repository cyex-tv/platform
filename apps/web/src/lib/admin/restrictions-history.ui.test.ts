import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Restrictions from './Restrictions.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));

const row = (i: number) => ({ id: `R${i}`, kind: 'UPLOADS', reason: `reason ${i}`, endsAt: null, createdBy: null, createdAt: '2026-11-01T10:00:00Z', liftedAt: '2026-11-02T10:00:00Z', liftedBy: null, active: false });
const status = { uploadsBlocked: false, ticketsBlocked: false, blocked: false };

describe('Restrictions history', () => {
  it('shows the first eight and lets staff show all of them', async () => {
    gql.mockResolvedValue({ accountRestrictions: Array.from({ length: 11 }, (_, i) => row(i)) });
    render(Restrictions, { accountId: 'A', status });
    await screen.findByText('reason 0');
    expect(screen.queryByText('reason 8')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Show all 11' }));
    expect(screen.getByText('reason 10')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Show fewer' }));
    expect(screen.queryByText('reason 10')).toBeNull();
  });

  it('has no button when everything already fits', async () => {
    gql.mockResolvedValue({ accountRestrictions: [row(1), row(2)] });
    render(Restrictions, { accountId: 'A', status });
    await screen.findByText('reason 1');
    expect(screen.queryByRole('button', { name: /Show all/ })).toBeNull();
  });
});
