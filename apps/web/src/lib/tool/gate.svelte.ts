// One action at a time, with a short rest after it: a second click while the first is on its way
// (or just answered) sends nothing. Used where a repeat costs something (revoking a session, the
// data export, which the API limits to five an hour).
export class Gate {
  busy = $state(false);
  readonly #rest: number;

  constructor(rest = 0) {
    this.#rest = rest;
  }

  /** Runs `fn` unless one is still running or resting; returns its result, or undefined when refused. */
  async run<T>(fn: () => Promise<T>): Promise<T | undefined> {
    if (this.busy) return undefined;
    this.busy = true;
    try {
      return await fn();
    } finally {
      if (this.#rest > 0) setTimeout(() => (this.busy = false), this.#rest);
      else this.busy = false;
    }
  }
}
