/** A one-shot timer that can be held and resumed with the time it had left (a toast while the pointer or focus is on it). */
export type PausableTimer = { pause(): void; resume(): void; cancel(): void };

export function pausableTimer(run: () => void, ms: number): PausableTimer {
  let left = ms;
  let started = Date.now();
  let id: ReturnType<typeof setTimeout> | undefined = setTimeout(run, left);
  return {
    pause() {
      if (id === undefined) return;
      clearTimeout(id);
      id = undefined;
      left = Math.max(0, left - (Date.now() - started));
    },
    resume() {
      if (id !== undefined) return;
      started = Date.now();
      id = setTimeout(run, left);
    },
    cancel() {
      clearTimeout(id);
      id = undefined;
      left = 0;
    },
  };
}

export type HoldReason = 'pointer' | 'focus';

/**
 * Holds a timer for as long as any reason is active: the pointer on a toast and focus inside it
 * are separate, so the pointer leaving while a button in it still has focus does not restart it.
 */
export function holds(timer: Pick<PausableTimer, 'pause' | 'resume'>) {
  const on = new Set<HoldReason>();
  return (reason: HoldReason, held: boolean) => {
    if (held) on.add(reason);
    else on.delete(reason);
    if (on.size) timer.pause();
    else timer.resume();
  };
}
