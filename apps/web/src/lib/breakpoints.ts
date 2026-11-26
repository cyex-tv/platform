// The widths the website switches layouts at, named once. Every `@media` width in a stylesheet or
// <style> block, and every `matchMedia` / `innerWidth` check in code, is one of these (the list is
// repeated in tokens.css, and breakpoints.test.ts fails when a file uses another width). CSS has no
// custom media, so a stylesheet writes the number: "below W" is `(max-width: W - 0.02px)`, "from W"
// is `(min-width: Wpx)`; the two never both match and never leave a gap.
export const BP = {
  /** Small phones: tabs and rows drop to their shortest form. */
  tiny: 480,
  /** Phones: controls and grids go to one column. */
  small: 640,
  /** Header search and popovers become sheets. */
  phone: 720,
  /** The side rail becomes a strip. */
  narrow: 900,
  /** A right-hand panel fits beside the list; below it opens as a sheet or on top. */
  panel: 1100,
  /** Extra columns: the paint studio's three-column layout. */
  wide: 1600,
} as const;

export type Breakpoint = keyof typeof BP;

/** Below `w`: `(max-width: w - 0.02px)`. */
export const below = (w: number) => `(max-width: ${+(w - 0.02).toFixed(2)}px)`;
/** From `w` up: `(min-width: wpx)`. */
export const from = (w: number) => `(min-width: ${w}px)`;
