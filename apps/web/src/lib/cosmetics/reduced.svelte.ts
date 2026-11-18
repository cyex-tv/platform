// Whether the reader asked for reduced motion, kept current for image paints and badges: they
// switch to their still first frame while it is on. One listener for the whole page.
export const reduced = $state({ on: false });

let watching = false;

/** Starts following the setting (call from an effect; the browser only). */
export function watchReduced() {
  if (watching || typeof matchMedia !== 'function') return;
  watching = true;
  const mq = matchMedia('(prefers-reduced-motion: reduce)');
  reduced.on = mq.matches;
  mq.addEventListener('change', () => (reduced.on = mq.matches));
}
