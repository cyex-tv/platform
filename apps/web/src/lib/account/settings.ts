// Settings (opened from the account menu, like 7TV's): one rail, Appearance / Notifications /
// Hidden / Billing. Account and Editors moved to your profile (owner rule, 2026/11/22: owner-only
// tabs ?s=account and ?s=editors); their old addresses (/me/settings/account, /me/account,
// /me/editors) open those tabs (movedToProfile). Pure, so it is tested.
import { markCurrent, type RailGroup } from '$lib/shell/rail';
import { sectionHref } from '$lib/profile/rows';

export const SETTINGS = '/me/settings';

/** Where /me/settings itself opens. */
export const SETTINGS_HOME = `${SETTINGS}/appearance`;

const ROWS = [
  { href: `${SETTINGS}/appearance`, label: 'Appearance', icon: 'sun' },
  { href: `${SETTINGS}/notifications`, label: 'Notifications', icon: 'bell' },
  { href: `${SETTINGS}/hidden`, label: 'Hidden', icon: 'eye-off' },
  { href: `${SETTINGS}/billing`, label: 'Billing', icon: 'shopping-bag' },
] as const;

const under = (path: string, base: string) => path === base || path.startsWith(`${base}/`);

/** The old addresses of what is now on your profile, and the tab each opens. */
const MOVED: { base: string; tab: 'account' | 'editors' }[] = [
  { base: `${SETTINGS}/account`, tab: 'account' },
  { base: '/me/account', tab: 'account' },
  { base: '/me/editors', tab: 'editors' },
];

/** The profile tab an old Settings address opens (`/users/<you>?s=account`, the hash kept), or null. */
export function movedToProfile(path: string, me: string, hash = ''): string | null {
  const hit = MOVED.find((m) => under(path, m.base));
  return hit ? `${sectionHref(me, hit.tab)}${hash}` : null;
}

/** Whether a path is one of the Settings pages (their rail, not the profile's). */
export function isSettingsPath(path: string): boolean {
  return under(path, SETTINGS) && !MOVED.some((m) => under(path, m.base));
}

/** The Settings rail, the current page marked (/me/settings itself is Appearance). */
export function settingsRail(path: string): RailGroup[] {
  const at = path === SETTINGS ? SETTINGS_HOME : path;
  return markCurrent([{ items: ROWS.map((r) => ({ ...r })) }], at);
}

/**
 * Which connection a name comes from: the one picked, else Twitch first (the API's order). Only
 * offered when more than one is linked.
 */
export function canChooseName(identities: readonly { platform: string }[]): boolean {
  return identities.length > 1;
}

/** Whether a connection may be removed: never the last one (the API refuses it too). */
export function canDisconnect(identities: readonly { platform: string }[]): boolean {
  return identities.length > 1;
}

// Appearance → Reduced motion, per device: "system" follows the OS, "reduce" turns motion down
// everywhere, "full" keeps it on even when the OS asks for less. app.html applies it before the
// first paint: <html data-motion="reduce" | "full"> (none for System; the front page's hero reads
// the same attribute), and a prefers-reduced-motion answer from matchMedia that follows it. The
// global CSS rules honour both values; a few component rules still follow the OS query.
export type MotionPref = 'system' | 'reduce' | 'full';
export const MOTION_KEY = 'cyexMotion';

export function readMotion(raw: string | null | undefined): MotionPref {
  return raw === 'reduce' || raw === 'full' ? raw : 'system';
}
