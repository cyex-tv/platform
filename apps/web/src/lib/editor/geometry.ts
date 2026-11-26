// The emote editor's shape maths, pure (geometry.test.ts): rectangles, the crop box and its
// handles, fitting and the orientation matrix (rotate and flip as one 2 × 2 integer matrix).
//
// Spaces: the *source* is a decoded frame (sw × sh). The *canvas* is the source turned and
// mirrored (orient); the crop box and every drawing live in canvas space, so cropping or
// resizing later never moves a drawing on the picture. The *output* is the crop scaled by
// (sx, sy).

export type Rect = { x: number; y: number; w: number; h: number };
export type Size = { w: number; h: number };
/** A 2 × 2 matrix as the canvas writes it: x' = a·x + c·y, y' = b·x + d·y (entries −1, 0 or 1). */
export type Mat = readonly [a: number, b: number, c: number, d: number];
/** An affine map as `setTransform(a, b, c, d, e, f)` takes it. */
export type Affine = readonly [a: number, b: number, c: number, d: number, e: number, f: number];

export const IDENTITY: Mat = [1, 0, 0, 1];
/** Turn 90° clockwise (y points down): right goes to bottom. */
export const ROT_CW: Mat = [0, 1, -1, 0];
export const ROT_CCW: Mat = [0, -1, 1, 0];
export const FLIP_H: Mat = [-1, 0, 0, 1];
export const FLIP_V: Mat = [1, 0, 0, -1];

/** `o` after `m`: the matrix that applies m first, then o. */
export function compose(o: Mat, m: Mat): Mat {
  // `+ 0` turns -0 into 0, so equal turns compare equal.
  return [o[0] * m[0] + o[2] * m[1] + 0, o[1] * m[0] + o[3] * m[1] + 0, o[0] * m[2] + o[2] * m[3] + 0, o[1] * m[2] + o[3] * m[3] + 0];
}

/** The size a w × h rectangle has after `m`. */
export function turnedSize(m: Mat, w: number, h: number): Size {
  return { w: Math.abs(m[0]) * w + Math.abs(m[2]) * h, h: Math.abs(m[1]) * w + Math.abs(m[3]) * h };
}

/** `m` with the translation that keeps the turned w × h rectangle at the origin. */
export function placed(m: Mat, w: number, h: number): Affine {
  const e = -(Math.min(0, m[0] * w) + Math.min(0, m[2] * h));
  const f = -(Math.min(0, m[1] * w) + Math.min(0, m[3] * h));
  return [m[0], m[1], m[2], m[3], e + 0, f + 0];
}

export function apply(t: Affine, x: number, y: number): [number, number] {
  return [t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]];
}

/** a after b (b first). */
export function chain(a: Affine, b: Affine): Affine {
  return [
    a[0] * b[0] + a[2] * b[1],
    a[1] * b[0] + a[3] * b[1],
    a[0] * b[2] + a[2] * b[3],
    a[1] * b[2] + a[3] * b[3],
    a[0] * b[4] + a[2] * b[5] + a[4],
    a[1] * b[4] + a[3] * b[5] + a[5],
  ];
}

/** The rectangle a turned rectangle covers (its corners' bounds). */
export function mapRect(t: Affine, r: Rect): Rect {
  const [x1, y1] = apply(t, r.x, r.y);
  const [x2, y2] = apply(t, r.x + r.w, r.y + r.h);
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
}

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** `r` in whole pixels, at least 1 × 1 and inside W × H. */
export function clampRect(r: Rect, W: number, H: number): Rect {
  let x = Math.round(r.x);
  let y = Math.round(r.y);
  let w = Math.round(r.w);
  let h = Math.round(r.h);
  w = clamp(w, 1, W);
  h = clamp(h, 1, H);
  x = clamp(x, 0, W - w);
  y = clamp(y, 0, H - h);
  return { x, y, w, h };
}

/** A rectangle from two corners, in any order. */
export function fromCorners(x1: number, y1: number, x2: number, y2: number): Rect {
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
}

