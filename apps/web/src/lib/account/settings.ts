// Settings (opened from the account menu, like 7TV's): one rail, Account / Appearance / Hidden /
// Editors / Billing. Editors is the existing /me/editors page, listed in this rail. The old /me/account
// address goes to Settings → Account. Pure, so it is tested.
import { markCurrent, type RailGroup } from '$lib/shell/rail';

export const SETTINGS = '/me/settings';

const ROWS = [
  { href: `${SETTINGS}/account`, label: 'Account', icon: 'user' },
  { href: `${SETTINGS}/appearance`, label: 'Appearance', icon: 'sun' },
  { href: `${SETTINGS}/hidden`, label: 'Hidden', icon: 'eye-off' },
  { href: '/me/editors', label: 'Editors', icon: 'circle-user' },
  { href: `${SETTINGS}/billing`, label: 'Billing', icon: 'shopping-bag' },
] as const;

const under = (path: string, base: string) => path === base || path.startsWith(`${base}/`);

/** Whether a path is one of the Settings pages (their rail, not the profile's). */
export function isSettingsPath(path: string): boolean {
  return under(path, SETTINGS) || under(path, '/me/editors') || under(path, '/me/account');
}

/** The Settings rail, the current page marked (/me/settings itself is Account). */
export function settingsRail(path: string): RailGroup[] {
  const at = path === SETTINGS || under(path, '/me/account') ? `${SETTINGS}/account` : path;
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
