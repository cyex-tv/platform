// Import and export of other paint JSON: the widely shared single-gradient format with integer
// colours (red in the top byte, alpha in the low one; JSON from other paint tools and our own
// cyex paints tool), and cyex paints project files. Written from the public JSON shape only.
// Import always lands in the cyex.tv model and says what it had to change.
import { LIMITS, normHex, rgbaToHex, round4, type PaintKind, type Shadow, type Stop, type StudioPaint, blankPaint } from './format';

/** Packed signed 32-bit RGBA integer → `#RRGGBB(AA)`. */
export function intToHex(n: number): string {
  const u = n >>> 0;
  return rgbaToHex(Math.floor(u / 0x1000000) % 256, Math.floor(u / 0x10000) % 256, Math.floor(u / 0x100) % 256, u % 256);
}

/** `#RRGGBB(AA)` → packed signed 32-bit RGBA integer (alpha 255 when the hex has none). */
export function hexToInt(hex: string): number {
  const h = (normHex(hex) ?? '#000000').slice(1);
  const a = h.length === 8 ? parseInt(h.slice(6, 8), 16) : 255;
  const u = parseInt(h.slice(0, 6), 16) * 256 + a; // up to 2^32 - 1
  return u > 0x7fffffff ? u - 0x100000000 : u;
}

/** A colour in imported JSON: an integer (or a string of one), or any hex. Null when it's neither. */
export function anyColour(c: unknown): string | null {
  if (typeof c === 'number' && Number.isFinite(c)) return intToHex(c | 0);
  if (typeof c === 'string') {
    const t = c.trim();
    if (/^-?\d+$/.test(t)) return intToHex(parseInt(t, 10) | 0);
    return normHex(t);
  }
  return null;
}

type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v);
const num = (v: unknown, d = 0) => (Number.isFinite(Number(v)) ? Number(v) : d);
const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));

export type Imported = { paint: StudioPaint; warnings: string[] };

/**
 * Reads paint JSON into the cyex.tv model. Accepts the single-gradient format (`function`, `angle`,
 * `repeat`, `shape`, `stops`, `shadows` with `x_offset`/`y_offset`, `image_url`), its API wrapper
 * (`{ name, data: { ... } }`), a layered paint (`gradients[]`: the first layer is kept), a cyex.tv
 * paint (`kind`, `x`/`y`, hex colours), integer or hex colours and stop positions in 0..1 or percent.
 */
export function importPaint(input: unknown): Imported {
  const warnings: string[] = [];
  let src: unknown = typeof input === 'string' ? JSON.parse(input) : input;
  if (isObj(src) && isObj(src.data) && !src.stops && !src.gradients) src = { name: src.name, ...src.data };
  if (!isObj(src)) throw new Error('That isn’t a paint: expected a JSON object.');
  const p = blankPaint();
  p.shadows = [];

  const rawName = String(src.name ?? '').trim();
  p.name = [...rawName].slice(0, LIMITS.nameMax).join('');
  if ([...rawName].length > LIMITS.nameMax) warnings.push(`The name was cut to ${LIMITS.nameMax} characters.`);

  let g: Obj = src;
  if (Array.isArray(src.gradients) && src.gradients.length) {
    const layers = src.gradients.filter(isObj);
    if (layers[0]) g = layers[0];
    if (layers.length > 1) warnings.push(`cyex.tv paints have one gradient; the first of ${layers.length} layers was kept.`);
    const t = src.text;
    if (isObj(t) && Object.keys(t).length) warnings.push('Text styles (weight, outline, text shadows) aren’t part of cyex.tv paints and were dropped.');
  }

  let bad = 0;
  const colour = (c: unknown, fallback = '#000000') => {
    const h = anyColour(c);
    if (h) return h;
    bad++;
    return fallback;
  };

  const fn = String(g.kind ?? g.function ?? 'LINEAR_GRADIENT').toUpperCase().replace(/-/g, '_');
  let kind: PaintKind = fn.includes('RADIAL') ? 'RADIAL_GRADIENT' : fn === 'URL' || fn === 'IMAGE' ? 'IMAGE' : 'LINEAR_GRADIENT';
  if (fn.includes('CONIC')) warnings.push('Conic gradients aren’t supported; it became a linear gradient.');
  p.kind = kind;
  p.repeat = g.repeat === true;

  const ang = num(g.angle, 90);
  const wrapped = ((Math.round(ang) % 360) + 360) % 360;
  p.angle = ang === 360 ? 360 : wrapped;
  if (p.angle !== ang) warnings.push(`The angle ${String(ang).replace('.', ',')}° became ${p.angle}°.`);
  p.shape = String(g.shape ?? '').toUpperCase() === 'ELLIPSE' ? 'ELLIPSE' : 'CIRCLE';

  if (kind === 'IMAGE') {
    const url = typeof g.image_url === 'string' ? g.image_url : isObj(g.image) && typeof g.image.host === 'string' ? g.image.host : '';
    p.stops = [];
    p.image = { id: null, url: safeUrl(url) };
    warnings.push('Picture paints are saved with a picture on cyex.tv: build one in Animated or upload the file.');
  } else {
    const raw = (Array.isArray(g.stops) ? g.stops : []).filter(isObj);
    const percent = raw.some((s) => num(s.at) > 1);
    let stops: Stop[] = raw.map((s) => ({ at: round4(clamp(num(s.at) / (percent ? 100 : 1), 0, 1)), color: colour(s.color) }));
    // never decreasing: CSS treats a stop before the one ahead of it as sitting on it
    let last = 0;
    let moved = false;
    stops = stops.map((s) => {
      if (s.at < last) {
        moved = true;
        return { ...s, at: last };
      }
      last = s.at;
      return s;
    });
    if (moved) warnings.push('Stops that went backwards were moved up to the stop before them.');
    if (stops.length > LIMITS.stopsMax) {
      warnings.push(`cyex.tv gradients have at most ${LIMITS.stopsMax} stops; ${stops.length - LIMITS.stopsMax} were dropped. Check the result.`);
      stops = thinStops(stops, LIMITS.stopsMax);
    }
    if (stops.length === 1) stops = [{ ...stops[0], at: 0 }, { ...stops[0], at: 1 }];
    if (stops.length === 0) {
      stops = blankPaint().stops;
      warnings.push('The paint had no stops, so the default ones were added.');
    }
    p.stops = stops;
  }

  const shadows = (Array.isArray(src.shadows) ? src.shadows : []).filter(isObj);
  let clamped = false;
  p.shadows = shadows.slice(0, LIMITS.shadowsMax).map((s): Shadow => {
    const x = num(s.x ?? s.x_offset), y = num(s.y ?? s.y_offset), r = num(s.radius);
    const out = { x: clamp(x, -LIMITS.offsetMax, LIMITS.offsetMax), y: clamp(y, -LIMITS.offsetMax, LIMITS.offsetMax), radius: clamp(r, 0, LIMITS.radiusMax), color: colour(s.color, '#000000') };
    if (out.x !== x || out.y !== y || out.radius !== r) clamped = true;
    return out;
  });
  if (shadows.length > LIMITS.shadowsMax) warnings.push(`cyex.tv paints have at most ${LIMITS.shadowsMax} shadows; ${shadows.length - LIMITS.shadowsMax} were dropped.`);
  if (clamped) warnings.push('Shadow offsets were limited to −16 to 16 px and blur to 0 to 16 px.');

  p.color = src.color == null ? (p.stops[0]?.color.slice(0, 7) ?? null) : colour(src.color, '#000000');
  if (bad) warnings.push(bad === 1 ? 'One colour wasn’t valid and became black.' : `${bad} colours weren’t valid and became black.`);
  return { paint: p, warnings };
}

