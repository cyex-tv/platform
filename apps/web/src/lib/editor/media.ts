// The editor's side of the media worker: import a file as frames, encode the finished frames.
// Everything stays in this browser tab; the encoded file goes to the upload form like a picked one.
// A video the worker can't decode (WebM, or no VideoDecoder) plays here in a <video>, frame by
// frame (videoFrames).
import type { WorkerIn, WorkerOut } from './media-protocol';
import { importScale, sniff, type Budget, type MediaKind } from './sniff';
import { delaysAtFps, delaysFromTimes, gifDelays, shownDelay } from './timing';
import { outSize, type Doc } from './model';
import { paintOutput } from './render';

export class Cancelled extends Error {
  constructor() {
    super('cancelled');
  }
}

export type Imported = {
  kind: MediaKind;
  /** The file's own size, and the size it was decoded at. */
  width: number;
  height: number;
  scaledWidth: number;
  scaledHeight: number;
  /** Frames in the file, and how many were read (fewer over the budget). */
  frames: number;
  bitmaps: ImageBitmap[];
  delays: number[];
};

export type ImportProgress = { done: number; total: number; kind: MediaKind | null };

let worker: Worker | null = null;
let nextId = 1;
const handlers = new Map<number, (m: WorkerOut) => void>();

function getWorker(): Worker {
  if (worker) return worker;
  worker = new Worker(new URL('./media.worker.ts', import.meta.url), { type: 'module' });
  worker.onmessage = (e: MessageEvent<WorkerOut>) => handlers.get(e.data.id)?.(e.data);
  worker.onerror = (e) => {
    e.preventDefault();
    const w = worker;
    worker = null;
    w?.terminate();
    for (const [id, h] of handlers) h({ type: 'error', id, code: 'failed', message: 'The editor’s worker stopped. Try again.' });
  };
  return worker;
}

const send = (m: WorkerIn, transfer: Transferable[] = []) => getWorker().postMessage(m, transfer);

/** Whether this browser encodes WebP (Chrome, Edge, Firefox: yes; Safari: no, GIF instead). */
let webp: Promise<boolean> | null = null;
export function canEncodeWebp(): Promise<boolean> {
  webp ??= new Promise((resolve) => {
    const id = nextId++;
    handlers.set(id, (m) => {
      handlers.delete(id);
      resolve(m.type === 'probe' && m.webp);
    });
    try {
      send({ type: 'probe', id });
    } catch {
      resolve(false);
    }
  });
  return webp;
}

/** Decodes `file` into frames, telling `progress` as it goes. */
export async function importFile(file: File, budget: Budget, progress: (p: ImportProgress) => void, signal: AbortSignal): Promise<Imported> {
  const head = new Uint8Array(await file.slice(0, 64).arrayBuffer());
  if (signal.aborted) throw new Cancelled();
  const kind = sniff(head);
  if (kind === 'unknown') throw new Error('That file isn’t a picture or video the editor can open.');
  if (kind === 'webm') return videoFrames(file, kind, budget, progress, signal);
  try {
    return await viaWorker(file, budget, progress, signal);
  } catch (e) {
    if (e instanceof FallbackToVideo) return videoFrames(file, kind, budget, progress, signal);
    throw e;
  }
}

class FallbackToVideo extends Error {}

