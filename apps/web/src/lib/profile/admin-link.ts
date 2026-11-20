// "Open in admin" at the bottom of a profile: the same rule as the admin user page itself
// (abuse::admin_account: users.view, or any of the user restrictions). Owners hold every
// permission in their session, so they pass too. Anyone else gets no markup at all.
const NEEDED = ['users.view', 'users.restrict', 'users.restrict_uploads', 'users.restrict_tickets', 'users.block'];

export function canOpenInAdmin(me: { permissions: readonly string[] } | null | undefined): boolean {
  return !!me && NEEDED.some((p) => me.permissions.includes(p));
}

export const adminUserHref = (accountId: string) => `/admin/users/${encodeURIComponent(accountId)}`;
