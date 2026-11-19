// Bars fixed or stuck to the bottom of the screen (the /search select bar, the emote page's phone
// action bar) register here with `use:bottomBar`, so the toast stacks sit above them instead of
// covering their buttons.

const bars = new Set<HTMLElement>();
let version = $state(0);

/** Svelte action: marks a bottom bar for as long as the node is mounted. */
export function bottomBar(node: HTMLElement) {
  bars.add(node);
  version++;
  return {
    destroy() {
      bars.delete(node);
      version++;
    },
  };
}

/** Changes whenever a bar mounts or unmounts; read it in an effect to re-measure. */
export function barsVersion(): number {
  return version;
}

export function barNodes(): HTMLElement[] {
  return [...bars];
}

type Box = { top: number; bottom: number; height: number };

/**
 * How far above the viewport's bottom edge the toasts must start so they clear every bar that
 * reaches into the toast zone (the bottom `zone` px). A hidden bar (height 0), one off screen, or
 * one sitting higher up the page doesn't lift them.
 */
export function lift(boxes: readonly Box[], viewportHeight: number, zone = 96): number {
  let out = 0;
  for (const b of boxes) {
    if (b.height <= 0 || b.top >= viewportHeight || b.bottom < viewportHeight - zone) continue;
    out = Math.max(out, viewportHeight - b.top);
  }
  return Math.max(0, Math.round(out));
}
