// The admin pickers' people (UserFilter): who to offer before anything is typed, and a pasted
// account ID or admin page link read as an ID.
import type { fetchWith as FetchWith } from '$lib/api';
import { gql } from '$lib/api';
import type { Person } from '$lib/components/user-filter';
import { ENTITLEMENT_TOP_HOLDERS } from '$lib/queries';
import { looksLikeId } from './audit';

/** The 5 accounts holding the most badges, paints and roles (entitlements.view or cosmetics.grant). */
export async function topHolders(signal: AbortSignal, withSignal: typeof FetchWith): Promise<Person[]> {
  return (await gql(ENTITLEMENT_TOP_HOLDERS, { limit: 5 }, withSignal(signal))).entitlementTopHolders;
}

/** An account ID, or the last part of an admin link (`/admin/users/<id>`), as an ID; else null. */
export function pastedAccount(text: string): string | null {
  const last = text.trim().split('/').filter(Boolean).pop() ?? '';
  return looksLikeId(last) ? last.toUpperCase() : null;
}
