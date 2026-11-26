// Every set is public (ADR 0017, amendment 2026/11/22): nobody picks who can open a set any more.
// Staff can still take one off the listings (moderation); such a set carries a small mark on its
// card. Listed sets (every other set) carry none.
import type { IconName } from '$lib/components/Icon.svelte';
import type { SetVisibility } from '$lib/gql/graphql';

/** The small mark on a set card staff took off the listings; listed sets carry none. */
export function setMark(v: SetVisibility | null | undefined): { icon: IconName; label: string } | null {
  if (v === 'UNLISTED') return { icon: 'link', label: 'Unlisted by staff' };
  if (v === 'PRIVATE') return { icon: 'lock', label: 'Hidden by staff' };
  return null;
}