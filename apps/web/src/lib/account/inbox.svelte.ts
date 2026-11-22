// Your notifications, shared by the bell in the header, the phone menu and your profile's Notifications tab:
// how many are unread (cheap, asked often) and the list itself (latest 100, asked when opened).
// Reads and marks go through the API; a failed mark leaves things as they were.
import { gql } from '$lib/api';
import { MARK_READ, MY_NOTIFICATIONS, UNREAD_COUNT } from '$lib/queries';
import { session } from '$lib/tool/session.svelte';

export type Note = { id: string; kind: string; payload: string; createdAt: string; read: boolean };

export const inbox = $state<{ count: number; known: boolean; items: Note[] | null; loading: boolean; error: string | null; marking: boolean }>({
  count: 0,
  /** The count came from the API at least once (until then 0 means "not asked yet"). */
  known: false,
  items: null,
  loading: false,
  error: null,
  /** "Mark all read" is on its way: the button waits, a second press sends nothing. */
  marking: false,
});

/** The unread count. A failure keeps the number it had: a bell must never complain. */
export async function refreshCount(): Promise<void> {
  try {
    inbox.count = (await gql(UNREAD_COUNT, {})).unreadNotificationCount;
    inbox.known = true;
  } catch {
    /* keep the last number */
  }
}

/** The list, newest first. The old list stays on screen while it loads again. */
export async function loadInbox(): Promise<void> {
  inbox.loading = true;
  inbox.error = null;
  try {
    const list = (await gql(MY_NOTIFICATIONS, {})).myNotifications;
    inbox.items = list;
    inbox.count = list.filter((n) => !n.read).length;
    inbox.known = true;
  } catch (e) {
    inbox.error = e instanceof Error ? e.message : 'That didn’t load.';
  } finally {
    inbox.loading = false;
  }
}

/** Marks some (up to 100) or all of them read; true when it worked. */
export async function markRead(ids?: string[]): Promise<boolean> {
  if (!ids) {
    if (inbox.marking) return false;
    inbox.marking = true;
  }
  try {
    await gql(MARK_READ, { ids: ids ?? null });
  } catch {
    return false;
  } finally {
    if (!ids) inbox.marking = false;
  }
  if (inbox.items) {
    const set = ids ? new Set(ids) : null;
    inbox.items = inbox.items.map((n) => (!set || set.has(n.id) ? { ...n, read: true } : n));
    inbox.count = inbox.items.filter((n) => !n.read).length;
  } else {
    await refreshCount();
  }
  return true;
}

/** Keeps the count fresh while signed in: now, every minute, and when the tab comes back. */
export function watchInbox(): () => void {
  void refreshCount();
  const tick = () => {
    if (document.visibilityState === 'visible' && session.me) void refreshCount();
  };
  const timer = setInterval(tick, 60_000);
  document.addEventListener('visibilitychange', tick);
  return () => {
    clearInterval(timer);
    document.removeEventListener('visibilitychange', tick);
  };
}

/** Forgets everything (signing out or deleting the account). */
export function forgetInbox(): void {
  inbox.count = 0;
  inbox.known = false;
  inbox.items = null;
  inbox.error = null;
}
