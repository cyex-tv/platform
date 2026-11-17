// Loading for the signed-in pages: one request at a time per page, the newest one wins, and
// a request that is replaced (the page's inputs changed, or the page closed) is aborted.
import { untrack } from 'svelte';
import { ApiError } from '$lib/api';
import { fail, notify } from './notices.svelte';

export type LoadCtx = { signal: AbortSignal; fetch: typeof fetch };

export type Loader<T> = {
  readonly data: T | null;
  readonly error: string | null;
  readonly loading: boolean;
  /** Asks again. The old data stays on screen while it loads. */
  reload(): Promise<void>;
  /** Replaces the data in place (after a change the page already knows the result of). */
  set(next: T | null): void;
};

/**
 * Runs `fn` now and again whenever state it reads synchronously changes (an id from the URL,
 * say). Pass `ctx.fetch` to `gql` so a replaced request is aborted. Call during component setup.
 */
export function loader<T>(fn: (ctx: LoadCtx) => Promise<T>): Loader<T> {
  let data = $state<T | null>(null) as T | null;
  let error = $state<string | null>(null);
  let loading = $state(true);
  let seq = 0;
  let ctl: AbortController | null = null;

  function start(keep: boolean): Promise<void> {
    ctl?.abort();
    const mine = ++seq;
    const c = (ctl = new AbortController());
    if (!keep) data = null;
    error = null;
    loading = true;
    const f: typeof fetch = (input, init) => fetch(input, { ...init, signal: c.signal });
    let run: Promise<T>;
    try {
      run = fn({ signal: c.signal, fetch: f });
    } catch (e) {
      run = Promise.reject(e);
    }
    return run.then(
      (v) => {
        if (mine !== seq) return;
        data = v;
        loading = false;
      },
      (e) => {
        if (mine !== seq) return;
        error = e instanceof Error && !(e instanceof ApiError && e.code === 'offline' && c.signal.aborted)
          ? e.message
          : 'That didn’t load.';
        loading = false;
      },
    );
  }

  $effect(() => {
    start(false);
    return () => {
      seq++;
      ctl?.abort();
    };
  });

  return {
    get data() { return data; },
    get error() { return error; },
    get loading() { return loading; },
    reload: () => untrack(() => start(true)),
    set(next) { data = next; },
  };
}

/**
 * Runs a change: ignores a second call while the first runs, shows a failure as a toast,
 * and says `ok` on success. Returns whether it worked.
 */
export function runner() {
  let busy = $state(false);
  return {
    get busy() { return busy; },
    async run(fn: () => Promise<unknown>, ok?: string): Promise<boolean> {
      if (busy) return false;
      busy = true;
      try {
        await fn();
        if (ok) notify(ok);
        return true;
      } catch (e) {
        fail(e);
        return false;
      } finally {
        busy = false;
      }
    },
  };
}
