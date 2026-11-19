// Previews for the layer and effect library: each option drawn as a small paint (a layer on its
// own; an effect over a sample gradient with soft clouds, so there is something for it to change),
// shown on a sample name. Drawn a few at a time so the dialog never freezes, and kept for the
// session. A hovered or focused option plays (see `play`).
import type { AnimDoc, Engine } from './engine/index.js';

export type LibKind = 'layer' | 'effect';
/** The size previews are drawn at: the 4:1 shape of a paint on a name. */
export const THUMB_W = 160;
export const THUMB_H = 40;
/** The frame a still preview shows: a little way in, past any fade-in. */
export const STILL_FRAME = 3;

/** The paint a preview draws for one option. */
export function thumbDoc(PL: Engine, kind: LibKind, id: string): AnimDoc {
  const E = PL.Engine;
  const doc = E.blankDoc();
  if (kind === 'layer') doc.layers = [E.newLayer(id)];
  else {
    doc.layers = [
      E.newLayer('linear', { params: { stops: PL.clone(PL.PALETTES.sunset), angle: 90, speed: 0 } }),
      E.newLayer('clouds', { params: { stops: PL.clone(PL.PALETTES.mono) }, blend: 'overlay', opacity: 0.7 }),
    ];
    doc.effects = [E.newEffect(id)];
  }
  doc.width = THUMB_W;
  doc.height = THUMB_H;
  return doc;
}

let canvas: HTMLCanvasElement | null = null;
function toUrl(img: ImageData): string {
  canvas ??= document.createElement('canvas');
  canvas.width = img.width;
  canvas.height = img.height;
  canvas.getContext('2d')!.putImageData(img, 0, 0);
  return canvas.toDataURL('image/png');
}

const cache = new Map<string, string | null>();
const key = (kind: LibKind, id: string) => `${kind}:${id}`;

/** A still preview as a data URL, from the cache when drawn before; null when it can't be drawn. */
export function stillOf(PL: Engine, kind: LibKind, id: string): string | null {
  const k = key(kind, id);
  if (cache.has(k)) return cache.get(k)!;
  let url: string | null = null;
  try {
    const cp = PL.Engine.compile(thumbDoc(PL, kind, id));
    url = toUrl(PL.Engine.renderFrame(cp, Math.min(STILL_FRAME, cp.frames - 1), THUMB_W, THUMB_H));
  } catch {
    // A broken item keeps a plain preview.
  }
  cache.set(k, url);
  return url;
}

export const cached = (kind: LibKind, id: string) => cache.get(key(kind, id));

/**
 * Draws the stills of `ids` a few at a time (about 12 ms of work per slice), calling `ondraw` for
 * each. Returns a stop function; a new batch should stop the one before it.
 */
export function drawAll(PL: Engine, kind: LibKind, ids: string[], ondraw: (id: string, url: string | null) => void): () => void {
  let stopped = false;
  let i = 0;
  const step = () => {
    if (stopped) return;
    const t0 = performance.now();
    while (i < ids.length && performance.now() - t0 < 12) {
      const id = ids[i++];
      ondraw(id, stillOf(PL, kind, id));
    }
    if (i < ids.length) setTimeout(step, 0);
  };
  setTimeout(step, 0);
  return () => {
    stopped = true;
  };
}

/**
 * Plays one option's preview: frames drawn as they are shown, about 16 a second, handed to
 * `onframe` until the returned stop function is called.
 */
export function play(PL: Engine, kind: LibKind, id: string, onframe: (url: string) => void): () => void {
  let stopped = false;
  let cp: ReturnType<Engine['Engine']['compile']>;
  try {
    cp = PL.Engine.compile(thumbDoc(PL, kind, id));
  } catch {
    return () => {};
  }
  let frame = STILL_FRAME;
  let last = 0;
  const tick = (now: number) => {
    if (stopped) return;
    if (now - last >= 60) {
      last = now;
      frame = (frame + 1) % Math.max(1, cp.frames);
      try {
        onframe(toUrl(PL.Engine.renderFrame(cp, frame, THUMB_W, THUMB_H)));
      } catch {
        stopped = true;
        return;
      }
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  return () => {
    stopped = true;
  };
}