function viaWorker(file: File, budget: Budget, progress: (p: ImportProgress) => void, signal: AbortSignal): Promise<Imported> {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    const bitmaps: ImageBitmap[] = [];
    const delays: number[] = [];
    const stamps: number[] = [];
    let info: Extract<WorkerOut, { type: 'meta' }> | null = null;
    const stop = () => {
      handlers.delete(id);
      signal.removeEventListener('abort', abort);
    };
    const abort = () => {
      stop();
      worker?.postMessage({ type: 'cancel', id } satisfies WorkerIn);
      for (const b of bitmaps) b.close();
      reject(new Cancelled());
    };
    signal.addEventListener('abort', abort, { once: true });
    handlers.set(id, (m) => {
      if (m.type === 'meta') {
        info = m;
        progress({ done: 0, total: m.reading, kind: m.kind });
      } else if (m.type === 'frame') {
        bitmaps[m.index] = m.bitmap;
        if (m.ts !== undefined) stamps[m.index] = m.ts;
        else delays[m.index] = m.delay ?? 0;
        progress({ done: m.index + 1, total: info?.reading ?? m.index + 1, kind: info?.kind ?? null });
      } else if (m.type === 'done') {
        stop();
        if (!info || !bitmaps.length) return reject(new Error('That file has no pictures in it.'));
        const n = bitmaps.length;
        // A video's delays come from its frames' times; a picture's are its own.
        const d = m.times ? delaysFromTimes(stamps.length === n ? stamps : m.times.slice(0, n), m.end ?? 0) : delays.map((x) => shownDelay(x, n));
        resolve({ kind: info.kind, width: info.width, height: info.height, scaledWidth: info.scaledWidth, scaledHeight: info.scaledHeight, frames: info.frames, bitmaps, delays: d });
      } else if (m.type === 'error') {
        stop();
        for (const b of bitmaps) b.close();
        reject(m.code === 'video-fallback' ? new FallbackToVideo() : new Error(m.message));
      }
    });
    send({ type: 'decode', id, file, budget });
  });
}

// ---- Video in a <video> ----

const once = (el: EventTarget, ok: string, signal: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const done = () => {
      el.removeEventListener(ok, done);
      el.removeEventListener('error', fail);
      resolve();
    };
    const fail = () => {
      el.removeEventListener(ok, done);
      reject(new Error('This browser can’t play that video. Try an MP4 (H.264).'));
    };
    el.addEventListener(ok, done, { once: true });
    el.addEventListener('error', fail, { once: true });
    signal.addEventListener('abort', () => reject(new Cancelled()), { once: true });
  });

type FrameCallbackVideo = HTMLVideoElement & { requestVideoFrameCallback?: (cb: (now: number, m: { mediaTime: number }) => void) => number };

/** The video's frame rate, from the frames it shows while it plays a moment (30 when it can't tell). */
async function frameRate(v: FrameCallbackVideo, signal: AbortSignal): Promise<number> {
  if (!v.requestVideoFrameCallback) return 30;
  const times: number[] = [];
  await new Promise<void>((resolve) => {
    const timer = setTimeout(resolve, 1500);
    const tick = (_: number, m: { mediaTime: number }) => {
      times.push(m.mediaTime);
      if (times.length >= 12 || signal.aborted) {
        clearTimeout(timer);
        resolve();
      } else v.requestVideoFrameCallback!(tick);
    };
    v.requestVideoFrameCallback!(tick);
    void v.play().catch(() => resolve());
  });
  v.pause();
  const gaps = times
    .slice(1)
    .map((t, i) => t - times[i])
    .filter((g) => g > 0.001)
    .sort((a, b) => a - b);
  if (gaps.length < 3) return 30;
  const fps = 1 / gaps[Math.floor(gaps.length / 2)];
  // Snap to the common rates.
  const common = [12, 15, 23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60];
  const near = common.find((c) => Math.abs(c - fps) / c < 0.04);
  return near ?? Math.min(120, Math.max(1, Math.round(fps)));
}

