import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import Harness from '$lib/test/InspectorHarness.svelte';

afterEach(cleanup);

describe('InspectorLayout with PanelBlock and LinkTabs', () => {
  it('puts the panel in a labelled aside with an uppercase label, beside the head, tabs and content', () => {
    render(Harness);
    const aside = screen.getByRole('complementary', { name: 'About Main' });
    expect(aside.dataset.testid).toBe('panel');
    expect(screen.getByRole('heading', { level: 2, name: 'Set' })).toBeTruthy();
    expect(screen.getByRole('heading', { level: 1, name: 'Main' })).toBeTruthy();
    expect(screen.getByText('The grid')).toBeTruthy();
    // Facts: a block with its own level-3 label and a facts list.
    expect(screen.getByRole('heading', { level: 3, name: 'Facts' })).toBeTruthy();
    expect(aside.querySelector('dl.kv dd')?.textContent).toBe('24');
  });

  it('marks the current tab (and only that one) and gives the tabs a name', () => {
    render(Harness);
    const nav = screen.getByRole('navigation', { name: 'Sections' });
    const current = nav.querySelectorAll('[aria-current="page"]');
    expect(current.length).toBe(1);
    expect(current[0].textContent).toContain('Emotes');
    expect(current[0].textContent).toContain('24');
  });

  it('opens a folded block on purpose, and keeps its link', async () => {
    const user = userEvent.setup();
    render(Harness);
    const toggle = screen.getByRole('button', { name: /Editors/ });
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    const body = document.getElementById(toggle.getAttribute('aria-controls')!)!;
    expect(body.hidden).toBe(true);
    await user.click(toggle);
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(body.hidden).toBe(false);
    expect(screen.getByRole('link', { name: 'Manage' }).getAttribute('href')).toBe('/me/editors');
  });

  it('with fold, the narrow panel has a Show/Hide toggle that shuts its body', async () => {
    const user = userEvent.setup();
    render(Harness, { fold: true });
    const toggle = screen.getByRole('button', { name: /About Main/ });
    const body = document.getElementById(toggle.getAttribute('aria-controls')!)!;
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    expect(body.classList.contains('shut')).toBe(true);
    await user.click(toggle);
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(body.classList.contains('shut')).toBe(false);
  });

  it('without fold there is no toggle', () => {
    render(Harness);
    expect(screen.queryByRole('button', { name: /About Main/ })).toBeNull();
  });
});
