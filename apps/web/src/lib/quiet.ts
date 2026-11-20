// A load that may fail without stopping the page (a list that fills a menu, a count on a rail).
// `.catch(() => {})` hides that it failed; `quiet` says so in the console while developing, and
// can tell the screen (`onFail`), so a control that stays empty can say why and offer a retry.

/**
 * Waits for `promise`; on failure logs `label` in development and calls `onFail`, and resolves to
 * `undefined` instead of rejecting.
 */
export async function quiet<T>(promise: Promise<T>, label: string, onFail?: (error: unknown) => void): Promise<T | undefined> {
  try {
    return await promise;
  } catch (error) {
    if (import.meta.env.DEV) console.warn(`[quiet] ${label} failed`, error);
    onFail?.(error);
    return undefined;
  }
}
