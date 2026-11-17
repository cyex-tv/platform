// One render job: every frame of an animated paint, then one encoded file. Runs in the render
// worker, or on the page when workers aren't available (yielding between frames either way).
import type { AnimDoc, Engine } from './engine/index.js';

export type Format = 'webp' | 'apng' | 'gif';
export type Picture = { src: string; w: number; h: number; data: Uint8ClampedArray };
export type JobIn = { id: number; doc: AnimDoc; width: number; height: number; format: Format; still: boolean; pictures: Picture[] };
export type JobOut =
  | { type: 'progress'; id: number; done: number; total: number }
  | { type: 'still'; id: number; blob: Blob }
  | { type: 'done'; id: number; blob: Blob; frames: number; width: number; height: number }
  | { type: 'error'; id: number; message: string; code?: string };

export const MIME: Record<Format, string> = { webp: 'image/webp', apng: 'image/png', gif: 'image/gif' };
export const EXT: Record<Format, string> = { webp: 'webp', apng: 'png', gif: 'gif' };

export async function runJob(PL: Engine, job: JobIn, post: (m: JobOut) => void, cancelled: () => boolean, pause: () => Promise<void>) {
  const { id } = job;
  try {
    for (const p of job.pictures) PL.imageCache[p.src] = { w: p.w, h: p.h, data: p.data };
    const cp = PL.Engine.compile(job.doc);
    const total = cp.frames;
    const frames: ImageData[] = [];
    for (let i = 0; i < total; i++) {
      if (cancelled()) return;
      frames.push(PL.Engine.renderFrame(cp, i, job.width, job.height));
      if (i === 0 && job.still) post({ type: 'still', id, blob: await PL.Encode.encodeApng([frames[0]]) });
      post({ type: 'progress', id, done: i + 1, total });
      await pause();
    }
    if (cancelled()) return;
    const delayMs = 1000 / job.doc.fps;
    let blob: Blob;
    if (job.format === 'gif') blob = PL.Encode.encodeGif(frames, { delayCs: Math.max(2, Math.round(100 / job.doc.fps)), colors: 256, alphaCut: 110 });
    else if (job.format === 'webp') {
      try {
        blob = await PL.Encode.encodeWebp(frames, { delayMs, quality: 0.9 });
      } catch (e) {
        if ((e as Error)?.message === 'webp-unsupported') {
          post({ type: 'error', id, message: 'This browser can’t make WebP files. Choose APNG or GIF.', code: 'webp-unsupported' });
          return;
        }
        throw e;
      }
    } else blob = await PL.Encode.encodeApng(frames, { delayMs });
    if (blob.type !== MIME[job.format]) blob = new Blob([blob], { type: MIME[job.format] });
    post({ type: 'done', id, blob, frames: total, width: job.width, height: job.height });
  } catch (e) {
    post({ type: 'error', id, message: (e as Error)?.message || String(e) });
  }
}
