// The cyex.tv paint as the studio edits it, the API's limits (ADR 0015) and the checks that
// enforce them live, with messages people can act on. Colours are `#RRGGBB` or `#RRGGBBAA`,
// stored upper case, the way the API keeps them.
import { PICTURE_RULES, checkAnimation, checkSize } from '$lib/cosmetics/upload';

export type PaintKind = 'LINEAR_GRADIENT' | 'RADIAL_GRADIENT' | 'IMAGE';
export type PaintShape = 'CIRCLE' | 'ELLIPSE';
export type Stop = { at: number; color: string };
export type Shadow = { x: number; y: number; radius: number; color: string };

/** One paint in the studio. `image` is the picture an IMAGE paint shows (its URL for previews). */
export type StudioPaint = {
  name: string;
  kind: PaintKind;
  angle: number;
  repeat: boolean;
  shape: PaintShape;
  /** Shown where a client can't draw the paint. */
  color: string | null;
  stops: Stop[];
  shadows: Shadow[];
  image: { id: string | null; url: string | null } | null;
};

export const LIMITS = {
  nameMax: 32,
  stopsMin: 2,
  stopsMax: 10,
  shadowsMax: 4,
  offsetMax: 16,
  radiusMax: 16,
  angleMax: 360,
  /**
   * Paint pictures (the same rules as `PICTURE_RULES.paint` in $lib/cosmetics/upload): at least 64
   * px tall, at most 4.096 px a side, 8:1 at most, 8 MB, 120 frames, 10 seconds.
   */
  imageMinHeight: PICTURE_RULES.paint.minHeight,
  imageMaxSide: PICTURE_RULES.paint.maxSide,
  imageMaxAspect: PICTURE_RULES.paint.maxAspect,
  imageMaxBytes: PICTURE_RULES.paint.maxBytes,
  imageMaxFrames: PICTURE_RULES.paint.maxFrames,
  imageMaxDurationMs: PICTURE_RULES.paint.maxDurationMs,
} as const;

// ---- colours ----

const HEX = /^#?([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;

/** `#RRGGBB` or `#RRGGBBAA` (upper case) from any 3, 4, 6 or 8 digit hex, with or without `#`; null when it isn't one. Full alpha is dropped. */
export function normHex(input: string | null | undefined): string | null {
  const m = HEX.exec(String(input ?? '').trim());
  if (!m) return null;
  let h = m[1].toUpperCase();
  if (h.length <= 4) h = [...h].map((c) => c + c).join('');
  if (h.length === 8 && h.endsWith('FF')) h = h.slice(0, 6);
  return '#' + h;
}

/** Whether the API takes this colour as it is (`#RRGGBB` or `#RRGGBBAA`). */
export const isApiHex = (c: unknown): c is string => typeof c === 'string' && /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(c);

/** [r, g, b, a] in 0..255 from a hex colour (black when it isn't one). */
export function hexToRgba(hex: string): [number, number, number, number] {
  const h = (normHex(hex) ?? '#000000').slice(1);
  const n = (i: number) => parseInt(h.slice(i, i + 2), 16);
  return [n(0), n(2), n(4), h.length === 8 ? n(6) : 255];
}

const two = (v: number) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0').toUpperCase();

/** `#RRGGBB`, or `#RRGGBBAA` when `a` is under 255. */
export function rgbaToHex(r: number, g: number, b: number, a = 255): string {
  return '#' + two(r) + two(g) + two(b) + (Math.round(a) >= 255 ? '' : two(a));
}

/** The colour without its alpha (`#RRGGBB`), for `<input type="color">`. */
export const opaque = (hex: string) => (normHex(hex) ?? '#000000').slice(0, 7);
/** Alpha 0..1. */
export const alphaOf = (hex: string) => hexToRgba(hex)[3] / 255;
/** The same colour with a new alpha (0..1). */
export function withAlpha(hex: string, alpha: number): string {
  const [r, g, b] = hexToRgba(hex);
  return rgbaToHex(r, g, b, Math.round(Math.min(1, Math.max(0, alpha)) * 255));
}

// ---- blanks ----

export function blankPaint(): StudioPaint {
  return {
    name: '',
    kind: 'LINEAR_GRADIENT',
    angle: 90,
    repeat: false,
    shape: 'CIRCLE',
    color: '#FF7A45',
    stops: [
      { at: 0, color: '#FF7A45' },
      { at: 0.5, color: '#FF3D7F' },
      { at: 1, color: '#8F5BFF' },
    ],
    shadows: [],
    image: null,
  };
}

export const clonePaint = (p: StudioPaint): StudioPaint => ({
  ...p,
  stops: p.stops.map((s) => ({ ...s })),
  shadows: p.shadows.map((s) => ({ ...s })),
  image: p.image ? { ...p.image } : null,
});

// ---- checks ----

export type Field = 'name' | 'angle' | 'color' | 'stops' | 'shadows' | 'image';
export type Issue = { field: Field; index?: number; message: string };

// Hidden and control characters the API refuses in names (crates/shared/src/text.rs).
const HIDDEN = /[\p{Cc}\p{Cf}\p{Co}\p{Cn}\p{Zl}\p{Zp}]/u;

