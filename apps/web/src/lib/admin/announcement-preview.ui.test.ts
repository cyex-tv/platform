import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import AnnouncementPreview from './AnnouncementPreview.svelte';

describe('the announcement preview', () => {
  it('shows the kind, the line and the link as the bar will', () => {
    render(AnnouncementPreview, { kind: 'MAINTENANCE', text: '  Down at noon ', href: '/status', linkLabel: '' });
    expect(screen.getByRole('img', { name: 'Preview of the announcement' })).toBeTruthy();
    expect(screen.getByText('Maintenance')).toBeTruthy();
    expect(screen.getByText('Down at noon')).toBeTruthy();
    expect(screen.getByText('Learn more')).toBeTruthy();
  });

  it('asks for a line when empty and shows no link without a safe address', () => {
    render(AnnouncementPreview, { kind: 'INFO', text: '', href: null, linkLabel: 'Go' });
    expect(screen.getByText('Your line shows here.')).toBeTruthy();
    expect(screen.queryByText('Go')).toBeNull();
  });
});
