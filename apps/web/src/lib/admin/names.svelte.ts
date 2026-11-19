// Account names for the admin lists (an actor id, a reporter id): looked up once each, shown as
// the id until they arrive, and kept for the session.
import { gql } from '$lib/api';
import { ACCOUNT } from '$lib/queries';

type Who = { name: string; avatar: string | null } | null;
const known = $state<Record<string, Who | undefined>>({});
const asked = new Set<string>();

/** Starts looking the ids up; `who(id)` then reads them reactively. */
export function lookUp(ids: Iterable<string | null | undefined>) {
  for (const id of ids) {
    if (!id || asked.has(id)) continue;
    asked.add(id);
    gql(ACCOUNT, { id })
      .then((r) => (known[id] = r.account ? { name: r.account.displayName, avatar: r.account.avatar ?? null } : null))
      .catch(() => asked.delete(id));
  }
}

/** The name once known, `null` for an unknown or blocked account, `undefined` while loading. */
export const who = (id: string): Who | undefined => known[id];
