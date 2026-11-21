// A column that sticks under the header only while it fits the window (the admin rail and the
// inspector panel). Taller than the window, it gets `data-tall` and scrolls with the page instead,
// so it never needs a scroll bar of its own (owner rule for the admin pages).

/** Whether a column of `height` px fits under the header (`nav` px) in a window of `room` px. */
export function fits(height: number, room: number, nav: number): boolean {
  return height <= room - nav + 1;
}

export function fitSticky(node: HTMLElement) {
  let frame = 0;
  function check() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      const nav = parseFloat(getComputedStyle(node).getPropertyValue('--nav-h')) || 56;
      // Measured without the minimum height the full-height look gives it.
      const inner = node.scrollHeight;
      if (fits(inner, innerHeight, nav)) delete node.dataset.tall;
      else node.dataset.tall = '';
    });
  }
  check();
  const ro = new ResizeObserver(check);
  ro.observe(node);
  for (const c of node.children) ro.observe(c);
  const mo = new MutationObserver(check);
  mo.observe(node, { subtree: true, childList: true, characterData: true });
  addEventListener('resize', check, { passive: true });
  return {
    destroy() {
      cancelAnimationFrame(frame);
      ro.disconnect();
      mo.disconnect();
      removeEventListener('resize', check);
    },
  };
}
