import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import AdminLink from './AdminLink.svelte';
import { adminUserHref, canOpenInAdmin } from './admin-link';

describe('Open in admin', () => {
  it('shows for the permissions the admin user page needs, to nobody else', () => {
    expect(canOpenInAdmin(null)).toBe(false);
    expect(canOpenInAdmin({ permissions: [] })).toBe(false);
    expect(canOpenInAdmin({ permissions: ['emotes.review', 'cosmetics.grant', 'audit.view'] })).toBe(false);
    for (const p of ['users.view', 'users.restrict', 'users.restrict_uploads', 'users.restrict_tickets', 'users.block']) {
      expect(canOpenInAdmin({ permissions: ['x', p] })).toBe(true);
    }
  });

  it('links to the admin page of that account', () => {
    expect(adminUserHref('01ABC')).toBe('/admin/users/01ABC');
    expect(adminUserHref('a/b')).toBe('/admin/users/a%2Fb');
  });

  it('renders no markup without a session', () => {
    const { body } = render(AdminLink, { props: { accountId: '01ABC' } });
    expect(body).not.toContain('admin');
    expect(body).not.toContain('<a');
  });
});
