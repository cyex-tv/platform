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
