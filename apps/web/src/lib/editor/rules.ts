// The upload rules as the editor's checklist, pure (rules.test.ts): the same for everyone, read
// from the API (`limits.emote`, and the account's own caps from `uploadQuota`), with the built-in
// numbers until it answers. Each rule says its current value and whether it passes; a failing one
// names the one-click fix the editor offers.
import { bytes, count } from '$lib/format';
import { megabytes } from '$lib/rules';

export type EditorLimits = {
  maxSide: number;
  maxBytes: number;
  maxFrames: number;
  /** No frame shorter than this (ms): 20 ms is 50 fps. */
  minDelayMs: number;
  /** Width / height at most (4, so 4:1). */
  maxAspect: number;
  /** Height / width at most (3, so 1:3). */
  maxTall: number;
  /** Width × height × frames at most (the API's decoding cap); null: none. */
  maxTotalPixels: number | null;
  /** One run of the animation at most (ms); null: none. */
  maxDurationMs: number | null;
};

export const EDITOR_LIMITS: EditorLimits = {
  maxSide: 1000,
  maxBytes: 8 * 1024 * 1024,
  maxFrames: 1000,
  minDelayMs: 20,
  maxAspect: 4,
  maxTall: 3,
  maxTotalPixels: 100_000_000,
  maxDurationMs: null,
};

type MediaLike = {
  maxBytes: number;
  maxSide: number;
  maxFrames: number;
  maxTotalPixels?: number | null;
  maxDurationMs?: number | null;
  maxAspect?: number | null;
  maxTall?: number | null;
  maxFps?: number | null;
};
type CapsLike = { maxBytes: number; maxSide: number; maxFrames: number; maxFps?: number | null; maxAspect?: number | null; maxTall?: number | null };

/** The limits from the API's answers (either may be missing): the account's caps win. */
export function editorLimits(emote?: MediaLike | null, caps?: CapsLike | null): EditorLimits {
  const l = EDITOR_LIMITS;
  const big = (n: number | null | undefined) => (n == null || n >= Number.MAX_SAFE_INTEGER ? null : n);
  return {
    maxSide: caps?.maxSide ?? emote?.maxSide ?? l.maxSide,
    maxBytes: caps?.maxBytes ?? emote?.maxBytes ?? l.maxBytes,
    maxFrames: caps?.maxFrames ?? emote?.maxFrames ?? l.maxFrames,
    // 50 fps is 20 ms a frame; the API shows shorter frames for 100 ms, so the editor holds them to it.
    minDelayMs: Math.ceil(1000 / (caps?.maxFps ?? emote?.maxFps ?? 1000 / l.minDelayMs)),
    maxAspect: caps?.maxAspect ?? emote?.maxAspect ?? l.maxAspect,
    maxTall: caps?.maxTall ?? emote?.maxTall ?? l.maxTall,
    maxTotalPixels: emote ? big(emote.maxTotalPixels) : l.maxTotalPixels,
    maxDurationMs: emote ? big(emote.maxDurationMs) : l.maxDurationMs,
  };
}

export type FixId = 'fit' | 'drop' | 'fps' | 'aspect' | 'tall' | 'shrink';
export type RuleState = 'pass' | 'fail' | 'pending';
export type Rule = { id: string; label: string; value: string; limit: string; state: RuleState; fix: FixId | null };

export type Output = {
  width: number;
  height: number;
  frames: number;
  /** The shortest frame (ms); Infinity for a still. */
  shortest: number;
  /** One run (ms). */
  duration: number;
  /** The encoded file's size; null until it is encoded. */
  size: number | null;
};

const px = (n: number) => `${count(n)} px`;
/** Megapixels: one decimal under 100, whole (1.000) above. */
const mp = (n: number) => (n >= 100_000_000 ? `${count(Math.round(n / 1_000_000))} MP` : `${String(Math.round(n / 100_000) / 10).replace('.', ',')} MP`);
const fps = (ms: number) => String(Math.round((1000 / ms) * 10) / 10).replace('.', ',');

