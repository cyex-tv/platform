// Undo and redo for the emote editor, pure (history.test.ts). Every edit is a whole new document
// (the editor's documents share what they don't change), so undo is a step back in a list. A
// drag shows its live state with `preview` and lands as one step with `push`; at most `limit`
// steps are kept, the oldest dropped first.

export type Step<T> = { doc: T; label: string };

export class History<T> {
  #past: Step<T>[] = [];
  #future: Step<T>[] = [];
  #now: T;
  #label = '';
  readonly limit: number;

  constructor(initial: T, limit = 200) {
    this.#now = initial;
    this.limit = limit;
  }

  get now(): T {
    return this.#now;
  }

  /** A new step: what `next` changed is called `label` ("Crop", "Delete 3 frames"). */
  push(next: T, label: string) {
    if (next === this.#now) return;
    this.#past.push({ doc: this.#now, label: this.#label });
    if (this.#past.length > this.limit) this.#past.shift();
    this.#now = next;
    this.#label = label;
    this.#future = [];
  }

  /** Replaces the current state without a step (settings that aren't edits). */
  replace(next: T) {
    this.#now = next;
  }

  get canUndo() {
    return this.#past.length > 0;
  }
  get canRedo() {
    return this.#future.length > 0;
  }
  /** What undo would take back. */
  get undoLabel(): string | null {
    return this.canUndo ? this.#label : null;
  }
  get redoLabel(): string | null {
    return this.#future.at(-1)?.label ?? null;
  }
  /** Steps taken since the start (or the last reset): 0 means nothing is edited. */
  get depth() {
    return this.#past.length;
  }

  undo(): T {
    const prev = this.#past.pop();
    if (!prev) return this.#now;
    this.#future.push({ doc: this.#now, label: this.#label });
    this.#now = prev.doc;
    this.#label = prev.label;
    return this.#now;
  }

  redo(): T {
    const next = this.#future.pop();
    if (!next) return this.#now;
    this.#past.push({ doc: this.#now, label: this.#label });
    this.#now = next.doc;
    this.#label = next.label;
    return this.#now;
  }

  /** Starts over from `doc` with no steps. */
  reset(doc: T) {
    this.#past = [];
    this.#future = [];
    this.#now = doc;
    this.#label = '';
  }
}
