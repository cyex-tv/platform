// Who is signed in. Only the browser has the session cookie, so this loads from the browser:
// the root layout (for the site nav) and the tool layout both call loadMe(), and share one
// request. Pages that change the account call loadMe({ fresh: true }).
import { gql, onSignedOut } from '$lib/api';
import { SIGNED_OUT_TEXT, dismiss, notify, toasts } from './notices.svelte';
import { ME } from '$lib/queries';
import type { MeQuery } from '$lib/gql/graphql';

export type Me = MeQuery['me'];

/**
 * `known`: an answer (signed in or not) has arrived, so the header can show the right slot.
 * `failed`: the last request failed for another reason than being signed out (the API is down),
 * so nobody knows yet whether you are signed in: the header shows a neutral slot, not Sign in.
 */
export const session = $state<{ me: Me | null; known: boolean; failed: boolean; ended: boolean }>({ me: null, known: false, failed: false, ended: false });
// `ended`: the session ended while the page was open. The header and menu go to Sign in, but `me`
// stays, so the page being used keeps its account and nothing on it vanishes.

const signedOut = (e: unknown) => (e as { code?: unknown } | null)?.code === 'signed_out';

let pending: Promise<Me> | null = null;

/**
 * The signed-in account. Every call shares the first request (in flight or done) unless
 * `fresh` asks for a new one. A failed request isn't kept, so the next call tries again.
 */
export function loadMe({ fresh = false }: { fresh?: boolean } = {}): Promise<Me> {
  if (pending && !fresh) return pending;
  const mine: Promise<Me> = gql(ME, {}).then(({ me }) => {
    if (pending === mine) {
      session.me = me;
      session.failed = false;
      session.ended = false;
      if (me) clearSignedOut();
    }
    session.known = true;
    return me;
  });
  pending = mine;
  mine.catch((e) => {
    session.known = true;
    if (pending !== mine) return;
    pending = null;
    session.failed = !signedOut(e);
    if (!session.failed) session.me = null;
  });
  return mine;
}

// The session can end while a page is open (signed out in another tab, expired, ended from the
// sessions list). Whichever call finds out, the person hears it once, in one place: a notice with a
// way back in, and the header stops showing the old account. The call's own page keeps what it had
// (the card or row that failed stays); only visitors who were never signed in are left alone, since
// for them `signed_out` is the expected answer.
let signedOutNotice: number | null = null;

/** The notice's Sign in link goes back to the page the person is on when they click it. */
export const signInBack = () => `/signin?back=${encodeURIComponent(location.pathname + location.search)}`;

export function announceSignedOut() {
  if (!session.me || session.ended) return;
  session.ended = true;
  session.known = true;
  session.failed = false;
  pending = null;
  if (signedOutNotice !== null && toasts.some((t) => t.id === signedOutNotice)) return;
  signedOutNotice = notify(SIGNED_OUT_TEXT, true, { action: { label: 'Sign in', href: signInBack }, sticky: true });
}
/** Takes the notice away (the person signed in again). */
export function clearSignedOut() {
  if (signedOutNotice !== null) dismiss(signedOutNotice);
  signedOutNotice = null;
}
onSignedOut(announceSignedOut);

/** Forgets the account (after signing out), so the next loadMe() asks again. */
export function forgetMe() {
  pending = null;
  session.me = null;
  session.ended = false;
  session.failed = false;
}

export function can(permission: string): boolean {
  return session.me?.permissions.includes(permission) ?? false;
}

/** The permissions that open a part of the admin pages. */
export const ADMIN_PERMISSIONS = [
  'emotes.review',
  'emotes.takedown',
  'roles.manage',
  'users.view',
  'users.restrict',
  'audit.view',
  'global_set.edit',
  'cosmetics.manage',
  'cosmetics.grant',
  'shop.manage',
  'shop.orders',
  'codes.manage',
  'roles.assign',
  'reports.manage',
  'users.restrict_uploads',
  'users.restrict_tickets',
  'users.block',
  'users.sessions',
  'users.notes',
  'entitlements.view',
  'billing.view',
  'subscriptions.manage',
  'emotes.edit_any',
  'emotes.delete_any',
  'editors.manage_any',
  'review.settings',
  'site.announce',
  'emotes.flags',
  'emotes.review_personal',
  'emotes.transfer',
];

/** Whether the account may open any part of the admin pages. */
export function isAdmin(me: Pick<NonNullable<Me>, 'permissions'> | null | undefined): boolean {
  return me?.permissions.some((p) => ADMIN_PERMISSIONS.includes(p)) ?? false;
}
