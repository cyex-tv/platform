import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import NotificationRow from './NotificationRow.svelte';

const note = (read: boolean) => ({ id: 'n1', kind: 'emote_approved', payload: JSON.stringify({ emoteName: 'Kappa' }), createdAt: '2026-11-16T23:10:00Z', read });

vi.mock('$env/dynamic/public', () => ({ env: {} }));

describe('NotificationRow mark as read', () => {
  it('has its own button on an unread one, which marks just that one and hands focus to the row', async () => {
    const onread = vi.fn();
    render(NotificationRow, { note: note(false), onread });
    const btn = screen.getByRole('button', { name: 'Mark as read' });
    await userEvent.setup().click(btn);
    expect(onread).toHaveBeenCalledOnce();
    expect(onread.mock.calls[0][0].id).toBe('n1');
  });

  it('has no button on a read one, or without onread', () => {
    render(NotificationRow, { note: note(true), onread: () => {} });
    expect(screen.queryByRole('button', { name: 'Mark as read' })).toBeNull();
  });

  it('the button is reachable by keyboard and Enter marks it', async () => {
    const onread = vi.fn();
    render(NotificationRow, { note: note(false), onread });
    const user = userEvent.setup();
    await user.tab();
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Mark as read' }));
    await user.keyboard('{Enter}');
    expect(onread).toHaveBeenCalledOnce();
  });
});
