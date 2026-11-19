/** Resolves after the load event and an idle moment, so the page's own work and its largest paint go first. */
export function settled(timeout = 2000): Promise<void> {
  return new Promise((done) => {
    const idle = () => ('requestIdleCallback' in window ? requestIdleCallback(() => done(), { timeout }) : setTimeout(done, 200));
    if (document.readyState === 'complete') idle();
    else addEventListener('load', idle, { once: true });
  });
}
