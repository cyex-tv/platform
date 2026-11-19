// The cyex.tv logo's geometry, shared by the static SVG and the 3D scene so the two line up
// exactly when one fades into the other. The 32-unit grid of the cyex mark: three open bars,
// the middle one offset to the right and lit. `stroke` is the bars' wall.

export type Bar = { x: number; y: number; w: number; h: number; lit: boolean };

export const GRID = 32;
export const STROKE = 1.3;

export const BARS: readonly Bar[] = [
  { x: 3, y: 5, w: 15, h: 6, lit: false },
  { x: 10, y: 13, w: 18, h: 6, lit: true },
  { x: 3, y: 21, w: 15, h: 6, lit: false },
];

/** The logo's centre on the grid (the 3D scene turns around it). */
export const CENTER = { x: 15.5, y: 16 };