/** The checklist for `o` against `l`. */
export function checkRules(o: Output, l: EditorLimits): Rule[] {
  const side = Math.max(o.width, o.height);
  const aspect = o.width / o.height;
  const wide = aspect > l.maxAspect + 1e-9;
  const tall = 1 / aspect > l.maxTall + 1e-9;
  const animated = o.frames > 1;
  const rules: Rule[] = [
    {
      id: 'side',
      label: 'Size',
      value: `${count(o.width)} × ${count(o.height)} px`,
      limit: `at most ${px(l.maxSide)} a side`,
      state: side <= l.maxSide ? 'pass' : 'fail',
      fix: side <= l.maxSide ? null : 'fit',
    },
    {
      id: 'aspect',
      label: 'Shape',
      value: ratioText(o.width, o.height),
      limit: wide ? `too wide: at most ${l.maxAspect}:1` : tall ? `too tall: at most 1:${l.maxTall}` : `between 1:${l.maxTall} and ${l.maxAspect}:1`,
      state: wide || tall ? 'fail' : 'pass',
      fix: wide ? 'aspect' : tall ? 'tall' : null,
    },
    {
      id: 'frames',
      label: 'Frames',
      value: count(o.frames),
      limit: `at most ${count(l.maxFrames)}`,
      state: o.frames <= l.maxFrames ? 'pass' : 'fail',
      fix: o.frames <= l.maxFrames ? null : 'drop',
    },
    {
      id: 'fps',
      label: 'Frame rate',
      value: animated ? `${fps(o.shortest)} fps at most` : 'still',
      limit: `at most ${fps(l.minDelayMs)} fps (${l.minDelayMs} ms a frame)`,
      state: !animated || o.shortest >= l.minDelayMs ? 'pass' : 'fail',
      fix: !animated || o.shortest >= l.minDelayMs ? null : 'fps',
    },
  ];
  if (l.maxTotalPixels != null) {
    const all = o.width * o.height * o.frames;
    const ok = all <= l.maxTotalPixels;
    rules.push({ id: 'pixels', label: 'Pixels in all frames', value: mp(all), limit: `at most ${mp(l.maxTotalPixels)}`, state: ok ? 'pass' : 'fail', fix: ok ? null : o.frames > 1 ? 'drop' : 'shrink' });
  }
  if (l.maxDurationMs != null && animated) {
    const ok = o.duration <= l.maxDurationMs;
    rules.push({ id: 'duration', label: 'Length', value: seconds(o.duration), limit: `at most ${seconds(l.maxDurationMs)}`, state: ok ? 'pass' : 'fail', fix: null });
  }
  rules.push({
    id: 'bytes',
    label: 'File size',
    value: o.size == null ? 'not encoded yet' : bytes(o.size),
    limit: `at most ${megabytes(l.maxBytes)} MB`,
    state: o.size == null ? 'pending' : o.size <= l.maxBytes ? 'pass' : 'fail',
    fix: o.size != null && o.size > l.maxBytes ? 'shrink' : null,
  });
  return rules;
}

/** A shape as a ratio, the long side over 1: `2,5:1` wide, `1:3` tall. */
export function ratioText(w: number, h: number): string {
  const r = w / h;
  const n = (v: number) => String(Math.round(v * 100) / 100).replace('.', ',');
  return r >= 1 ? `${n(r)}:1` : `1:${n(1 / r)}`;
}

/** Whether `w` × `h` is a shape the rules take. */
export const shapeOk = (w: number, h: number, l: Pick<EditorLimits, 'maxAspect' | 'maxTall'>) => w / h <= l.maxAspect + 1e-9 && h / w <= l.maxTall + 1e-9;

export const seconds = (ms: number) => `${String(Math.round(ms / 100) / 10).replace('.', ',')} s`;

/**
 * The rules the checklist shows. The pixel budget across all frames is checked (it gates upload
 * and the decoder needs it) but stays out of sight while it passes: the side and frame limits
 * already keep an emote far below it, so a line for it is noise. It appears if it ever fails.
 */
export const visibleRules = (rules: readonly Rule[]): Rule[] => rules.filter((r) => r.id !== 'pixels' || r.state !== 'pass');

/** Every rule passes (none failing or still waiting for the encoder). */
export const allPass = (rules: readonly Rule[]) => rules.every((r) => r.state === 'pass');

/** Only the size rule waits: everything else passes. */
export const onlySizePending = (rules: readonly Rule[]) => rules.every((r) => r.state === 'pass' || (r.id === 'bytes' && r.state === 'pending'));

/** The fix buttons' words. */
export function fixLabel(fix: FixId, l: EditorLimits): string {
  switch (fix) {
    case 'fit':
      return `Fit to ${count(l.maxSide)} px`;
    case 'drop':
      return 'Drop every 2nd frame';
    case 'fps':
      return `Limit to ${fps(l.minDelayMs)} fps`;
    case 'aspect':
      return `Crop to ${l.maxAspect}:1`;
    case 'tall':
      return `Crop to 1:${l.maxTall}`;
    case 'shrink':
      return 'Scale to 75%';
  }
}
