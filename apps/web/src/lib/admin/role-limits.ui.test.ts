import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import type { RoleLimitFieldsFragment } from '$lib/gql/graphql';
import RoleLimits from './RoleLimits.svelte';

const limits: RoleLimitFieldsFragment[] = [
  { key: 'gql_account', group: 'REQUESTS', default: 600, max: 1_000_000, windowSeconds: 60, value: null },
  { key: 'uploads_per_hour', group: 'UPLOADS', default: 20, max: 1_000_000, windowSeconds: 3600, value: null },
  { key: 'pending_emotes', group: 'UPLOADS', default: 20, max: 1_000_000, windowSeconds: null, value: 100 },
  { key: 'data_export', group: 'ACCOUNT', default: 5, max: 1_000_000, windowSeconds: 3600, value: null },
];

describe('the role editor’s Limits', () => {
  it('shows each limit in its group with the default as the placeholder and the override filled in', () => {
    render(RoleLimits, { limits, values: { gql_account: '', uploads_per_hour: '', pending_emotes: '100', data_export: '' } });
    for (const g of ['Requests', 'Uploads', 'Account actions']) expect(screen.getByRole('group', { name: g })).toBeTruthy();
    const api = screen.getByLabelText('API requests') as HTMLInputElement;
    expect(api.placeholder).toBe('600');
    expect(api.value).toBe('');
    expect((screen.getByLabelText('Waiting for review') as HTMLInputElement).value).toBe('100');
    // The tooltip is the box's description: what it counts and its default.
    const tip = document.getElementById(api.getAttribute('aria-describedby')!.split(' ')[0])!;
    expect(tip.getAttribute('role')).toBe('tooltip');
    expect(tip.textContent).toContain('Default: 600 a minute.');
    const pendingTip = document.getElementById('limit-pending_emotes-tip')!;
    expect(pendingTip.textContent).toContain('Default: 20 at once.');
  });

  it('says what is wrong with a box once it is typed in, and every problem once tried', async () => {
    const { rerender } = render(RoleLimits, { limits, values: { gql_account: '', uploads_per_hour: '0', pending_emotes: '', data_export: '' } });
    // Not typed in yet, not tried: no error.
    expect(screen.queryByText(/^1 to/)).toBeNull();
    const api = screen.getByLabelText('API requests') as HTMLInputElement;
    await fireEvent.input(api, { target: { value: 'fast' } });
    expect(screen.getByText('A whole number, or empty for the default.')).toBeTruthy();
    expect(api.getAttribute('aria-invalid')).toBe('true');
    await rerender({ tried: true });
    expect(screen.getByText('1 to 1.000.000.')).toBeTruthy();
  });

  it('can be shown read-only', () => {
    render(RoleLimits, { limits, values: {}, disabled: true });
    expect((screen.getByLabelText('Emote uploads') as HTMLInputElement).disabled).toBe(true);
  });
});
