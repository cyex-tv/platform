// Media queries, once. `media(query).matches` is reactive (read it in a $derived, a $effect or the
// template) and every query shares one listener however many components read it. It is false on
// the server and until the first mount, so hydration never disagrees with the server's page.
// Read outside anything reactive (a handler, a transition) it gives the answer right now; `now(query)` says so.
import { createSubscriber } from 'svelte/reactivity';

/** The widths the site switches layouts at, named once. */
export const PHONE = '(max-width: 720px)'; // header search, popovers become sheets
export const NARROW = '(max-width: 899.98px)'; // the side rail becomes a strip
export const NO_PANEL = '(max-width: 1099.98px)'; // no right-hand panel: it opens as a sheet or on top
export const PHONE_TABS = '(max-width: 520px)'; // the review queue's short tab names
export const PANEL = '(min-width: 1100px)'; // a right-hand panel beside the list
export const REDUCED_MOTION = '(prefers-reduced-motion: reduce)';

const available = () => typeof matchMedia === 'function';

class Query {
  #value = $state(false);
  #ready = $state(false);
  #subscribe: () => void;

  constructor(readonly query: string) {
    this.#subscribe = createSubscriber((update) => {
      const list = matchMedia(query);
      this.#value = list.matches;
      this.#ready = true;
      const on = () => {
        this.#value = list.matches;
        update();
      };
      list.addEventListener('change', on);
      return () => {
        list.removeEventListener('change', on);
        this.#ready = false;
      };
    });
  }

  /**
   * Reactive where something is tracking it (a $derived, a $effect, the template): re-reads when
   * the answer changes, and is false until mounted. Anywhere else (a handler) it is the answer now.
   */
  get matches(): boolean {
    if (!available()) return false;
    if (!$effect.tracking()) return matchMedia(this.query).matches;
    this.#subscribe();
    return this.#value;
  }

  /** Whether the answer is known yet: false on the server and before the first mount. */
  get ready(): boolean {
    if (!available() || !$effect.tracking()) return false;
    this.#subscribe();
    return this.#ready;
  }
}

const queries = new Map<string, Query>();

export function media(query: string): Query {
  let q = queries.get(query);
  if (!q) queries.set(query, (q = new Query(query)));
  return q;
}

/** The answer right now, not reactive. */
export function now(query: string): boolean {
  return available() && matchMedia(query).matches;
}

const motionPref = () => (typeof document === 'undefined' ? undefined : document.documentElement.dataset.motion);

/**
 * Whether the reader wants reduced motion (the OS's, or Settings → Appearance, which wins: the
 * `data-motion` attribute on <html> says "reduce" or "full"; a change of the setting fires
 * `cyexmotion`, which app.html turns into `change` on the query). Reactive like `media()`.
 */
export const reducedMotion = {
  get matches(): boolean {
    const os = media(REDUCED_MOTION).matches; // read first, so a reactive reader is subscribed either way
    const pref = motionPref();
    return pref === 'reduce' ? true : pref === 'full' ? false : os;
  },
};

/** `reducedMotion.matches`, not reactive: for transitions and handlers that ask at the moment they run. */
export function stillNow(): boolean {
  const pref = motionPref();
  return pref === 'reduce' ? true : pref === 'full' ? false : now(REDUCED_MOTION);
}
