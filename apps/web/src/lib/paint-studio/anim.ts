// Helpers for animated paints: output sizes within cyex.tv's picture limits, library search,
// pictures inside documents, and the local copy of a saved paint's layers.
import type { AnimDoc, Engine, ItemDef } from './engine/index.js';

/** Output sizes: 64 px tall is 1x on cyex.tv; taller ones stay sharp at 2x and 4x. */
export const SIZES = [
  { value: '256x64', label: '256 × 64 (4:1)', width: 256, height: 64 },
  { value: '384x64', label: '384 × 64 (6:1)', width: 384, height: 64 },
  { value: '512x64', label: '512 × 64 (8:1)', width: 512, height: 64 },
  { value: '512x128', label: '512 × 128 (4:1, sharp)', width: 512, height: 128 },
  { value: '1000x250', label: '1000 × 250 (4:1, sharpest)', width: 1000, height: 250 },
] as const;
export const FPS = [10, 20, 25, 33, 50] as const;

export const sizeKey = (d: Pick<AnimDoc, 'width' | 'height'>) => `${d.width}x${d.height}`;

/** Library search: every word must match the name, tags, category or description (synonyms count). Ranked. */
export function searchItems(PL: Engine, store: Record<string, ItemDef>, query: string): ItemDef[] {
  const items = Object.values(store);
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return items;
  const syn = (w: string) => {
    const group = PL.SEARCH_SYNONYMS?.find((g) => g.includes(w));
    return group ? group : [w];
  };
  const scored: { d: ItemDef; s: number }[] = [];
  for (const d of items) {
    const name = d.name.toLowerCase();
    const tags = (d.tags ?? []).join(' ').toLowerCase();
    const rest = `${d.cat} ${d.desc ?? ''}`.toLowerCase();
    let s = 0;
    let all = true;
    for (const w of words) {
      const alts = syn(w);
      const hit = (t: string) => alts.some((a) => t.includes(a));
      const ws = name.startsWith(w) ? 6 : name.includes(w) ? 4 : hit(name) ? 3 : hit(tags) ? 2 : hit(rest) ? 1 : 0;
      if (!ws) {
        all = false;
        break;
      }
      s += ws;
    }
    if (all) scored.push({ d, s });
  }
  return scored.sort((a, b) => b.s - a.s || a.d.name.localeCompare(b.d.name)).map((x) => x.d);
}

/** Decodes the pictures a document carries (data URLs in image params) into the engine's cache. */
export async function cachePictures(PL: Engine, doc: AnimDoc): Promise<void> {
  for (const l of doc.layers) {
    const src = l.params?.src;
    if (typeof src !== 'string' || !src.startsWith('data:image/') || PL.imageCache[src]) continue;
    try {
      // through an <img> (img-src allows data:), not fetch (connect-src doesn't)
      const img = new Image();
      img.src = src;
      await img.decode();
      const bmp = await createImageBitmap(img);
      const cv = document.createElement('canvas');
      cv.width = bmp.width;
      cv.height = bmp.height;
      const ctx = cv.getContext('2d', { willReadFrequently: true })!;
      ctx.drawImage(bmp, 0, 0);
      PL.imageCache[src] = { w: bmp.width, h: bmp.height, data: ctx.getImageData(0, 0, bmp.width, bmp.height).data };
      bmp.close();
    } catch {
      /* a broken picture draws nothing */
    }
  }
}

// The layers behind a saved picture paint stay in this browser, so the person who made it can
// edit it again; cyex.tv itself only keeps the rendered picture.
const KEY = 'cyexStudioProjects';
type Store = Record<string, { doc: AnimDoc; savedAt: string }>;
function read(): Store {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    return v && typeof v === 'object' ? v : {};
  } catch {
    return {};
  }
}
export function rememberProject(paintId: string, doc: AnimDoc) {
  try {
    const all = read();
    all[paintId] = { doc, savedAt: new Date().toISOString() };
    localStorage.setItem(KEY, JSON.stringify(all));
  } catch {
    /* storage full or blocked: the paint is saved anyway */
  }
}
export function recallProject(paintId: string): AnimDoc | null {
  return read()[paintId]?.doc ?? null;
}
