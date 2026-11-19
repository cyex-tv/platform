// A name paint as CSS (ADR 0015). Pure: a Paint in, style declarations out, so the same paint
// looks the same on the profile, in the account menu, in admin previews and on channel pages.
// Everything that reaches CSS is checked first: colours against a strict #RRGGBB(AA) pattern,
// numbers clamped to the API's ranges, image addresses limited to http(s) with no characters that
// could close the url(). A paint that fails the checks falls back to its plain colour, or to the
// inherited text colour.

export type PaintStop = { at: number; color: string };
export type PaintShadow = { x: number; y: number; radius: number; color: string };
export type PaintImage = { host: string; animated: boolean; ready?: boolean; files: { name: string }[] };
export type PaintLike = {
  kind: 'LINEAR_GRADIENT' | 'RADIAL_GRADIENT' | 'IMAGE';
  angle: number;
  repeat: boolean;
  shape: 'CIRCLE' | 'ELLIPSE';
  color?: string | null;
  stops: readonly PaintStop[];
  shadows: readonly PaintShadow[];
  image?: PaintImage | null;
};

const COLOR = /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/;
/** Characters allowed in an image address inside url("…"): no quotes, parentheses, backslashes or spaces. */
const URL_SAFE = /^[A-Za-z0-9\-._~:/?#[\]@!$&*+,;=%]+$/;

export const MAX_SHADOWS = 4;
export const MAX_STOPS = 10;

/** A colour the API allows (`#RRGGBB` or `#RRGGBBAA`), or null. */
export function safeColor(c: string | null | undefined): string | null {
  return typeof c === 'string' && COLOR.test(c) ? c.toLowerCase() : null;
}

const clamp = (v: number, lo: number, hi: number) => (Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : lo);
const num = (v: number) => String(Math.round(v * 100) / 100);

/** An http(s) address safe to put in url("…"), or null. */
export function safeUrl(u: string): string | null {
  try {
    const parsed = new URL(u);
    if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') return null;
  } catch {
    return null;
  }
  return URL_SAFE.test(u) ? u : null;
}

/** The gradient stops, clamped to 0..1 and never going backwards; null when fewer than 2 are valid. */
function stopList(stops: readonly PaintStop[]): string | null {
  const out: string[] = [];
  let last = 0;
  for (const s of stops.slice(0, MAX_STOPS)) {
    const c = safeColor(s.color);
    if (!c) return null;
    last = Math.max(last, clamp(s.at, 0, 1));
    out.push(`${c} ${num(last * 100)}%`);
  }
  return out.length >= 2 ? out.join(', ') : null;
}

/** The image paint's picture: the first frame when `still` (reduced motion) and it has one. */
function savedImageUrl(img: PaintImage | null | undefined, still: boolean): string | null {
  if (!img || img.ready === false || !img.host) return null;
  const names = new Set(img.files.map((f) => f.name));
  const name = still && img.animated && names.has('1x_static.webp') ? '1x_static.webp' : '1x.webp';
  if (!names.has(name)) return null;
  return safeUrl(`${img.host.replace(/\/+$/, '')}/${name}`);
}

/**
 * An address a preview may draw from: http(s) as for saved paints, or a `blob:` URL the page made
 * itself (the paint studio's render or picked file, not on the CDN yet). Null for anything else.
 */
export function safePreviewUrl(u: string | null | undefined): string | null {
  if (!u) return null;
  // Characters that could close url("…") are percent-encoded rather than refused.
  const e = u.replace(/["'\\\n\r()\s<>]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase().padStart(2, '0')}`);
  if (e.startsWith('blob:')) return safeUrl(e.slice(5)) ? e : null;
  return safeUrl(e);
}

/**
 * The paint's `background-image` value (gradient or url), or null when it can't be drawn.
 * `imageUrl` replaces an image paint's own picture (the paint studio's local preview).
 */
export function paintImage(p: PaintLike, still = false, imageUrl?: string | null): string | null {
  const rep = p.repeat ? 'repeating-' : '';
  if (p.kind === 'IMAGE') {
    const url = imageUrl !== undefined ? safePreviewUrl(imageUrl) : savedImageUrl(p.image, still);
    return url ? `url("${url}")` : null;
  }
  const stops = stopList(p.stops);
  if (!stops) return null;
  if (p.kind === 'RADIAL_GRADIENT') return `${rep}radial-gradient(${p.shape === 'ELLIPSE' ? 'ellipse' : 'circle'}, ${stops})`;
  if (p.kind === 'LINEAR_GRADIENT') return `${rep}linear-gradient(${num(clamp(p.angle, 0, 360))}deg, ${stops})`;
  return null;
}

/** `drop-shadow(…)` filters, at most 4, values clamped to the API's ranges; '' for none. */
export function paintFilter(p: Pick<PaintLike, 'shadows'>): string {
  return p.shadows
    .slice(0, MAX_SHADOWS)
    .map((s) => {
      const c = safeColor(s.color);
      if (!c) return '';
      return `drop-shadow(${num(clamp(s.x, -16, 16))}px ${num(clamp(s.y, -16, 16))}px ${num(clamp(s.radius, 0, 16))}px ${c})`;
    })
    .filter(Boolean)
    .join(' ');
}

/** A plain colour for the paint: its fallback, else its first stop; null when it has neither. */
export function paintColor(p: PaintLike): string | null {
  return safeColor(p.color) ?? safeColor(p.stops[0]?.color);
}

/**
 * Style declarations that draw the paint into text (`style="…"` on an inline element). Without a
 * drawable paint, only the fallback colour (or nothing). `still` uses an image paint's first frame.
 * This is the one renderer: the site, the admin lists and the paint studio's previews and CSS
 * export all draw through it (the studio passes `imageUrl` for a picture not saved yet).
 */
export function paintStyle(p: PaintLike | null | undefined, still = false, imageUrl?: string | null): string {
  if (!p) return '';
  const fallback = paintColor(p);
  const image = paintImage(p, still, imageUrl);
  const out: string[] = [];
  if (fallback) out.push(`color: ${fallback}`);
  if (image && p.kind === 'IMAGE') {
    // The text itself is transparent, so a picture that doesn't load (blocked by the CSP, missing
    // on the CDN, still on its way) or has see-through parts would leave the name (or part of it)
    // invisible. A flat layer in the plain colour (or the surrounding text colour) sits under the
    // picture and shows wherever it doesn't.
    out.push(
      `background-image: ${image}, linear-gradient(currentColor, currentColor)`,
      `background-size: ${p.repeat ? 'auto 100%' : 'cover'}, 100% 100%`,
      `background-repeat: ${p.repeat ? 'repeat' : 'no-repeat'}, no-repeat`,
      'background-position: center',
      '-webkit-background-clip: text',
      'background-clip: text',
      '-webkit-text-fill-color: transparent',
    );
  } else if (image) {
    out.push(
      `background-image: ${image}`,
      'background-size: 100% 100%',
      'background-repeat: no-repeat',
      'background-position: center',
      '-webkit-background-clip: text',
      'background-clip: text',
      '-webkit-text-fill-color: transparent',
    );
  }
  const filter = paintFilter(p);
  if (filter) out.push(`filter: ${filter}`);
  return out.join('; ');
}

/** A ring (an avatar's border) in the paint: the gradient as a background, or its plain colour. */
export function paintRing(p: PaintLike | null | undefined): string {
  if (!p) return '';
  if (p.kind !== 'IMAGE') {
    const image = paintImage({ ...p, repeat: false });
    if (image) return `background: ${image}`;
  }
  const c = paintColor(p);
  return c ? `background: ${c}` : '';
}

// ── Readability floor ────────────────────────────────────────────────────────────────────────
// A pale paint vanishes on the light ground and a very dark one on the dark ground. The paint
// keeps its look; PaintedName adds a faint outline (a soft drop-shadow) only where the paint falls
// under 3:1 against the ground, and only in the theme where that holds. A paint that has shadows of
// its own gets none (the floor on top of them reads as mud).

/** The two grounds names are drawn on (tokens.css `--bg`). */
export const GROUND = { light: '#ffffff', dark: '#08090c' } as const;
/** Contrast a painted name should reach against its ground (WCAG large-text / graphics floor). */
export const FLOOR_RATIO = 3;

function channels(hex: string): [number, number, number, number] {
  const n = (i: number) => parseInt(hex.slice(i, i + 2), 16);
  return [n(1), n(3), n(5), hex.length === 9 ? n(7) / 255 : 1];
}

/** WCAG relative luminance of a `#RRGGBB` colour (0 black … 1 white). */
export function luminance(hex: string): number {
  const [r, g, b] = channels(hex).slice(0, 3).map((v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}

/** WCAG contrast ratio between two colours, 1..21. A translucent `fg` is blended onto `bg` first. */
export function contrast(fg: string, bg: string): number {
  const [r, g, b, a] = channels(fg);
  const [br, bg_, bb] = channels(bg);
  const mix = (f: number, k: number) => Math.round(f * a + k * (1 - a));
  const flat = `#${[mix(r, br), mix(g, bg_), mix(b, bb)].map((v) => v.toString(16).padStart(2, '0')).join('')}`;
  const [l1, l2] = [luminance(flat), luminance(bg)].sort((x, y) => y - x);
  return (l1! + 0.05) / (l2! + 0.05);
}

/** The colours a paint is drawn in: its stops, else its plain colour. Empty for an image paint without one. */
function paintColors(p: PaintLike): string[] {
  const stops = p.kind !== 'IMAGE' ? p.stops.map((s) => safeColor(s.color)).filter((c): c is string => !!c) : [];
  if (stops.length >= 2) return stops;
  const c = paintColor(p);
  return c ? [c] : [];
}

/**
 * Which theme a paint needs a readability floor in: 'light' when any of its colours is under 3:1
 * on the white ground, 'dark' when under 3:1 on the dark one, null when it reads on both (or has
 * no known colour, like an image paint without a fallback). A paint can't be both: no colour is
 * under 3:1 on both grounds, but a gradient from pale to near-black can; then 'both'.
 */
export function paintFloor(p: PaintLike | null | undefined): 'light' | 'dark' | 'both' | null {
  if (!p) return null;
  // A paint with shadows of its own already carries its outline: adding the floor over them muddies it.
  if (p.shadows?.length) return null;
  const cs = paintColors(p);
  if (!cs.length) return null;
  const light = cs.some((c) => contrast(c, GROUND.light) < FLOOR_RATIO);
  const dark = cs.some((c) => contrast(c, GROUND.dark) < FLOOR_RATIO);
  return light && dark ? 'both' : light ? 'light' : dark ? 'dark' : null;
}

/** The readability floor's halo on each ground (PaintedName draws the same in its styles). */
export const FLOOR_FILTER = {
  light: 'drop-shadow(0 0 0.5px rgb(0 0 0 / 0.55))',
  dark: 'drop-shadow(0 0 0.5px rgb(255 255 255 / 0.55))',
} as const;

/** The halo a paint needs on this ground ('' when it reads without one): for previews drawn on a fixed ground. */
export function floorFilter(p: PaintLike | null | undefined, ground: 'light' | 'dark'): string {
  const f = paintFloor(p);
  return f === 'both' || f === ground ? FLOOR_FILTER[ground] : '';
}
