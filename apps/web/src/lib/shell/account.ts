// The signed-in destinations, in one place: the AccountMenu popover, the phone menu sheet and
// the profile rail all read these, so the same places have the same names and icons everywhere.
import type { IconName } from '$lib/components/Icon.svelte';

export type ShellLink = { href: string; label: string; icon: IconName; match: (path: string, search?: URLSearchParams) => boolean };

const under = (base: string) => (p: string) => p === base || p.startsWith(`${base}/`);

/** Your profile is your dashboard (/users/<id>): its sections are ?s= tabs. */
const profile = (id: string, s = '') => `/users/${encodeURIComponent(id)}${s ? `?s=${s}` : ''}`;
const on = (id: string, s: string | null) => (p: string, q?: URLSearchParams) =>
  p === profile(id) && (q?.get('s') ?? null) === s;

/** Your profile: what everyone else sees, with your edit controls. */
export function profileLink(id: string): ShellLink {
  return { href: profile(id), label: 'Profile', icon: 'circle-user', match: on(id, null) };
}

/** The account menu's list after Profile: your emotes and your cosmetics, both on the profile. */
export function menuLinks(id: string): ShellLink[] {
  return [
    {
      href: profile(id, 'uploads'),
      label: 'Your emotes',
      icon: 'smile',
      match: (p, q) => on(id, 'uploads')(p, q) || p.startsWith('/me/emotes'),
    },
    { href: profile(id, 'cosmetics'), label: 'Cosmetics', icon: 'brush', match: on(id, 'cosmetics') },
  ];
}

export const UPLOAD: ShellLink = { href: '/me/upload', label: 'Upload', icon: 'upload', match: under('/me/upload') };

export const ACCOUNT: ShellLink = { href: '/me/account', label: 'Account', icon: 'user', match: under('/me/account') };
export const ADMIN: ShellLink = { href: '/admin', label: 'Admin', icon: 'shield', match: under('/admin') };
export const EMOTES: ShellLink = {
  href: '/search',
  label: 'Emotes',
  icon: 'grid',
  match: (p) => p.startsWith('/search') || p.startsWith('/emotes'),
};

/** The account rows after the library: Account, then Admin for staff. */
export function settingsLinks(admin: boolean): ShellLink[] {
  return admin ? [ACCOUNT, ADMIN] : [ACCOUNT];
}

/** "TWITCH" → "Twitch". */
export function platformName(p: string | null | undefined): string | null {
  return p ? `${p.charAt(0)}${p.slice(1).toLowerCase()}` : null;
}
