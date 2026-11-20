import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import PopoverHarness from '../test/PopoverHarness.svelte';

describe('Popover', () => {
  it('opens on the trigger, moves focus in, and ties the trigger to the panel', async () => {
    const user = userEvent.setup();
    render(PopoverHarness);
    const trigger = screen.getByRole('button', { name: 'Open' });
    expect(trigger.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByRole('dialog')).toBeNull();

    await user.click(trigger);
    const panel = await screen.findByRole('dialog', { name: 'Filters' });
    expect(trigger.getAttribute('aria-expanded')).toBe('true');
    expect(trigger.getAttribute('aria-controls')).toBe(panel.id);
    await vi.waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Inside' })));
  });

  it('closes on the trigger again', async () => {
    const user = userEvent.setup();
    const onclose = vi.fn();
    render(PopoverHarness, { onclose });
    const trigger = screen.getByRole('button', { name: 'Open' });
    await user.click(trigger);
    await screen.findByRole('dialog');
    await user.click(trigger);
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('closes on Escape and puts focus back on the trigger', async () => {
    const user = userEvent.setup();
    const onclose = vi.fn();
    render(PopoverHarness, { onclose });
    const trigger = screen.getByRole('button', { name: 'Open' });
    await user.click(trigger);
    await vi.waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Inside' })));
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(trigger);
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('closes on a press outside, but not inside, and when focus leaves', async () => {
    const user = userEvent.setup();
    render(PopoverHarness);
    const trigger = screen.getByRole('button', { name: 'Open' });
    await user.click(trigger);
    await screen.findByRole('dialog');
    await user.click(screen.getByRole('button', { name: 'Inside' }));
    expect(screen.queryByRole('dialog')).not.toBeNull();
    await user.click(screen.getByRole('button', { name: 'Outside' }));
    expect(screen.queryByRole('dialog')).toBeNull();

    await user.click(trigger);
    await screen.findByRole('dialog');
    screen.getByRole('button', { name: 'Outside' }).focus();
    await vi.waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });
});