/** Everything the API would refuse, as messages for the fields they belong to. Empty when it can be saved. */
export function validatePaint(p: StudioPaint): Issue[] {
  const out: Issue[] = [];
  const name = p.name.trim();
  if (!name) out.push({ field: 'name', message: 'Give the paint a name.' });
  else if ([...name].length > LIMITS.nameMax) out.push({ field: 'name', message: `Names are at most ${LIMITS.nameMax} characters (this one has ${[...name].length}).` });
  else if (HIDDEN.test(name)) out.push({ field: 'name', message: 'Names can’t have hidden or control characters.' });

  if (p.kind === 'LINEAR_GRADIENT' && !(Number.isInteger(p.angle) && p.angle >= 0 && p.angle <= LIMITS.angleMax))
    out.push({ field: 'angle', message: 'The angle is a whole number from 0 to 360 degrees.' });
  if (p.color != null && !isApiHex(p.color)) out.push({ field: 'color', message: 'Use #RRGGBB or #RRGGBBAA for the fallback colour.' });

  if (p.kind === 'IMAGE') {
    if (!p.image?.id && !p.image?.url) out.push({ field: 'image', message: 'Build or upload a picture for this paint.' });
  } else {
    const n = p.stops.length;
    if (n < LIMITS.stopsMin) out.push({ field: 'stops', message: `A gradient needs at least ${LIMITS.stopsMin} stops.` });
    if (n > LIMITS.stopsMax) out.push({ field: 'stops', message: `A gradient has at most ${LIMITS.stopsMax} stops (this one has ${n}). Remove ${n - LIMITS.stopsMax}.` });
    let last = 0;
    p.stops.forEach((s, i) => {
      if (!(s.at >= 0 && s.at <= 1)) out.push({ field: 'stops', index: i, message: `Stop ${i + 1} is outside 0–100%.` });
      else if (s.at < last) out.push({ field: 'stops', index: i, message: `Stop ${i + 1} sits before stop ${i}. Stops go left to right.` });
      if (!isApiHex(s.color)) out.push({ field: 'stops', index: i, message: `Stop ${i + 1} needs a #RRGGBB or #RRGGBBAA colour.` });
      last = Math.max(last, s.at);
    });
  }

  if (p.shadows.length > LIMITS.shadowsMax) out.push({ field: 'shadows', message: `At most ${LIMITS.shadowsMax} shadows (this paint has ${p.shadows.length}).` });
  p.shadows.forEach((s, i) => {
    if (!(Math.abs(s.x) <= LIMITS.offsetMax && Math.abs(s.y) <= LIMITS.offsetMax))
      out.push({ field: 'shadows', index: i, message: `Shadow ${i + 1}: offsets are −16 to 16 px.` });
    if (!(s.radius >= 0 && s.radius <= LIMITS.radiusMax)) out.push({ field: 'shadows', index: i, message: `Shadow ${i + 1}: the blur is 0 to 16 px.` });
    if (!isApiHex(s.color)) out.push({ field: 'shadows', index: i, message: `Shadow ${i + 1} needs a #RRGGBB or #RRGGBBAA colour.` });
  });
  return out;
}

/**
 * Problems with a paint picture before it is uploaded (size, sides, shape, frames, run time):
 * what the API refuses, said here first.
 */
export function checkPicture(info: { bytes: number; width: number; height: number; frames?: number; durationMs?: number }): string[] {
  const out: string[] = [];
  const size = checkSize('paint', info.width, info.height);
  if (size) out.push(size);
  if (info.bytes > LIMITS.imageMaxBytes) out.push(`Paint pictures are at most 8 MB (this one is ${fmtBytes(info.bytes)}). Try WebP, fewer frames or a smaller size.`);
  const frames = info.frames ?? 1;
  if (frames > 1 && !size) {
    const why = checkAnimation('paint', { frames, durationMs: info.durationMs ?? 0 }, info.width, info.height);
    if (why) out.push(why);
  }
  return out;
}

/** The stops sorted left to right (stable), positions clamped to 0..1. */
export function sortStops(stops: Stop[]): Stop[] {
  return stops
    .map((s, i) => ({ s: { at: Math.min(1, Math.max(0, s.at)), color: s.color }, i }))
    .sort((a, b) => a.s.at - b.s.at || a.i - b.i)
    .map((x) => x.s);
}

/** Positions rounded to 4 decimals (what a person can set, and what survives a round trip). */
export const round4 = (x: number) => Math.round(x * 10000) / 10000;

// ---- formats (cyex: 1.000, 1,5 MB) ----

export const fmtInt = (n: number) => String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
export function fmtBytes(b: number): string {
  if (b < 1024) return fmtInt(b) + ' B';
  const units = ['KB', 'MB', 'GB'];
  let i = -1;
  do {
    b /= 1024;
    i++;
  } while (b >= 1024 && i < units.length - 1);
  return (b < 10 ? b.toFixed(1) : String(Math.round(b))).replace('.', ',') + ' ' + units[i];
}
/** A percentage for a stop position: 0 to 100, at most one decimal, decimal comma. */
export const fmtPct = (at: number) => String(Math.round(at * 1000) / 10).replace('.', ',') + '%';
