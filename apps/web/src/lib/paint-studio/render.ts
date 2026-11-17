// The page side of rendering: a renderer owns one worker (made on first use) and runs one job at
// a time; a new job cancels the one before it. Without workers (or OffscreenCanvas) it renders on
// the page in small steps instead, so the page still never freezes for a whole render.
import type { AnimDoc } from './engine/index.js';
import { runJob, type Format, type JobIn, type JobOut, type Picture } from './render-job';

export type { Format } from './render-job';
export type Rendered = { blob: Blob; frames: number; width: number; height: number };
export type RenderOptions = {
  width?: number;
  height?: number;
  format: Format;
  onProgress?: (done: number, total: number) => void;
  onStill?: (blob: Blob) => void;
};

export class Cancelled extends Error {
  constructor() {
    super('cancelled');
  }
}

const engine = () => import('./engine/index.js').then((m) => m.default);

/** Pictures (image params) the document uses, from the engine's picture cache. */
async function picturesOf(doc: AnimDoc): Promise<Picture[]> {
  const PL = await engine();
  const out: Picture[] = [];
  for (const l of doc.layers) {
    const src = l.params?.src;
    const im = typeof src === 'string' && src ? PL.imageCache[src] : null;
    if (im && typeof src === 'string') out.push({ src, w: im.w, h: im.h, data: im.data });
  }
  return out;
}

const canWork = () => typeof Worker !== 'undefined' && typeof OffscreenCanvas !== 'undefined';

export type Renderer = { render(doc: AnimDoc, opts: RenderOptions): Promise<Rendered>; cancel(): void; dispose(): void };

export function createRenderer(): Renderer {
  let worker: Worker | null = null;
  let broken = false;
  let nextId = 1;
  let current: { id: number; reject: (e: unknown) => void; flag: { cancelled: boolean } } | null = null;
  const handlers = new Map<number, (m: JobOut) => void>();

  function ensureWorker(): Worker | null {
    if (worker || broken || !canWork()) return worker;
    try {
      worker = new Worker(new URL('./render.worker.ts', import.meta.url), { type: 'module' });
      worker.onmessage = (e: MessageEvent<JobOut>) => handlers.get(e.data.id)?.(e.data);
      worker.onerror = (e) => {
        e.preventDefault();
        broken = true;
        worker?.terminate();
        worker = null;
        const c = current;
        if (c) {
          current = null;
          handlers.delete(c.id);
          c.reject(new Error('The renderer stopped. Try again.'));
        }
      };
    } catch {
      broken = true;
      worker = null;
    }
    return worker;
  }

  function cancel() {
    const c = current;
    if (!c) return;
    current = null;
    handlers.delete(c.id);
    c.flag.cancelled = true;
    worker?.postMessage({ type: 'cancel', id: c.id });
    c.reject(new Cancelled());
  }

  async function render(doc: AnimDoc, opts: RenderOptions): Promise<Rendered> {
    cancel();
    const id = nextId++;
    const flag = { cancelled: false };
    // claim the slot before the await below, so a newer call cancels this one
    const claim = { id, reject: (_: unknown) => {}, flag };
    current = claim;
    const pictures = await picturesOf(doc);
    if (flag.cancelled) throw new Cancelled();
    const job: JobIn = {
      id,
      doc: JSON.parse(JSON.stringify(doc)),
      width: opts.width ?? doc.width,
      height: opts.height ?? doc.height,
      format: opts.format,
      still: Boolean(opts.onStill),
      pictures,
    };
    return new Promise<Rendered>((resolve, reject) => {
      claim.reject = reject;
      const onMessage = (m: JobOut) => {
        if (current?.id !== id) return;
        if (m.type === 'progress') opts.onProgress?.(m.done, m.total);
        else if (m.type === 'still') opts.onStill?.(m.blob);
        else {
          handlers.delete(id);
          current = null;
          if (m.type === 'done') resolve({ blob: m.blob, frames: m.frames, width: m.width, height: m.height });
          else reject(Object.assign(new Error(m.message), { code: m.code }));
        }
      };
      handlers.set(id, onMessage);
      const w = ensureWorker();
      if (w) w.postMessage({ type: 'render', ...job });
      else
        void engine().then((PL) =>
          runJob(PL, job, onMessage, () => flag.cancelled, () => new Promise((r) => setTimeout(r, 0))),
        );
    });
  }

  return {
    render,
    cancel,
    dispose() {
      cancel();
      worker?.terminate();
      worker = null;
    },
  };
}
