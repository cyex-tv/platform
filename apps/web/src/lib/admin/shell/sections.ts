// The admin sections, once: the rail (AdminShell) and the overview's index are both made from
// this list, so a section never shows in one and not the other. Each section says which
// permissions open it; `count` names the number the rail and the overview show beside it.
import type { IconName } from '$lib/components/Icon.svelte';
import type { RailGroup } from '$lib/shell/rail';
import { markCurrent } from '$lib/shell/rail';
import { USER_PAGE_PERMISSIONS } from '../permissions';

export type CountKey = 'review' | 'reports' | 'items' | 'orders' | 'codes';
export type Counts = Partial<Record<CountKey, number>>;
export type Section = {
  href: string;
  label: string;
  icon: IconName;
  /** One line for the overview's index. */
  text: string;
  /** Any one of these opens it. */
  any: string[];
  count?: CountKey;
  /** Open tickets: a number above 0 here waits for someone. */
  urgent?: boolean;
};
export type SectionGroup = { label: string; icon: IconName; items: Section[] };

const TICKETS = ['emotes.review', 'emotes.takedown', 'reports.manage'];
const COSMETICS = ['cosmetics.manage', 'cosmetics.grant'];

export const SECTIONS: SectionGroup[] = [
  {
    label: 'Tickets',
    icon: 'ticket',
    items: [
      { href: '/admin/review', label: 'Review', icon: 'inbox', text: 'Uploads waiting for a verdict, oldest first.', any: ['emotes.review', 'emotes.review_personal'], count: 'review', urgent: true },
      { href: '/admin/reports', label: 'Reports', icon: 'flag', text: 'Open reports, grouped by emote: dismiss or take down.', any: TICKETS, count: 'reports', urgent: true },
    ],
  },
  {
    label: 'Cosmetics',
    icon: 'palette',
    items: [
      { href: '/admin/cosmetics/paints', label: 'Paints', icon: 'brush', text: 'Name paints: make, edit, give and take away.', any: COSMETICS },
      { href: '/admin/cosmetics/badges', label: 'Badges', icon: 'badge', text: 'Chat badges: make, edit, give and take away.', any: COSMETICS },
      { href: '/admin/cosmetics/owners', label: 'Owners', icon: 'users', text: 'Who owns a badge or paint, and since when.', any: ['cosmetics.grant'] },
    ],
  },
  {
    label: 'Shop',
    icon: 'shopping-bag',
    items: [
      { href: '/admin/shop/items', label: 'Items', icon: 'tag', text: 'What is on sale: prices, sale windows, stock.', any: ['shop.manage'], count: 'items' },
      { href: '/admin/shop/orders', label: 'Orders', icon: 'list', text: 'Every purchase and gift. Refund a paid order.', any: ['shop.orders'], count: 'orders' },
      { href: '/admin/shop/codes', label: 'Codes', icon: 'key', text: 'Batches of redeem codes: make, hand out, turn off.', any: ['codes.manage'], count: 'codes' },
    ],
  },
  {
    label: 'People',
    icon: 'users',
    items: [
      { href: '/admin/users', label: 'Users', icon: 'users', text: 'One account on one page: restrictions, roles, billing, notes.', any: USER_PAGE_PERMISSIONS },
      { href: '/admin/roles', label: 'Roles', icon: 'key', text: 'Who may do what, and their limits.', any: ['roles.manage', 'roles.assign'] },
      { href: '/admin/grants', label: 'Give to many', icon: 'user-plus', text: 'One badge, paint or role for a list of people.', any: ['cosmetics.grant', 'roles.assign', 'roles.manage'] },
      { href: '/admin/entitlements', label: 'Entitlement graph', icon: 'network', text: 'How each badge, paint and role was given, and by whom.', any: ['entitlements.view', 'cosmetics.grant'] },
    ],
  },
  {
    label: 'System',
    icon: 'settings',
    items: [
      { href: '/admin/announcements', label: 'Announcements', icon: 'message-square', text: 'One line under the header for everyone.', any: ['site.announce'] },
      { href: '/admin/audit', label: 'Audit log', icon: 'scroll', text: 'Every audited change, newest first.', any: ['audit.view'] },
    ],
  },
];

/** The groups with only the sections `can` opens; empty groups drop out. */
export function visibleSections(can: (p: string) => boolean): SectionGroup[] {
  return SECTIONS.map((g) => ({ ...g, items: g.items.filter((s) => s.any.some(can)) })).filter((g) => g.items.length);
}

/** The rail: Overview, then a collapsible group per area, the current row marked. */
export function adminRail(can: (p: string) => boolean, counts: Counts, path: string): RailGroup[] {
  const groups: RailGroup[] = [
    { items: [{ href: '/admin', label: 'Overview', icon: 'gauge' }] },
    ...visibleSections(can).map((g) => ({
      toggle: { label: g.label, icon: g.icon },
      items: g.items.map((s) => ({ href: s.href, label: s.label, icon: s.icon, count: s.count ? counts[s.count] : undefined })),
    })),
  ];
  return markCurrent(groups, path, ['/admin']);
}

/** The section a path belongs to (the phone menu button names it). */
export function sectionFor(path: string): Section | null {
  let best: Section | null = null;
  for (const g of SECTIONS)
    for (const s of g.items)
      if ((path === s.href || path.startsWith(`${s.href}/`)) && (!best || s.href.length > best.href.length)) best = s;
  return best;
}
