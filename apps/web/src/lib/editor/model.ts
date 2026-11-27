// The emote editor's document and every edit on it, pure (model.test.ts). A document never
// changes: each edit returns a new one, which is what undo steps back through (history.ts).
// The pictures themselves (decoded frames) are not in it: a frame points at a source picture by
// index, so duplicating or reordering frames copies no pixels.
import {
  FLIP_H,
  FLIP_V,
  IDENTITY,
  ROT_CCW,
  ROT_CW,
  clampRect,
  compose,
  cropToRatio,
  fitScale,
  mapRect,
  placed,
  scaled,
  turnedSize,
  type Affine,
  type Mat,
  type Rect,
  type Size,
} from './geometry';
import { boomerang, changeSpeed, dropEvery, limitFps, moveFrames, reversed, setDelay, shortest } from './timing';

export type Frame = { id: number; src: number; delay: number };

export type ShapeKind = 'pen' | 'line' | 'arrow' | 'rect' | 'ellipse' | 'text';
export type Shape = {
  id: number;
  kind: ShapeKind;
  /** pen: x, y, x, y, …; line and arrow: x1, y1, x2, y2; rect, ellipse and text: x, y, w, h. */
  pts: number[];
  color: string;
  /** The fill of a rectangle or ellipse, the outline of text. */
  fill: string;
  width: number;
  filled: boolean;
  outlined: boolean;
  text?: string;
  /** Text: its size in canvas pixels. */
  size?: number;
  /** The frame (id) it is drawn on; null on every frame. */
  frame: number | null;
};

export type Doc = {
  /** The source pictures' size (as imported). */
  sw: number;
  sh: number;
  orient: Mat;
  /** In canvas space (the source turned and mirrored), whole pixels. */
  crop: Rect;
  /** The output scale of the crop: the output is crop × (sx, sy). */
  sx: number;
  sy: number;
  frames: readonly Frame[];
  shapes: readonly Shape[];
  nextId: number;
  /**
   * Playback speed, 1 as the frames are timed (absent: 1). The frames keep their own delays; the
   * upload is timed by `exportFrames`. 0 freezes the animation: the upload is a still of the frame
   * on show.
   */
  speed?: number;
};

/** A new document for `delays.length` source pictures of sw × sh, one frame each. */
export function createDoc(sw: number, sh: number, delays: readonly number[]): Doc {
  return {
    sw,
    sh,
    orient: IDENTITY,
    crop: { x: 0, y: 0, w: sw, h: sh },
    sx: 1,
    sy: 1,
    frames: delays.map((delay, i) => ({ id: i + 1, src: i, delay })),
    shapes: [],
    nextId: delays.length + 1,
  };
}

/** The canvas (turned source) size. */
export const canvasSize = (d: Doc): Size => turnedSize(d.orient, d.sw, d.sh);
/** The output size. */
export const outSize = (d: Doc): Size => scaled(d.crop, d.sx, d.sy);
/** Source → canvas. */
export const orientTransform = (d: Doc): Affine => placed(d.orient, d.sw, d.sh);
/** Canvas → output. */
export const outputTransform = (d: Doc): Affine => {
  const o = outSize(d);
  const kx = o.w / d.crop.w;
  const ky = o.h / d.crop.h;
  return [kx, 0, 0, ky, -d.crop.x * kx, -d.crop.y * ky];
};

// ---- Crop and size ----

export function setCrop(d: Doc, r: Rect): Doc {
  const c = canvasSize(d);
  const crop = clampRect(r, c.w, c.h);
  return same(crop, d.crop) ? d : { ...d, crop };
}

const same = (a: Rect, b: Rect) => a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h;

export const resetCrop = (d: Doc): Doc => setCrop(d, { x: 0, y: 0, ...canvasSize(d) });

/** Shortens the crop to `tall` times its width (height / width) when it is taller, keeping its middle. */
export function cropNoTaller(d: Doc, tall: number): Doc {
  if (d.crop.h / d.crop.w <= tall) return d;
  const h = Math.max(1, Math.floor(d.crop.w * tall));
  return setCrop(d, { x: d.crop.x, y: d.crop.y + Math.round((d.crop.h - h) / 2), w: d.crop.w, h });
}

