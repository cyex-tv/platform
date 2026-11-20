import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import Toasts from '../tool/Toasts.svelte';
import { notify, toasts } from '../tool/notices.svelte';

afterEach(() => {
  vi.useRealTimers();
  toasts.splice(0);
});

describe('Toasts', () => {
  it('shows a notice as a status and an error as an alert', async () => {
    render(Toasts);
    notify('Saved.');
    notify('It broke.', true);
    flushSync();
    expect((await screen.findByRole('status')).textContent).toContain('Saved.');
    expect((await screen.findByRole('alert')).textContent).toContain('It broke.');
  });

  it('keeps at most three of each kind, dropping the oldest', async () => {
    render(Toasts);
    for (const t of ['one', 'two', 'three', 'four']) notify(t);
    for (const t of ['e1', 'e2', 'e3', 'e4']) notify(t, true);
    flushSync();
    await vi.waitFor(() => {
      expect(screen.getAllByRole('status')).toHaveLength(3);
      expect(screen.getAllByRole('alert')).toHaveLength(3);
    });
    const shown = screen.getAllByRole('status').map((n) => n.textContent).join(' ');
    expect(shown).not.toContain('one');
    expect(shown).toContain('four');
  });

  it('a repeat bumps a count instead of stacking', async () => {
    render(Toasts);
    notify('Copied.');
    notify('Copied.');
    notify('Copied.');
    flushSync();
    await vi.waitFor(() => expect(screen.getAllByRole('status')).toHaveLength(1));
    expect(screen.getByRole('status').textContent).toContain('×3');
  });

  it('a toast with an action never merges, runs it once and goes', async () => {
    const user = userEvent.setup();
    render(Toasts);
    const run = vi.fn();
    notify('Deleted.', false, { action: { label: 'Undo', run } });
    notify('Deleted.', false, { action: { label: 'Undo', run } });
    flushSync();
    await vi.waitFor(() => expect(screen.getAllByRole('button', { name: 'Undo' })).toHaveLength(2));
    await user.click(screen.getAllByRole('button', { name: 'Undo' })[0]);
    expect(run).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => expect(screen.getAllByRole('button', { name: 'Undo' })).toHaveLength(1));
  });

  it('can be dismissed, and goes by itself after its time', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    render(Toasts);
    notify('First.');
    notify('Second.');
    flushSync();
    await vi.waitFor(() => expect(screen.getAllByRole('status')).toHaveLength(2));
    await user.click(screen.getAllByRole('button', { name: 'Dismiss' })[0]);
    await vi.waitFor(() => expect(screen.getAllByRole('status')).toHaveLength(1));
    vi.advanceTimersByTime(3600);
    await vi.waitFor(() => expect(screen.queryAllByRole('status')).toHaveLength(0));
  });
});
