// The cyex deck picture's four panes: an exact 2 x 2 grid (same width, same height, same gap),
// right of the 31px sidebar column. The lower right one is the lit pane (the one playing).
const LEFT = 38;
const TOP = 16;
const GAP = 4;
const W = 30;
const H = 22;

export type Pane = { id: string; x: number; y: number; w: number; h: number; lit: boolean };

export const DECK_PANES: readonly Pane[] = [
  { id: 'a', x: LEFT, y: TOP, w: W, h: H, lit: false },
  { id: 'b', x: LEFT + W + GAP, y: TOP, w: W, h: H, lit: false },
  { id: 'c', x: LEFT, y: TOP + H + GAP, w: W, h: H, lit: false },
  { id: 'd', x: LEFT + W + GAP, y: TOP + H + GAP, w: W, h: H, lit: true },
];
