// Page tabs with icons: every icon tab keeps its words as its accessible name (the words may be hidden
// under 700px, where the icon is the tab and the name shows in a tooltip), the current one is marked,
// and the arrow keys move between them.
import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import LinkTabs from './LinkTabs.svelte';

afterEach(cleanup);

const items = [
  { href: '/u?s=overview', label: 'Overview', icon: 'gauge' as const },
  { href: '/u', label: 'Uploaded emotes', icon: 'upload' as const, count: '12', current: true },
  { href: '/u?s=sets', label: 'Emote sets', icon: 'folder' as const },
];

describe('LinkTabs with icons', () => {
  it('gives each icon tab an icon and the label (and count) as its aria-label', () => {
    render(LinkTabs, { label: 'nora’s profile', items });
    const nav = screen.getByRole('navigation', { name: 'nora’s profile' });
    const links = [...nav.querySelectorAll('a')];
    expect(links.map((a) => a.getAttribute('aria-label'))).toEqual(['Overview', 'Uploaded emotes, 12', 'Emote sets']);
    for (const a of links) expect(a.querySelector('svg')).toBeTruthy();
  });

  it('marks only the current tab', () => {
    render(LinkTabs, { label: 'Sections', items });
    const current = screen.getByRole('navigation').querySelectorAll('[aria-current="page"]');
    expect(current.length).toBe(1);
    expect(current[0].getAttribute('aria-label')).toBe('Uploaded emotes, 12');
  });

  it('moves focus with the arrow keys', async () => {
    const user = userEvent.setup();
    render(LinkTabs, { label: 'Sections', items });
    screen.getByRole('link', { name: 'Overview' }).focus();
    await user.keyboard('{ArrowRight}');
    expect(document.activeElement?.getAttribute('aria-label')).toBe('Uploaded emotes, 12');
    await user.keyboard('{End}');
    expect(document.activeElement?.getAttribute('aria-label')).toBe('Emote sets');
  });

  it('a tab without an icon stays words only', () => {
    render(LinkTabs, { label: 'Sections', items: [{ href: '/a', label: 'Emotes' }] });
    const a = screen.getByRole('link', { name: 'Emotes' });
    expect(a.getAttribute('aria-label')).toBeNull();
    expect(a.querySelector('svg')).toBeNull();
  });
});
