// Short notices for the signed-in pages (cyex-app web-tool toasts): notices bottom right,
// errors bottom left. A repeat bumps a count instead of stacking; at most three passive ones of each.
// A notice with a live action (an Undo, a Sign in link) is never pushed out to make room.
export type Toast = { id: number; text: string; error: boolean; count: number; action?: { label: string; run?: () => void; /** A link instead of a button; a function is read when it is clicked, so it follows the page. */ href?: string | (() => string) };
  /** How long a toast with an Undo waits, for its countdown line. */ ms?: number };

import { pausableTimer, type PausableTimer } from './pausable-timer';

export const toasts = $state<Toast[]>([]);
/** The notice that says the session ended (see `announceSignedOut`). */
export const SIGNED_OUT_TEXT = 'You’re signed out.';
let next = 1;
/** The auto-dismiss timers of passive toasts, held while the pointer or focus is on one. */
const timers = new Map<number, PausableTimer>();

/** Holds a toast's auto-dismiss while it is hovered or focused. A toast with a live Undo (an action and a deadline) is never held: its send time must match its countdown line. */
export function pauseToast(id: number) {
  if (toasts.find((t) => t.id === id)?.ms) return;
  timers.get(id)?.pause();
}

/** Lets a held toast run out the time it had left. */
export function resumeToast(id: number) {
  timers.get(id)?.resume();
}

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
  timers.get(id)?.cancel();
  timers.delete(id);
}

/**
 * Shows a notice and returns its id. `action` adds a button to it (an Undo); `ms` is how long it
 * stays (`sticky` keeps it until it is dismissed). A toast with an action never merges with a repeat.
 */
export function notify(text: string, error = false, opts: { action?: Toast['action']; ms?: number; sticky?: boolean } = {}): number {
  const same = opts.action ? undefined : toasts.find((t) => t.text === text && t.error === error && !t.action);
  if (same) {
    same.count += 1;
    return same.id;
  }
  const toast: Toast = { id: next++, text, error, count: 1, action: opts.action, ms: opts.action ? opts.ms : undefined };
  toasts.push(toast);
  const passive = toasts.filter((t) => t.error === error && !t.action);
  if (passive.length > 3) dismiss(passive[0].id);
  if (!opts.sticky) timers.set(toast.id, pausableTimer(() => dismiss(toast.id), opts.ms ?? (error ? 9000 : 3500)));
  return toast.id;
}

/** Shows an error's message, for `catch` blocks. */
export function fail(e: unknown) {
  // The signed-out notice already says it (once, with a way back in); a second red line would only repeat it.
  if ((e as { code?: unknown } | null)?.code === 'signed_out' && toasts.some((t) => t.text === SIGNED_OUT_TEXT)) return;
  notify(e instanceof Error ? e.message : 'That didn’t work.', true);
}