export type Handle = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw' | 'move';
export const HANDLES: Exclude<Handle, 'move'>[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

/** Where a handle sits on a rectangle. */
export function handlePoint(r: Rect, h: Exclude<Handle, 'move'>): [number, number] {
  const x = h.includes('w') ? r.x : h.includes('e') ? r.x + r.w : r.x + r.w / 2;
  const y = h.includes('n') ? r.y : h.includes('s') ? r.y + r.h : r.y + r.h / 2;
  return [x, y];
}

/**
 * The crop box after dragging `handle` by (dx, dy) from `start`, inside W × H. With a `ratio`
 * (width / height) the box keeps it: a corner follows the larger of the two moves, an edge grows the
 * other side around the middle. The opposite edge or corner stays put; nothing goes under 1 px.
 */
export function dragCrop(start: Rect, handle: Handle, dx: number, dy: number, W: number, H: number, ratio: number | null = null): Rect {
  if (handle === 'move') {
    return { x: clamp(Math.round(start.x + dx), 0, W - start.w), y: clamp(Math.round(start.y + dy), 0, H - start.h), w: start.w, h: start.h };
  }
  let x1 = start.x;
  let y1 = start.y;
  let x2 = start.x + start.w;
  let y2 = start.y + start.h;
  if (handle.includes('w')) x1 = clamp(x1 + dx, 0, x2 - 1);
  if (handle.includes('e')) x2 = clamp(x2 + dx, x1 + 1, W);
  if (handle.includes('n')) y1 = clamp(y1 + dy, 0, y2 - 1);
  if (handle.includes('s')) y2 = clamp(y2 + dy, y1 + 1, H);
  if (!ratio) return clampRect({ x: x1, y: y1, w: x2 - x1, h: y2 - y1 }, W, H);

  let w = x2 - x1;
  let h = y2 - y1;
  const corner = handle.length === 2;
  if (corner) {
    // Follow the bigger move, measured against the ratio.
    if (Math.abs(dx) >= Math.abs(dy) * ratio) h = w / ratio;
    else w = h * ratio;
  } else if (handle === 'e' || handle === 'w') h = w / ratio;
  else w = h * ratio;
  // The fixed point: the opposite corner, or the middle of the opposite edge.
  const ax = handle.includes('w') ? start.x + start.w : handle.includes('e') ? start.x : start.x + start.w / 2;
  const ay = handle.includes('n') ? start.y + start.h : handle.includes('s') ? start.y : start.y + start.h / 2;
  // Room from the fixed point to the edges the box grows toward.
  const roomW = handle.includes('w') ? ax : handle.includes('e') ? W - ax : 2 * Math.min(ax, W - ax);
  const roomH = handle.includes('n') ? ay : handle.includes('s') ? H - ay : 2 * Math.min(ay, H - ay);
  const k = Math.min(1, roomW / w, roomH / h);
  w = Math.max(1, w * k);
  h = Math.max(1, h * k);
  const x = handle.includes('w') ? ax - w : handle.includes('e') ? ax : ax - w / 2;
  const y = handle.includes('n') ? ay - h : handle.includes('s') ? ay : ay - h / 2;
  return clampRect({ x, y, w, h }, W, H);
}

/** The largest box of `ratio` (width / height) inside `r`, centred on it. */
export function fitRatio(r: Rect, ratio: number): Rect {
  let w = r.w;
  let h = w / ratio;
  if (h > r.h) {
    h = r.h;
    w = h * ratio;
  }
  return { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h };
}

/** A box of `ratio` for a free crop: the largest inside W × H, centred. */
export function cropToRatio(W: number, H: number, ratio: number): Rect {
  return clampRect(fitRatio({ x: 0, y: 0, w: W, h: H }, ratio), W, H);
}

/** The scale that brings w × h within `max` on its longer side (1 when it already fits). */
export function fitScale(w: number, h: number, max: number): number {
  return Math.min(1, max / Math.max(w, h));
}

/** The output size of a crop at (sx, sy): whole pixels, at least 1. */
export function scaled(crop: Size, sx: number, sy: number): Size {
  return { w: Math.max(1, Math.round(crop.w * sx)), h: Math.max(1, Math.round(crop.h * sy)) };
}

/**
 * The view that fits a W × H picture in a vw × vh stage with `pad` around it, never past `max`
 * zoom: { zoom, x, y } where the picture's top left lands.
 */
export function fitView(W: number, H: number, vw: number, vh: number, pad = 24, max = 8): { zoom: number; x: number; y: number } {
  const zoom = Math.max(0.01, Math.min(max, (vw - pad * 2) / W, (vh - pad * 2) / H));
  return { zoom, x: (vw - W * zoom) / 2, y: (vh - H * zoom) / 2 };
}

/** Zooming to `next` around the stage point (px, py), keeping what is under it there. */
export function zoomAt(view: { zoom: number; x: number; y: number }, next: number, px: number, py: number) {
  const k = next / view.zoom;
  return { zoom: next, x: px - (px - view.x) * k, y: py - (py - view.y) * k };
}

/** The crop presets: free, or width:height. */
export const RATIOS: { id: string; label: string; ratio: number | null }[] = [
  { id: 'free', label: 'Free', ratio: null },
  { id: '1:1', label: '1:1', ratio: 1 },
  { id: '2:1', label: '2:1', ratio: 2 },
  { id: '3:1', label: '3:1', ratio: 3 },
  { id: '4:1', label: '4:1', ratio: 4 },
];

/** Distance from (px, py) to the segment (x1, y1)–(x2, y2). */
export function segmentDistance(px: number, py: number, x1: number, y1: number, x2: number, y2: number): number {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = dx * dx + dy * dy;
  const t = len ? clamp(((px - x1) * dx + (py - y1) * dy) / len, 0, 1) : 0;
  return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy));
}
