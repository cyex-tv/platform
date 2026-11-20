// Short notices for the signed-in pages (cyex-app web-tool toasts): notices bottom right,
// errors bottom left. A repeat bumps a count instead of stacking; at most three of each.
export type Toast = { id: number; text: string; error: boolean; count: number; action?: { label: string; run: () => void };
  /** How long a toast with an Undo waits, for its countdown line. */ ms?: number };

export const toasts = $state<Toast[]>([]);
let next = 1;

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

/**
 * Shows a notice and returns its id. `action` adds a button to it (an Undo); `ms` is how long it
 * stays. A toast with an action never merges with a repeat.
 */
export function notify(text: string, error = false, opts: { action?: Toast['action']; ms?: number } = {}): number {
  const same = opts.action ? undefined : toasts.find((t) => t.text === text && t.error === error && !t.action);
  if (same) {
    same.count += 1;
    return same.id;
  }
  const toast: Toast = { id: next++, text, error, count: 1, action: opts.action, ms: opts.action ? opts.ms : undefined };
  toasts.push(toast);
  const mine = toasts.filter((t) => t.error === error);
  if (mine.length > 3) dismiss(mine[0].id);
  setTimeout(() => dismiss(toast.id), opts.ms ?? (error ? 9000 : 3500));
  return toast.id;
}

/** Shows an error's message, for `catch` blocks. */
export function fail(e: unknown) {
  notify(e instanceof Error ? e.message : 'That didn’t work.', true);
}
