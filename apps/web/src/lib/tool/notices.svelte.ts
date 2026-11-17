// Short notices for the signed-in pages (cyex-app web-tool toasts): notices bottom right,
// errors bottom left. A repeat bumps a count instead of stacking; at most three of each.
export type Toast = { id: number; text: string; error: boolean; count: number };

export const toasts = $state<Toast[]>([]);
let next = 1;

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}

export function notify(text: string, error = false) {
  const same = toasts.find((t) => t.text === text && t.error === error);
  if (same) {
    same.count += 1;
    return;
  }
  const toast = { id: next++, text, error, count: 1 };
  toasts.push(toast);
  const mine = toasts.filter((t) => t.error === error);
  if (mine.length > 3) dismiss(mine[0].id);
  setTimeout(() => dismiss(toast.id), error ? 9000 : 3500);
}

/** Shows an error's message, for `catch` blocks. */
export function fail(e: unknown) {
  notify(e instanceof Error ? e.message : 'That didn’t work.', true);
}