// Keep the first and last stop and spread the rest evenly, so the gradient keeps its ends.
function thinStops(stops: Stop[], max: number): Stop[] {
  const out: Stop[] = [];
  for (let i = 0; i < max; i++) out.push(stops[Math.round((i * (stops.length - 1)) / (max - 1))]);
  return out;
}

/** Only http(s) links survive an import. */
export function safeUrl(u: unknown): string | null {
  if (typeof u !== 'string' || !u.trim()) return null;
  try {
    const x = new URL(u.trim());
    return x.protocol === 'https:' || x.protocol === 'http:' ? x.href : null;
  } catch {
    return null;
  }
}

/** The paint as single-gradient JSON with integer colours (what other paint tools read). */
export function exportPaint(p: StudioPaint): string {
  const o: Obj = {
    name: p.name,
    function: p.kind === 'IMAGE' ? 'URL' : p.kind,
    repeat: p.repeat,
    angle: p.angle,
    shape: p.shape.toLowerCase(),
    image_url: p.kind === 'IMAGE' ? (p.image?.url ?? '') : '',
    stops: p.kind === 'IMAGE' ? [] : p.stops.map((s) => ({ at: s.at, color: hexToInt(s.color) })),
    shadows: p.shadows.map((s) => ({ x_offset: s.x, y_offset: s.y, radius: s.radius, color: hexToInt(s.color) })),
  };
  if (p.color) o.color = hexToInt(p.color);
  return JSON.stringify(o, null, 2);
}

/** The paint in cyex.tv's own JSON (hex colours, the API's field names). */
export function exportCyex(p: StudioPaint): string {
  return JSON.stringify(
    {
      name: p.name,
      kind: p.kind,
      angle: p.angle,
      repeat: p.repeat,
      shape: p.shape,
      color: p.color,
      stops: p.kind === 'IMAGE' ? [] : p.stops,
      shadows: p.shadows,
      ...(p.kind === 'IMAGE' ? { image: p.image?.url ?? null } : {}),
    },
    null,
    2,
  );
}

/** A cyex paints project file (`{ cyexPaintProject: 1, doc }`): returns the document, or null when the JSON isn't one. */
export function projectDoc(input: unknown): unknown | null {
  return isObj(input) && input.cyexPaintProject && isObj(input.doc) ? input.doc : null;
}

/** Engine shadows (integer colours, `x_offset`) to cyex.tv shadows. */
export function engineShadows(list: unknown): Shadow[] {
  return (Array.isArray(list) ? list : []).filter(isObj).slice(0, LIMITS.shadowsMax).map((s) => ({
    x: clamp(num(s.x_offset), -LIMITS.offsetMax, LIMITS.offsetMax),
    y: clamp(num(s.y_offset), -LIMITS.offsetMax, LIMITS.offsetMax),
    radius: clamp(num(s.radius), 0, LIMITS.radiusMax),
    color: anyColour(s.color) ?? '#000000',
  }));
}

/** cyex.tv shadows to the engine's form (kept in project files). */
export const toEngineShadows = (list: Shadow[]) => list.map((s) => ({ x_offset: s.x, y_offset: s.y, radius: s.radius, color: hexToInt(s.color) }));
