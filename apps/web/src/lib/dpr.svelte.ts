// The screen's device pixel ratio, for pictures that pick their file in script instead of with a
// srcset (see `pictureFor` in $lib/emote). 0 until the page has mounted: the server render and
// the hydrating client both offer a srcset then, so the first paint is sharp and hydration
// matches; from then on the file is picked here. Follows page zoom and moves to another screen.
// `epoch` counts settled window resizes (200 ms after the last), for what is measured once and
// must be measured again when the layout changes (image paints, see PaintedName).
export const display = $state({ dpr: 0, epoch: 0 });

let watching = false;

/** Starts following the ratio (the root layout calls it once mounted; the browser only). */
export function watchDpr() {
  if (watching || typeof window === 'undefined') return;
  watching = true;
  const read = () => (display.dpr = window.devicePixelRatio || 1);
  read();
  let settle: ReturnType<typeof setTimeout> | undefined;
  addEventListener('resize', () => {
    read();
    clearTimeout(settle);
    settle = setTimeout(() => (display.epoch += 1), 200);
  });
}
