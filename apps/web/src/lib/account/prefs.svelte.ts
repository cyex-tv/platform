// Your notification preferences, shared by Settings → Notifications and the pop-ups (which skip
// the kinds whose pop-up is off). Loaded once per page load when asked; a save sends only what
// changed and keeps the API's answer. A failed save leaves things as they were.
import { gql } from '$lib/api';
import { NOTIFICATION_PREFS, SET_NOTIFICATION_PREFS } from '$lib/queries';
import type { Pref } from './prefs';

export const notePrefs = $state<{ items: Pref[] | null; error: boolean; saving: string | null }>({ items: null, error: false, saving: null });

let asked: Promise<void> | null = null;
// Which load is the newest; an older answer, or one that lands after signing out, is dropped.
let seq = 0;

/** Loads them (once, unless `fresh`). */
export function loadPrefs(fresh = false): Promise<void> {
  if (asked && !fresh) return asked;
  notePrefs.error = false;
  const n = ++seq;
  const mine = gql(NOTIFICATION_PREFS, {})
    .then((r) => {
      if (n === seq) notePrefs.items = r.myNotificationPrefs;
    })
    .catch(() => {
      if (n !== seq) return;
      notePrefs.error = true;
      if (asked === mine) asked = null;
    });
  asked = mine;
  return mine;
}

/** The kinds whose pop-up is off (empty until loaded: then everything pops up, as before). */
export function quietKinds(): Set<string> {
  return new Set((notePrefs.items ?? []).filter((p) => !p.toast).map((p) => p.kind));
}

/** Changes one kind. False when there was nothing to change; a failure throws. */
export async function savePref(kind: string, change: { notify?: boolean; toast?: boolean }): Promise<boolean> {
  const cur = notePrefs.items?.find((p) => p.kind === kind);
  if (!cur || notePrefs.saving) return false;
  const next = { kind, notify: change.notify ?? cur.notify, toast: change.toast ?? cur.toast };
  notePrefs.saving = kind;
  try {
    notePrefs.items = (await gql(SET_NOTIFICATION_PREFS, { prefs: [next] })).setNotificationPrefs;
    return true;
  } finally {
    notePrefs.saving = null;
  }
}

/** Forget them (signing out). */
export function forgetPrefs() {
  seq++;
  asked = null;
  notePrefs.error = false;
  notePrefs.items = null;
}
