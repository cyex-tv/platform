// Shared motion for Svelte transitions. Every helper reads prefers-reduced-motion when it runs
// (cyex-design: read the query at the moment you animate), and returns no motion under it.
import { cubicOut } from 'svelte/easing';
import type { TransitionConfig } from 'svelte/transition';

/** Whether the reader asked for reduced motion, read now. */
export function still(): boolean {
  return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** `ms`, or 0 under reduced motion: for duration params of fly, fade, flip and friends. */
export function dur(ms: number): number {
  return still() ? 0 : ms;
}

export type PopParams = { duration?: number; delay?: number; y?: number; from?: number };

/**
 * A popover's entrance and exit: opacity with a small scale (from `from`, default .97) and an
 * optional `y` offset in px, cubic ease-out. Uses the `scale` and `translate` properties, so a
 * node's own `transform` is left alone; set `transform-origin` on the node to aim it.
 *   <div in:pop out:pop={{ duration: 100 }}>…</div>
 */
export function pop(_node: Element, { duration = 150, delay = 0, y = 0, from = 0.97 }: PopParams = {}): TransitionConfig {
  if (still()) return { duration: 0 };
  return {
    delay,
    duration,
    easing: cubicOut,
    css: (t, u) => `opacity: ${t}; scale: ${from + (1 - from) * t}; translate: 0 ${u * y}px;`,
  };
}