/** Narrows the crop to `ratio` (width / height) when it is wider, keeping its middle. */
export function cropNoWider(d: Doc, ratio: number): Doc {
  if (d.crop.w / d.crop.h <= ratio) return d;
  const w = Math.max(1, Math.floor(d.crop.h * ratio));
  return setCrop(d, { x: d.crop.x + Math.round((d.crop.w - w) / 2), y: d.crop.y, w, h: d.crop.h });
}

/** The crop at `ratio`, as large as the canvas allows, centred. */
export function cropRatio(d: Doc, ratio: number): Doc {
  const c = canvasSize(d);
  return setCrop(d, cropToRatio(c.w, c.h, ratio));
}

/** The output `w` × `h` (stretched when it doesn't match the crop's shape). */
export function resize(d: Doc, w: number, h: number): Doc {
  const sx = Math.max(1, Math.round(w)) / d.crop.w;
  const sy = Math.max(1, Math.round(h)) / d.crop.h;
  return sx === d.sx && sy === d.sy ? d : { ...d, sx, sy };
}

/** The output as wide as `w`, its height following the crop's shape. */
export function resizeKeep(d: Doc, w: number, side: 'w' | 'h' = 'w'): Doc {
  const s = side === 'w' ? Math.max(1, Math.round(w)) / d.crop.w : Math.max(1, Math.round(w)) / d.crop.h;
  // The other side of a stretched output keeps its proportion to this one.
  const k = side === 'w' ? d.sy / d.sx : d.sx / d.sy;
  return side === 'w' ? { ...d, sx: s, sy: s * k } : { ...d, sy: s, sx: s * k };
}

/** Scales the output by `k` (both sides). */
export const scaleBy = (d: Doc, k: number): Doc => ({ ...d, sx: d.sx * k, sy: d.sy * k });

/** The output within `max` px on its longer side. */
export function fitTo(d: Doc, max: number): Doc {
  const o = outSize(d);
  const k = fitScale(o.w, o.h, max);
  if (k >= 1) return d;
  // Exactly `max` on the longer side, whatever the rounding.
  const w = o.w >= o.h ? max : Math.max(1, Math.round(o.w * k));
  const h = o.h > o.w ? max : Math.max(1, Math.round(o.h * k));
  return resize(d, Math.min(w, max), Math.min(h, max));
}

export const isStretched = (d: Doc) => Math.abs(d.sx - d.sy) > 1e-9;

// ---- Turning ----

/** Turns or mirrors the canvas with `op`; the crop and the drawings turn with it. */
function turn(d: Doc, op: Mat): Doc {
  const before = canvasSize(d);
  const t = placed(op, before.w, before.h);
  const orient = compose(op, d.orient);
  const shapes = d.shapes.map((s) => mapShape(s, t));
  const next = { ...d, orient, shapes };
  // A quarter turn swaps the output's sides along with the crop's.
  const quarter = op[0] === 0;
  return { ...next, crop: clampRect(mapRect(t, d.crop), ...whole(canvasSize(next))), sx: quarter ? d.sy : d.sx, sy: quarter ? d.sx : d.sy };
}

const whole = (s: Size): [number, number] => [s.w, s.h];

export const rotateCw = (d: Doc) => turn(d, ROT_CW);
export const rotateCcw = (d: Doc) => turn(d, ROT_CCW);
export const flipH = (d: Doc) => turn(d, FLIP_H);
export const flipV = (d: Doc) => turn(d, FLIP_V);

function mapShape(s: Shape, t: Affine): Shape {
  if (s.kind === 'pen' || s.kind === 'line' || s.kind === 'arrow') {
    const pts: number[] = [];
    for (let i = 0; i + 1 < s.pts.length; i += 2) pts.push(t[0] * s.pts[i] + t[2] * s.pts[i + 1] + t[4], t[1] * s.pts[i] + t[3] * s.pts[i + 1] + t[5]);
    return { ...s, pts };
  }
  const [x, y, w, h] = s.pts;
  if (s.kind === 'text') {
    // Text stays upright: its box moves with its middle.
    const cx = x + w / 2;
    const cy = y + h / 2;
    const nx = t[0] * cx + t[2] * cy + t[4];
    const ny = t[1] * cx + t[3] * cy + t[5];
    return { ...s, pts: [nx - w / 2, ny - h / 2, w, h] };
  }
  const r = mapRect(t, { x, y, w, h });
  return { ...s, pts: [r.x, r.y, r.w, r.h] };
}

