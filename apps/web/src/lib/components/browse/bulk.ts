// Adding many emotes to one set from the browse page: what goes in, what is skipped and why,
// and the sentence that sums it up. Pure, so it is unit tested.

export type BulkPlan = {
  /** Ids to add now, in the order picked. */
  add: string[];
  /** Ids the set already holds. */
  already: string[];
  /** Ids that don't fit: the set's capacity runs out. */
  overflow: string[];
};

/** Splits the picked ids by what the set holds and how many free places it has. */
export function plan(ids: readonly string[], inSet: ReadonlySet<string>, free: number): BulkPlan {
  const already = ids.filter((id) => inSet.has(id));
  const fresh = ids.filter((id) => !inSet.has(id));
  const room = Math.max(0, free);
  return { add: fresh.slice(0, room), already, overflow: fresh.slice(room) };
}

/**
 * What the Add button says about a plan: `add` (something will go in), `already` (every pick is in
 * the set), `full` (nothing fits), `none` (nothing picked yet).
 */
export function addState(picked: number, p: BulkPlan | null): 'none' | 'add' | 'already' | 'full' {
  if (!picked || !p) return 'none';
  if (p.add.length) return 'add';
  return p.already.length >= picked ? 'already' : 'full';
}

export type BulkResult = { added: number; already: number; overflow: number; taken: number; failed: number };

/** "Added 8 to Main. 2 were already there. 1 didn't fit: the set is full." */
export function summary(set: string, r: BulkResult): string {
  const out = [r.added ? `Added ${r.added} to ${set}.` : `Nothing was added to ${set}.`];
  if (r.already) out.push(`${r.already} ${r.already === 1 ? 'was' : 'were'} already there.`);
  if (r.taken) out.push(`${r.taken} ${r.taken === 1 ? 'has' : 'have'} a name the set already uses.`);
  if (r.overflow) out.push(`${r.overflow} didn’t fit: the set is full.`);
  if (r.failed) out.push(`${r.failed} couldn’t be added.`);
  return out.join(' ');
}
