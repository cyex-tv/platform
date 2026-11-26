// Frame timing for the emote editor, pure (timing.test.ts). A frame is shown for its `delay` in
// whole milliseconds; an animation loops from its last frame straight back to the first, so the
// last frame keeps its own duration (a video's last sample, not zero and not a copy of the first).
// Rounding is cumulative everywhere: each frame ends at the rounded end time of the exact one, so
// a 30 fps clip is 33, 34, 33, 33, 34, … ms and its loop lasts exactly as long as the clip.

export type Timed = { delay: number };

/** Browsers show a GIF frame under 20 ms for 100 ms, and the API counts it the same (shown_ms). */
export const SHORT_DELAY_MS = 20;
export const SHORT_SHOWN_MS = 100;

/** How long an imported frame is shown: as the browser showed it. */
export function shownDelay(delay: number, frames: number): number {
  if (frames <= 1) return 0;
  return !Number.isFinite(delay) || delay < SHORT_DELAY_MS ? SHORT_SHOWN_MS : Math.round(delay);
}

/** Cumulative rounding of exact durations (any unit, scaled by `k` into ms): whole, never under `min`. */
function roundRun(exact: readonly number[], k = 1, min = 1): number[] {
  const out: number[] = [];
  let t = 0;
  let shown = 0;
  for (const d of exact) {
    t += d * k;
    const end = Math.max(shown + min, Math.round(t));
    out.push(end - shown);
    shown = end;
  }
  return out;
}

/**
 * Delays (ms) for frames shown at `times` (µs, any order is sorted) ending at `end` µs: frame i
 * lasts until frame i + 1 starts, the last until `end`. Equal times keep their order and get 1 ms.
 */
export function delaysFromTimes(times: readonly number[], end: number): number[] {
  if (!times.length) return [];
  const t = [...times].sort((a, b) => a - b);
  const exact = t.map((v, i) => (i + 1 < t.length ? t[i + 1] : Math.max(end, v + 1)) - v);
  return roundRun(exact, 1 / 1000);
}

/** Delays for `count` frames at `fps`, the last one ending at `duration` ms when given. */
export function delaysAtFps(count: number, fps: number, duration?: number): number[] {
  const step = 1000 / fps;
  const exact = Array.from({ length: count }, (_, i) => (i + 1 < count || duration === undefined ? step : Math.max(1, duration - i * step)));
  return roundRun(exact);
}

export const total = (frames: readonly Timed[]) => frames.reduce((s, f) => s + f.delay, 0);

/** The shortest delay of an animation (Infinity for a still). */
export function shortest(frames: readonly Timed[]): number {
  return frames.length > 1 ? Math.min(...frames.map((f) => f.delay)) : Infinity;
}

/** Frames per second at the shortest delay (0 for a still). */
export const peakFps = (frames: readonly Timed[]) => (frames.length > 1 ? 1000 / shortest(frames) : 0);

/** Every delay times 1 / `speed` (2 = twice as fast), rounded cumulatively, at least 1 ms. */
export function changeSpeed<T extends Timed>(frames: readonly T[], speed: number): T[] {
  const d = roundRun(
    frames.map((f) => f.delay),
    1 / speed,
  );
  return frames.map((f, i) => ({ ...f, delay: d[i] }));
}

export const setDelay = <T extends Timed>(frames: readonly T[], delay: number, pick?: (f: T, i: number) => boolean): T[] =>
  frames.map((f, i) => (!pick || pick(f, i) ? { ...f, delay: Math.max(1, Math.round(delay)) } : f));

export const reversed = <T,>(frames: readonly T[]): T[] => [...frames].reverse();

/**
 * Forward then back: 0 1 2 3 2 1, so neither end is shown twice in a row when it loops.
 * `copy` makes the frames of the way back (new ids).
 */
export function boomerang<T>(frames: readonly T[], copy: (f: T) => T): T[] {
  if (frames.length < 3) return [...frames, ...frames.slice(1).reverse().slice(1).map(copy)];
  return [...frames, ...frames.slice(1, -1).reverse().map(copy)];
}

/**
 * Keeps every `n`th frame (the first one stays); each kept frame takes the time of the ones
 * dropped after it, so the loop lasts as long as before.
 */
export function dropEvery<T extends Timed>(frames: readonly T[], n = 2): T[] {
  if (n < 2 || frames.length < 2) return [...frames];
  const out: T[] = [];
  frames.forEach((f, i) => {
    if (i % n === 0) out.push({ ...f });
    else out[out.length - 1].delay += f.delay;
  });
  return out;
}

/**
 * At most 1000 / `min` frames per second: the animation sampled every `min` ms (the frame showing
 * at that moment), a frame picked twice in a row becoming one longer frame. The run lasts exactly
 * as long; a last sample shorter than `min` joins the one before. Already within it: unchanged.
 */
export function limitFps<T extends Timed>(frames: readonly T[], min = SHORT_DELAY_MS): T[] {
  if (frames.length < 2 || shortest(frames) >= min) return [...frames];
  const starts: number[] = [];
  let t = 0;
  for (const f of frames) {
    starts.push(t);
    t += f.delay;
  }
  const end = t;
  const out: { i: number; delay: number }[] = [];
  let at = 0;
  for (let k = 0; k * min < end; k++) {
    const time = k * min;
    while (at + 1 < frames.length && starts[at + 1] <= time) at++;
    const d = Math.min(min, end - time);
    const last = out[out.length - 1];
    if (last && last.i === at) last.delay += d;
    else out.push({ i: at, delay: d });
  }
  const tail = out[out.length - 1];
  if (out.length > 1 && tail.delay < min) {
    out.pop();
    out[out.length - 1].delay += tail.delay;
  }
  return out.map((o) => ({ ...frames[o.i], delay: o.delay }));
}

/** Delays in GIF's hundredths of a second, rounded cumulatively, never under 2 (20 ms). */
export function gifDelays(delays: readonly number[]): number[] {
  return roundRun(delays, 1 / 10, 2);
}

/** Moves the frames at `picked` (indices) to sit before `to` (an index in the original list). */
export function moveFrames<T>(frames: readonly T[], picked: ReadonlySet<number>, to: number): T[] {
  const moving = frames.filter((_, i) => picked.has(i));
  const before = frames.slice(0, to).filter((_, i) => !picked.has(i)).length;
  const rest = frames.filter((_, i) => !picked.has(i));
  return [...rest.slice(0, before), ...moving, ...rest.slice(before)];
}
