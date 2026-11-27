import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import Harness from '../test/PopoverHoverHarness.svelte';

// jsdom has no PointerEvent: a plain event carrying the pointer type.
const ptr = (el: Element, type: string, pointerType = 'mouse') => {
  const e = new Event(type);
  Object.defineProperty(e, 'pointerType', { value: pointerType });
  return fireEvent(el, e);
};
const over = (el: Element) => ptr(el, 'pointerenter');
const out = (el: Element) => ptr(el, 'pointerleave');

describe('Popover with hover', () => {
  it('opens after a short delay without taking focus, and closes after leaving', async () => {
    render(Harness);
    const trigger = screen.getByRole('button', { name: 'Who' });
    const root = trigger.parentElement!;
    await over(root);
    expect(screen.queryByRole('menu')).toBeNull(); // not at once: passing over doesn't flicker
    await screen.findByRole('menu');
    expect(document.activeElement).not.toBe(screen.getByRole('menuitem'));
    await out(root);
    expect(screen.queryByRole('menu')).not.toBeNull(); // a grace period
    await new Promise((r) => setTimeout(r, 400));
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('passing over without staying opens nothing', async () => {
    render(Harness);
    const root = screen.getByRole('button', { name: 'Who' }).parentElement!;
    await over(root);
    await out(root);
    await new Promise((r) => setTimeout(r, 400));
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('a click while it is open from hover keeps it open; touch pointers never open it', async () => {
    render(Harness);
    const trigger = screen.getByRole('button', { name: 'Who' });
    const root = trigger.parentElement!;
    await ptr(root, 'pointerenter', 'touch');
    await new Promise((r) => setTimeout(r, 300));
    expect(screen.queryByRole('menu')).toBeNull();
    await over(root);
    await screen.findByRole('menu');
    await fireEvent.click(trigger);
    await out(root);
    await new Promise((r) => setTimeout(r, 400));
    expect(screen.queryByRole('menu')).not.toBeNull();
  });
});
