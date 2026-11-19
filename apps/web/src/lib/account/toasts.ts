// Which notifications pop up as toasts: unread, not shown or seen before, and newer than the
// page (less a grace period). Newest first, as the list comes. Pure, so it is tested.
type Item = { id: string; read: boolean; createdAt: string };

export function fresh<T extends Item>(items: readonly T[], seen: ReadonlySet<string>, since: number): T[] {
  return items.filter((n) => !n.read && !seen.has(n.id) && Date.parse(n.createdAt) >= since);
}
