// Paging for the emote page's tabs. The pager sits in the tab bar, so a tab hands this up.

/** What the tab bar's pager shows and does for the open tab. */
export type PagerState = { range: string; prev: boolean; next: boolean; busy: boolean; onprev: () => void; onnext: () => void };

/**
 * Cursor pages (the Activity tab): the cursor each page started from, so Previous can go back
 * without the API paging backwards. `starts[0]` is null (the newest rows).
 */
export class CursorTrail {
  private starts: (string | null)[] = [null];

  /** Where page `i` starts, or undefined when it isn't known yet. */
  start(i: number): string | null | undefined {
    return this.starts[i];
  }

  /** Records where the page after `i` starts (its `next`); null ends the trail there. */
  after(i: number, next: string | null) {
    this.starts.length = i + 1;
    if (next !== null) this.starts.push(next);
  }

  /** Whether there is a page after `i`. */
  hasNext(i: number): boolean {
    return this.starts.length > i + 1;
  }
}
