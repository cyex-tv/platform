// The set rail's groups and filter (SetRail.svelte, and the tile menu's "Add to…"): your sets first, then one group per
// account you edit for, each in the order the API gave. A filter field shows once there are more
// than FILTER_FROM sets; every word typed must appear in the set's name, its group's name or one
// of the channels it belongs to (case and accents ignored).
import { clean } from '$lib/format';

export const FILTER_FROM = 8;

export type PickSet = { id: string; name: string; ownerId: string; ownerName: string; mine: boolean; channels: readonly string[] };
export type SetGroup<T extends PickSet> = { key: string; label: string; sets: T[] };

/** Lower case, no accents, no bidi controls: what the filter compares. */
export function fold(s: string): string {
  return clean(s).normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase();
}

/** The groups, in order: yours ("Your sets"), then each account you edit for, by its name. */
export function groupSets<T extends PickSet>(sets: readonly T[]): SetGroup<T>[] {
  const by = new Map<string, SetGroup<T>>();
  for (const s of sets) {
    const key = s.mine ? 'mine' : s.ownerId;
    let g = by.get(key);
    if (!g) by.set(key, (g = { key, label: s.mine ? 'Your sets' : clean(s.ownerName) || 'Shared with you', sets: [] }));
    g.sets.push(s);
  }
  const mine = by.get('mine');
  const others = [...by.values()].filter((g) => g.key !== 'mine').sort((a, b) => a.label.localeCompare(b.label));
  return mine ? [mine, ...others] : others;
}

/** The groups with only the sets the query matches; groups left empty are dropped. */
export function filterGroups<T extends PickSet>(groups: readonly SetGroup<T>[], query: string): SetGroup<T>[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (!words.length) return [...groups];
  const out: SetGroup<T>[] = [];
  for (const g of groups) {
    const sets = g.sets.filter((s) => {
      const hay = fold([s.name, g.label, s.ownerName, ...s.channels].join(' '));
      return words.every((w) => hay.includes(w));
    });
    if (sets.length) out.push({ ...g, sets });
  }
  return out;
}
