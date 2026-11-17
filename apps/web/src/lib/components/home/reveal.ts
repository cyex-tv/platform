// Reveal once on scroll (cyex-web): opacity and a 28px rise over --dur-reveal, staggered by
// `step`. Only things still below the fold wait, so nothing on the first screen ever blinks,
// and under reduced motion nothing waits at all. Styles: `[data-reveal]` in the home page.
import type { Action } from 'svelte/action';

export const reveal: Action<HTMLElement, number | undefined> = (node, step = 0) => {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  if (typeof IntersectionObserver === 'undefined') return;
  if (node.getBoundingClientRect().top < innerHeight) return;
  node.style.setProperty('--step', String(step));
  node.dataset.reveal = 'wait';
  const io = new IntersectionObserver(
    ([entry]) => {
      if (!entry.isIntersecting) return;
      node.dataset.reveal = 'in';
      io.disconnect();
    },
    { rootMargin: '0px 0px -8% 0px' },
  );
  io.observe(node);
  return { destroy: () => io.disconnect() };
};
