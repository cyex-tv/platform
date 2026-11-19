// Reveal once on scroll (cyex-web): a row below the fold waits faded and a little low, and
// settles in when a fifth of it comes on screen. Rows already on screen, and every row under
// reduced motion (the system's or the site's), are left alone. Without script nothing is hidden.
import { motionReduced } from './capable';

export function reveal(node: HTMLElement) {
  if (typeof IntersectionObserver !== 'function' || motionReduced()) return;
  if (node.getBoundingClientRect().top < innerHeight) return;
  node.dataset.reveal = 'wait';
  const io = new IntersectionObserver(
    ([e]) => {
      if (!e.isIntersecting) return;
      node.dataset.reveal = 'done';
      io.disconnect();
    },
    { threshold: 0.2 },
  );
  io.observe(node);
  return { destroy: () => io.disconnect() };
}
