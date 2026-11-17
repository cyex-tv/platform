// The signed-in destinations, in one place: the AccountMenu popover, the phone menu sheet and
// the /me rail all read these, so the same places have the same names and icons everywhere.
import type { IconName } from '$lib/components/Icon.svelte';

export type ShellLink = { href: string; label: string; icon: IconName; match: (path: string) => boolean };

const under = (base: string) => (p: string) => p === base || p.startsWith(`${base}/`);

/** What you make: your emotes, upload, sets, channels. */
export const LIBRARY: ShellLink[] = [
  { href: '/me', label: 'Your emotes', icon: 'smile', match: (p) => p === '/me' || p.startsWith('/me/emotes') },
  { href: '/me/upload', label: 'Upload', icon: 'upload', match: under('/me/upload') },
  { href: '/me/sets', label: 'Sets', icon: 'layers', match: under('/me/sets') },
  { href: '/me/channels', label: 'Channels', icon: 'tv', match: under('/me/channels') },
];

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
