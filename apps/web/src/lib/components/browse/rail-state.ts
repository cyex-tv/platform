// Which groups of the browse page's set rail are folded shut, remembered per account on this
// device. Until you fold or unfold one yourself, a small rail is all open; a big one (many sets
// or many accounts you edit for) opens only your own sets and the group of the set you add to,
// so hundreds of sets never arrive as one long list. Pure apart from storage, so it is tested.

/** More sets than this, or more groups than RAIL_GROUPS_OPEN, and the rail starts folded. */
export const RAIL_SETS_OPEN = 24;
export const RAIL_GROUPS_OPEN = 4;

export const railKey = (account: string) => `cyex.browse.rail.${account}`;

/** The groups to fold when nothing is remembered. */
export function defaultShut(groups: readonly { key: string; sets: readonly unknown[] }[], targetGroup: string | null): Set<string> {
  const sets = groups.reduce((n, g) => n + g.sets.length, 0);
  if (sets <= RAIL_SETS_OPEN && groups.length <= RAIL_GROUPS_OPEN) return new Set();
  return new Set(groups.map((g) => g.key).filter((k) => k !== 'mine' && k !== targetGroup));
}

/** The remembered folded groups, or null when nothing is stored (or storage is off). */
export function loadShut(account: string): Set<string> | null {
  try {
    const raw = localStorage.getItem(railKey(account));
    if (raw === null) return null;
    const list = JSON.parse(raw);
    return Array.isArray(list) ? new Set(list.filter((k): k is string => typeof k === 'string')) : null;
  } catch {
    return null;
  }
}

export function saveShut(account: string, shut: Iterable<string>) {
  try {
    localStorage.setItem(railKey(account), JSON.stringify([...shut]));
  } catch {
    /* not remembered */
  }
}
