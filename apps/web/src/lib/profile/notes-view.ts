// Your profile → Notifications (?s=notifications, owner only; it was /me/notifications until
// 2026/11/22): what the list and the panel both read from the address. `?f=` is the filter (All,
// Unread or one group of kinds), `?n=` the picked notification. Pure, so it is tested.
import type { Note } from '$lib/account/inbox.svelte';
import { filterChips, groupOf } from '$lib/account/prefs';

export type NotesView = {
  /** The filter in use: a filter whose last notification went away falls back to All. */
  active: string;
  options: { value: string; label: string }[];
  items: Note[];
  picked: Note | undefined;
};

export function notesView(all: readonly Note[], q: URLSearchParams): NotesView {
  const filter = q.get('f') ?? 'all';
  const chips = filterChips(all.map((n) => n.kind));
  const active = filter === 'unread' || chips.some((c) => c.id === filter) ? filter : 'all';
  const items = active === 'all' ? [...all] : active === 'unread' ? all.filter((n) => !n.read) : all.filter((n) => groupOf(n.kind) === active);
  const pickedId = q.get('n');
  return {
    active,
    options: [
      { value: 'all', label: 'All' },
      { value: 'unread', label: 'Unread' },
      ...chips.filter((c) => c.id !== 'all').map((c) => ({ value: c.id, label: c.label })),
    ],
    items,
    picked: pickedId ? all.find((n) => n.id === pickedId) : undefined,
  };
}
