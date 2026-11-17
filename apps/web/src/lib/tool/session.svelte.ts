// Who is signed in. Only the browser has the session cookie, so this loads from the browser:
// the root layout (for the site nav) and the tool layout both call loadMe(), and share one
// request. Pages that change the account call loadMe({ fresh: true }).
import { gql } from '$lib/api';
import { ME } from '$lib/queries';
import type { MeQuery } from '$lib/gql/graphql';

export type Me = MeQuery['me'];

export const session = $state<{ me: Me | null }>({ me: null });

let pending: Promise<Me> | null = null;

/**
 * The signed-in account. Every call shares the first request (in flight or done) unless
 * `fresh` asks for a new one. A failed request isn't kept, so the next call tries again.
 */
export function loadMe({ fresh = false }: { fresh?: boolean } = {}): Promise<Me> {
  if (pending && !fresh) return pending;
  const mine: Promise<Me> = gql(ME, {}).then(({ me }) => {
    if (pending === mine) session.me = me;
    return me;
  });
  pending = mine;
  mine.catch(() => {
    if (pending === mine) pending = null;
  });
  return mine;
}

/** Forgets the account (after signing out), so the next loadMe() asks again. */
export function forgetMe() {
  pending = null;
  session.me = null;
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
];

/** Whether the account may open any part of the admin pages. */
export function isAdmin(me: Pick<NonNullable<Me>, 'permissions'> | null | undefined): boolean {
  return me?.permissions.some((p) => ADMIN_PERMISSIONS.includes(p)) ?? false;
}
