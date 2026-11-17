// Who is signed in, for the signed-in pages. Loaded once by the tool layout from the browser
// (only it has the session cookie); pages read it and refresh it after account changes.
import { gql } from '$lib/api';
import { ME } from '$lib/queries';
import type { MeQuery } from '$lib/gql/graphql';

export type Me = MeQuery['me'];

export const session = $state<{ me: Me | null }>({ me: null });

export async function loadMe(): Promise<Me> {
  const { me } = await gql(ME, {});
  session.me = me;
  return me;
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