// ---- Frames ----

const indexSet = (picked: Iterable<number>) => new Set(picked);

/** Removes the frames at `picked` (indices); the last frame can't go. Their drawings go too. */
export function deleteFrames(d: Doc, picked: Iterable<number>): Doc {
  const set = indexSet(picked);
  const frames = d.frames.filter((_, i) => !set.has(i));
  if (!frames.length || frames.length === d.frames.length) return d;
  const ids = new Set(frames.map((f) => f.id));
  return { ...d, frames, shapes: d.shapes.filter((s) => s.frame === null || ids.has(s.frame)) };
}

/** Keeps only the frames from `from` to `to` (indices, inclusive). */
export function trim(d: Doc, from: number, to: number): Doc {
  const a = Math.max(0, Math.min(from, to));
  const b = Math.min(d.frames.length - 1, Math.max(from, to));
  const drop = d.frames.map((_, i) => i).filter((i) => i < a || i > b);
  return deleteFrames(d, drop);
}

/** A copy of each frame at `picked` right after it, with copies of its own drawings. */
export function duplicateFrames(d: Doc, picked: Iterable<number>): Doc {
  const set = indexSet(picked);
  let nextId = d.nextId;
  const frames: Frame[] = [];
  const shapes = [...d.shapes];
  d.frames.forEach((f, i) => {
    frames.push(f);
    if (!set.has(i)) return;
    const copy = { ...f, id: nextId++ };
    frames.push(copy);
    for (const s of d.shapes) if (s.frame === f.id) shapes.push({ ...s, id: nextId++, frame: copy.id });
  });
  return set.size ? { ...d, frames, shapes, nextId } : d;
}

export const moveFramesTo = (d: Doc, picked: Iterable<number>, to: number): Doc => ({ ...d, frames: moveFrames(d.frames, indexSet(picked), to) });
export const reverseFrames = (d: Doc): Doc => ({ ...d, frames: reversed(d.frames) });

export function boomerangFrames(d: Doc): Doc {
  let nextId = d.nextId;
  const frames = boomerang(d.frames, (f) => ({ ...f, id: nextId++ }));
  return { ...d, frames, nextId };
}

export const speedFrames = (d: Doc, speed: number): Doc => ({ ...d, frames: changeSpeed(d.frames, speed) });

/** The slowest and fastest playback speed (0 is frozen, 20 is 2000%). */
export const MAX_SPEED = 20;
export const speedOf = (d: Doc) => d.speed ?? 1;
/** `d` at playback speed `speed` (0 to 20, clamped). */
export function setSpeed(d: Doc, speed: number): Doc {
  const s = Math.max(0, Math.min(MAX_SPEED, Number.isFinite(speed) ? speed : 1));
  return s === speedOf(d) ? d : { ...d, speed: s };
}

/** Whether the upload is a still because the speed is 0 (an animation frozen on one frame). */
export const frozen = (d: Doc) => d.frames.length > 1 && speedOf(d) <= 0;

/**
 * The frames as they upload. At speed 1 the frames themselves. At 0, one frame: the one at
 * `current`. Otherwise every delay times 1 / speed; when that makes a frame shorter than
 * `minDelay` (and the frames were within it at speed 1), the animation is sampled every
 * `minDelay` ms (frames merged or dropped, `limitFps`) so it plays at the right speed and never
 * faster than the frame-rate rule. Too fast at speed 1 stays as it is (the rule says so).
 */
export function exportFrames(d: Doc, current: number, minDelay: number): readonly Frame[] {
  const s = speedOf(d);
  if (d.frames.length < 2 || s === 1) return d.frames;
  if (s <= 0) return [{ ...d.frames[Math.max(0, Math.min(d.frames.length - 1, current))], delay: 0 }];
  const sped = changeSpeed(d.frames, s);
  return shortest(d.frames) >= minDelay && shortest(sped) < minDelay ? limitFps(sped, minDelay) : sped;
}

