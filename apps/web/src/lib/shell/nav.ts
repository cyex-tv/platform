// The header's main links and the channel list the empty header search shows. Pure, so the
// rules (who sees which link, how channels are ordered) are tested.
import type { IconName } from '$lib/components/Icon.svelte';
import { ADMIN, EMOTES, type ShellLink } from './account';

const under = (base: string) => (p: string) => p === base || p.startsWith(`${base}/`);

export const STORE: ShellLink = { href: '/shop', label: 'Shop', icon: 'shopping-bag', match: under('/shop') };
export const SUBSCRIPTION: ShellLink = {
  href: '/me/subscription',
  label: 'Subscription',
  icon: 'star',
  match: under('/me/subscription'),
};

/** The fields of the signed-in account the nav reads. */
export type NavViewer = { permissions: readonly string[] } | null | undefined;

/**
 * Whether the account has an active subscription. There are no subscriptions yet, so nobody
 * does; this is the one place to change once they exist.
 */
export function hasSubscription(_me: NavViewer): boolean {
  return false;
}

/**
 * The header's links: Emotes for everyone; signed in also Store, Subscription for subscribers
 * and Admin for staff. Signed out the header stays as it was (Emotes only).
 */
export function headerLinks(me: NavViewer, admin: boolean): ShellLink[] {
  if (!me) return [EMOTES];
  const out = [EMOTES, STORE];
  if (hasSubscription(me)) out.push(SUBSCRIPTION);
  if (admin) out.push(ADMIN);
  return out;
}

export type ChannelRole = 'OWNER' | 'ACCOUNT_EDITOR' | 'SET_EDITOR';

/** Highest first: your own channels, then accounts you edit, then accounts whose sets you edit. */
export const ROLE_ORDER: readonly ChannelRole[] = ['OWNER', 'ACCOUNT_EDITOR', 'SET_EDITOR'];

export const ROLE_LABEL: Record<ChannelRole, string> = {
  OWNER: 'Owner',
  ACCOUNT_EDITOR: 'Editor',
  SET_EDITOR: 'Set editor',
};

export const ROLE_ICON: Record<ChannelRole, IconName> = { OWNER: 'circle-user', ACCOUNT_EDITOR: 'users', SET_EDITOR: 'layers' };

type Ranked = { role: ChannelRole; channel: { login: string; platform: string } };

/** By role (owner, account editor, set editor), then login A to Z ignoring case, then platform. Unknown roles go last. */
export function sortChannels<T extends Ranked>(list: readonly T[]): T[] {
  const rank = (r: string) => {
    const i = ROLE_ORDER.indexOf(r as ChannelRole);
    return i < 0 ? ROLE_ORDER.length : i;
  };
  return [...list].sort(
    (a, b) =>
      rank(a.role) - rank(b.role) ||
      a.channel.login.localeCompare(b.channel.login, 'en', { sensitivity: 'base' }) ||
      a.channel.platform.localeCompare(b.channel.platform),
  );
}