async function videoFrames(file: File, kind: MediaKind, budget: Budget, progress: (p: ImportProgress) => void, signal: AbortSignal): Promise<Imported> {
  const url = URL.createObjectURL(file);
  const v = document.createElement('video') as FrameCallbackVideo;
  v.muted = true;
  v.playsInline = true;
  v.preload = 'auto';
  v.src = url;
  const bitmaps: ImageBitmap[] = [];
  try {
    await once(v, 'loadeddata', signal);
    let duration = v.duration;
    if (!Number.isFinite(duration)) {
      // Some WebM files say their length only once played to the end.
      v.currentTime = 1e7;
      await once(v, 'seeked', signal);
      duration = v.duration;
      v.currentTime = 0;
      await once(v, 'seeked', signal);
    }
    if (!Number.isFinite(duration) || duration <= 0 || !v.videoWidth) throw new Error('That video has no pictures in it.');
    const fps = await frameRate(v, signal);
    const all = Math.max(1, Math.round(duration * fps));
    const fit = importScale(v.videoWidth, v.videoHeight, all, budget);
    progress({ done: 0, total: fit.frames, kind });
    const canvas = document.createElement('canvas');
    canvas.width = fit.width;
    canvas.height = fit.height;
    const ctx = canvas.getContext('2d')!;
    ctx.imageSmoothingQuality = 'high';
    for (let k = 0; k < fit.frames; k++) {
      if (signal.aborted) throw new Cancelled();
      // The middle of each frame's time, so a seek lands on that frame and not its neighbour.
      v.currentTime = Math.min(duration - 0.0005, (k + 0.5) / fps);
      await once(v, 'seeked', signal);
      ctx.clearRect(0, 0, fit.width, fit.height);
      ctx.drawImage(v, 0, 0, fit.width, fit.height);
      bitmaps.push(await createImageBitmap(canvas));
      progress({ done: k + 1, total: fit.frames, kind });
    }
    const delays = delaysAtFps(fit.frames, fps, fit.frames === all ? duration * 1000 : undefined);
    return { kind, width: v.videoWidth, height: v.videoHeight, scaledWidth: fit.width, scaledHeight: fit.height, frames: all, bitmaps, delays };
  } catch (e) {
    for (const b of bitmaps) b.close();
    throw e;
  } finally {
    v.removeAttribute('src');
    v.load();
    URL.revokeObjectURL(url);
  }
}

// ---- Encoding ----

export type EncodeOptions = { format: 'webp' | 'gif'; quality: number };

/**
 * Encodes `d` (its frames painted from `sources`) into one file. Frames are painted here and
 * encoded in the worker, a few at a time.
 */
export function encodeDoc(d: Doc, sources: readonly ImageBitmap[], opts: EncodeOptions, progress: (done: number, total: number) => void, signal: AbortSignal): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    const o = outSize(d);
    const n = d.frames.length;
    const delays = n === 1 ? [0] : opts.format === 'gif' ? gifDelays(d.frames.map((f) => f.delay)).map((cs) => cs * 10) : d.frames.map((f) => f.delay);
    let acked = 0;
    let sent = 0;
    let wake: (() => void) | null = null;
    const stop = () => {
      handlers.delete(id);
      signal.removeEventListener('abort', abort);
      wake?.();
    };
    const abort = () => {
      stop();
      worker?.postMessage({ type: 'cancel', id } satisfies WorkerIn);
      reject(new Cancelled());
    };
    signal.addEventListener('abort', abort, { once: true });
    handlers.set(id, (m) => {
      if (m.type === 'progress') {
        acked = m.done;
        progress(acked, n);
        wake?.();
      } else if (m.type === 'encoded') {
        stop();
        resolve(m.blob);
      } else if (m.type === 'error') {
        stop();
        reject(new Error(m.code === 'no-webp' ? 'no-webp' : m.message));
      }
    });
    send({ type: 'encode-start', id, format: opts.format, width: o.w, height: o.h, quality: opts.quality, count: n });
    const off = typeof OffscreenCanvas !== 'undefined' ? new OffscreenCanvas(o.w, o.h) : null;
    const el = off ? null : Object.assign(document.createElement('canvas'), { width: o.w, height: o.h });
    const ctx = (off ? off.getContext('2d') : el!.getContext('2d')) as CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
    void (async () => {
      try {
        for (let i = 0; i < n; i++) {
          if (signal.aborted || !handlers.has(id)) return;
          // At most 4 frames waiting in the worker.
          while (sent - acked >= 4 && handlers.has(id)) await new Promise<void>((r) => (wake = r));
          if (!handlers.has(id)) return;
          paintOutput(ctx, d, i, sources[d.frames[i].src]);
          const bitmap = off ? off.transferToImageBitmap() : await createImageBitmap(el!);
          send({ type: 'encode-frame', id, bitmap, delay: delays[i] }, [bitmap]);
          sent++;
          if (i % 4 === 3) await new Promise((r) => setTimeout(r, 0));
        }
        send({ type: 'encode-end', id });
      } catch (e) {
        stop();
        reject(e);
      }
    })();
  });
}