/** `d` as it uploads: its frames timed by `exportFrames`, at speed 1. */
export function exportDoc(d: Doc, current: number, minDelay: number): Doc {
  const frames = exportFrames(d, current, minDelay);
  return frames === d.frames ? d : { ...d, frames, speed: 1 };
}
export const delayFrames = (d: Doc, delay: number, picked?: Iterable<number>): Doc => {
  const set = picked ? indexSet(picked) : null;
  return { ...d, frames: setDelay(d.frames, delay, set ? (_, i) => set.has(i) : undefined) };
};
export const dropEveryFrames = (d: Doc, n = 2): Doc => keepDrawings(d, dropEvery(d.frames, n));
export const limitFpsFrames = (d: Doc, minDelay: number): Doc => keepDrawings(d, limitFps(d.frames, minDelay));

/** Frames after a resample: drawings on a frame that is gone go too. */
function keepDrawings(d: Doc, frames: Frame[]): Doc {
  const ids = new Set(frames.map((f) => f.id));
  return { ...d, frames, shapes: d.shapes.filter((s) => s.frame === null || ids.has(s.frame)) };
}

// ---- Drawings ----

export function addShape(d: Doc, s: Omit<Shape, 'id'>): { doc: Doc; id: number } {
  const id = d.nextId;
  return { doc: { ...d, shapes: [...d.shapes, { ...s, id }], nextId: id + 1 }, id };
}

export function updateShape(d: Doc, id: number, patch: Partial<Shape>): Doc {
  return { ...d, shapes: d.shapes.map((s) => (s.id === id ? { ...s, ...patch } : s)) };
}

export const deleteShape = (d: Doc, id: number): Doc => ({ ...d, shapes: d.shapes.filter((s) => s.id !== id) });

/** The drawings shown on frame `frameId`. */
export const shapesOn = (d: Doc, frameId: number) => d.shapes.filter((s) => s.frame === null || s.frame === frameId);

/** A shape's bounds in canvas space. */
export function shapeBounds(s: Shape): Rect {
  if (s.kind === 'rect' || s.kind === 'ellipse' || s.kind === 'text') {
    const [x, y, w, h] = s.pts;
    return { x: Math.min(x, x + w), y: Math.min(y, y + h), w: Math.abs(w), h: Math.abs(h) };
  }
  let x1 = Infinity;
  let y1 = Infinity;
  let x2 = -Infinity;
  let y2 = -Infinity;
  for (let i = 0; i + 1 < s.pts.length; i += 2) {
    x1 = Math.min(x1, s.pts[i]);
    x2 = Math.max(x2, s.pts[i]);
    y1 = Math.min(y1, s.pts[i + 1]);
    y2 = Math.max(y2, s.pts[i + 1]);
  }
  return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
}

/** Moves a shape by (dx, dy). */
export function moveShape(s: Shape, dx: number, dy: number): Shape {
  if (s.kind === 'rect' || s.kind === 'ellipse' || s.kind === 'text') return { ...s, pts: [s.pts[0] + dx, s.pts[1] + dy, s.pts[2], s.pts[3]] };
  return { ...s, pts: s.pts.map((v, i) => v + (i % 2 ? dy : dx)) };
}

/** Scales a shape from `from` (its bounds at the start) to `to`. Text scales its size. */
export function fitShape(s: Shape, from: Rect, to: Rect): Shape {
  const kx = from.w ? to.w / from.w : 1;
  const ky = from.h ? to.h / from.h : 1;
  const mx = (x: number) => to.x + (x - from.x) * kx;
  const my = (y: number) => to.y + (y - from.y) * ky;
  if (s.kind === 'text') {
    const k = Math.max(0.05, Math.min(Math.abs(kx), Math.abs(ky)));
    return { ...s, size: Math.max(4, (s.size ?? 32) * k), pts: [to.x, to.y, s.pts[2] * k, s.pts[3] * k] };
  }
  if (s.kind === 'rect' || s.kind === 'ellipse') return { ...s, pts: [to.x, to.y, to.w, to.h] };
  return { ...s, pts: s.pts.map((v, i) => (i % 2 ? my(v) : mx(v))) };
}

export const framesAll = (d: Doc) => d.frames.map((_, i) => i);
export { fitScale };
