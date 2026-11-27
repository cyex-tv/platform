/// <reference lib="webworker" />
// The emote editor's worker: decodes pictures and videos into frames and encodes the finished
// frames into an animated WebP (or a GIF), so the page stays responsive while it works.
//   decode  GIF, PNG/APNG, JPEG, WebP (animated too), AVIF: WebCodecs' ImageDecoder where the
//           browser has it; otherwise GIFs with gifuct-js, animated WebPs frame by frame
//           (riff.ts), everything else as a still. MP4: mp4box.js reads the samples and
//           VideoDecoder decodes them. A video this can't do is handed back to the page
//           (code 'video-fallback'), which plays it in a <video> instead.
//           Frames are scaled down while decoding when the whole would be over the budget.
//   encode  each frame through the canvas's own WebP encoder, put together into one animated
//           WebP (riff.ts); or a GIF with gifenc.
import { parseGIF, decompressFrame } from 'gifuct-js';
import { GIFEncoder, applyPalette, quantize } from 'gifenc';
import { createFile, DataStream, MP4BoxBuffer, type Movie, type Sample } from 'mp4box';
import { muxAnimation, pictureChunks, readAnimation, stillWebp, type MuxFrame } from './riff';
import { MIME, importScale, sniff, type Budget, type MediaKind } from './sniff';
import type { WorkerIn, WorkerOut } from './media-protocol';
import { Failure, assertCanvasSize, assertDecodable, assertFileSize, assertWebp, readGif } from './guard';

declare const self: DedicatedWorkerGlobalScope;

const cancelled = new Set<number>();
const post = (m: WorkerOut, transfer: Transferable[] = []) => self.postMessage(m, transfer);

class Cancel extends Error {}
const check = (id: number) => {
  if (cancelled.has(id)) throw new Cancel();
};

/** Scales a picture into a reused canvas and hands it out as a bitmap. */
class Scaler {
  canvas: OffscreenCanvas;
  ctx: OffscreenCanvasRenderingContext2D;
  constructor(
    readonly w: number,
    readonly h: number,
  ) {
    assertCanvasSize(w, h, 120_000_000);
    this.canvas = new OffscreenCanvas(w, h);
    this.ctx = this.canvas.getContext('2d')!;
    this.ctx.imageSmoothingQuality = 'high';
  }
  bitmap(src: CanvasImageSource): ImageBitmap {
    this.ctx.clearRect(0, 0, this.w, this.h);
    this.ctx.drawImage(src, 0, 0, this.w, this.h);
    return this.canvas.transferToImageBitmap();
  }
}

type Sink = { id: number; budget: Budget };

function meta(s: Sink, kind: MediaKind, w: number, h: number, frames: number) {
  const fit = importScale(w, h, frames, s.budget);
  post({ type: 'meta', id: s.id, kind, width: w, height: h, frames, scaledWidth: fit.width, scaledHeight: fit.height, reading: fit.frames });
  return fit;
}

async function decode(id: number, file: Blob, budget: Budget) {
  assertFileSize(file.size);
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  const kind = sniff(bytes);
  const s: Sink = { id, budget };
  if (kind === 'unknown') throw new Failure('That file isn’t a picture or video the editor can open.');
  if (kind === 'webm') return post({ type: 'error', id, code: 'video-fallback', message: '' });
  if (kind === 'mp4') return decodeMp4(s, buf);
  const mime = MIME[kind];
  if (typeof ImageDecoder !== 'undefined') {
    let supported = false;
    try {
      supported = await ImageDecoder.isTypeSupported(mime);
    } catch {
      supported = false;
    }
    if (supported) {
      try {
        return await viaImageDecoder(s, kind, buf, mime);
      } catch (e) {
        if (e instanceof Cancel || e instanceof Failure) throw e;
        // fall through to the other ways
      }
    }
  }
  if (kind === 'gif') return viaGifuct(s, buf);
  if (kind === 'webp') {
    const anim = readAnimation(bytes);
    if (anim) return viaWebpFrames(s, anim);
  }
  return still(s, kind, file);
}

async function viaImageDecoder(s: Sink, kind: MediaKind, buf: ArrayBuffer, type: string) {
  const dec = new ImageDecoder({ data: buf, type });
  try {
    await dec.tracks.ready;
    await dec.completed;
    const track = dec.tracks.selectedTrack;
    const n = Math.max(1, track?.frameCount ?? 1);
    const first = await dec.decode({ frameIndex: 0 });
    const w = first.image.displayWidth;
    const h = first.image.displayHeight;
    assertDecodable(w, h, n, s.budget);
    const fit = meta(s, kind, w, h, n);
    const sc = new Scaler(fit.width, fit.height);
    for (let i = 0; i < fit.frames; i++) {
      check(s.id);
      const { image } = i === 0 ? first : await dec.decode({ frameIndex: i, completeFramesOnly: true });
      const delay = (image.duration ?? 0) / 1000;
      const bitmap = sc.bitmap(image);
      image.close();
      post({ type: 'frame', id: s.id, index: i, bitmap, delay }, [bitmap]);
    }
    post({ type: 'done', id: s.id });
  } finally {
    dec.close();
  }
}

async function viaGifuct(s: Sink, buf: ArrayBuffer) {
  const { gif, frames, width: w, height: h } = readGif(buf, s.budget);
  const fit = meta(s, 'gif', w, h, frames.length);
  const comp = new OffscreenCanvas(w, h);
  const ctx = comp.getContext('2d')!;
  const patch = new OffscreenCanvas(1, 1);
  const pctx = patch.getContext('2d')!;
  const sc = new Scaler(fit.width, fit.height);
  let saved: ImageData | null = null;
  let prev: { disposalType?: number; dims: { left: number; top: number; width: number; height: number } } | undefined;
  for (let i = 0; i < fit.frames; i++) {
    check(s.id);
    const f = decompressFrame(frames[i], gif.gct, true);
    if (prev?.disposalType === 2) ctx.clearRect(prev.dims.left, prev.dims.top, prev.dims.width, prev.dims.height);
    else if (prev?.disposalType === 3 && saved) ctx.putImageData(saved, 0, 0);
    saved = f.disposalType === 3 ? ctx.getImageData(0, 0, w, h) : null;
    const { width, height, left, top } = f.dims;
    if (width && height) {
      patch.width = width;
      patch.height = height;
      pctx.putImageData(new ImageData(new Uint8ClampedArray(f.patch), width, height), 0, 0);
      ctx.drawImage(patch, left, top);
    }
    const bitmap = sc.bitmap(comp);
    post({ type: 'frame', id: s.id, index: i, bitmap, delay: f.delay }, [bitmap]);
    prev = f;
    if (i % 8 === 7) await idle();
  }
  post({ type: 'done', id: s.id });
}

async function viaWebpFrames(s: Sink, anim: NonNullable<ReturnType<typeof readAnimation>>) {
  const { width: w, height: h } = anim;
  assertWebp(anim, s.budget);
  const fit = meta(s, 'webp', w, h, anim.frames.length);
  const comp = new OffscreenCanvas(w, h);
  const ctx = comp.getContext('2d')!;
  const sc = new Scaler(fit.width, fit.height);
  for (let i = 0; i < fit.frames; i++) {
    check(s.id);
    const f = anim.frames[i];
    const prev = anim.frames[i - 1];
    if (prev?.dispose) ctx.clearRect(prev.x, prev.y, prev.w, prev.h);
    const pic = await createImageBitmap(new Blob([stillWebp(f.chunks, f.w, f.h) as BlobPart], { type: 'image/webp' }));
    if (!f.blend) ctx.clearRect(f.x, f.y, f.w, f.h);
    ctx.drawImage(pic, f.x, f.y);
    pic.close();
    const bitmap = sc.bitmap(comp);
    post({ type: 'frame', id: s.id, index: i, bitmap, delay: f.delay }, [bitmap]);
  }
  post({ type: 'done', id: s.id });
}

async function still(s: Sink, kind: MediaKind, file: Blob) {
  let pic: ImageBitmap;
  try {
    pic = await createImageBitmap(file);
  } catch {
    throw new Failure(kind === 'avif' ? 'This browser can’t open AVIF pictures. Save it as PNG or WebP first.' : 'That file doesn’t open as a picture.');
  }
  const fit = meta(s, kind, pic.width, pic.height, 1);
  const bitmap = fit.scale === 1 ? pic : new Scaler(fit.width, fit.height).bitmap(pic);
  if (bitmap !== pic) pic.close();
  post({ type: 'frame', id: s.id, index: 0, bitmap, delay: 0 }, [bitmap]);
  post({ type: 'done', id: s.id });
}

const idle = () => new Promise((r) => setTimeout(r, 0));

// ---- MP4 ----

type Box = { write(stream: DataStream): void };

function description(file: ReturnType<typeof createFile>, trackId: number): Uint8Array | undefined {
  const trak = file.getTrackById(trackId) as unknown as { mdia: { minf: { stbl: { stsd: { entries: Record<string, Box | undefined>[] } } } } };
  for (const entry of trak.mdia.minf.stbl.stsd.entries) {
    const box = entry.avcC ?? entry.hvcC ?? entry.vpcC ?? entry.av1C;
    if (box) {
      const stream = new DataStream(undefined, 0, 1 /* big endian */);
      box.write(stream);
      return new Uint8Array(stream.buffer as ArrayBuffer, 8); // without the box header
    }
  }
  return undefined;
}

async function decodeMp4(s: Sink, buf: ArrayBuffer) {
  const fallback = () => post({ type: 'error', id: s.id, code: 'video-fallback', message: '' });
  if (typeof VideoDecoder === 'undefined') return fallback();
  const file = createFile();
  let info: Movie | null = null;
  const samples: Sample[] = [];
  let failed: unknown = null;
  file.onError = (e: unknown) => (failed = e);
  file.onReady = (i: Movie) => {
    info = i;
    const t = i.videoTracks[0];
    if (!t) return;
    file.setExtractionOptions(t.id, null, { nbSamples: Number.MAX_SAFE_INTEGER });
    file.start();
  };
  file.onSamples = (_id: number, _user: unknown, got: Sample[]) => {
    for (const x of got) samples.push(x);
  };
  file.appendBuffer(MP4BoxBuffer.fromArrayBuffer(buf, 0), true);
  file.flush();
  const movie = info as Movie | null;
  if (failed || !movie) return fallback();
  const track = movie.videoTracks[0];
  if (!track) throw new Failure('That video has no picture track.');
  const config: VideoDecoderConfig = {
    codec: track.codec.startsWith('vp08') ? 'vp8' : track.codec,
    codedWidth: track.video?.width ?? track.track_width,
    codedHeight: track.video?.height ?? track.track_height,
    description: description(file, track.id),
  };
  let ok = false;
  try {
    ok = !!(await VideoDecoder.isConfigSupported(config)).supported;
  } catch {
    ok = false;
  }
  if (!ok || !samples.length) return fallback();

  const scale = 1_000_000 / track.timescale;
  const times = samples.map((x) => x.cts * scale).sort((a, b) => a - b);
  let end = 0;
  for (const x of samples) end = Math.max(end, (x.cts + x.duration) * scale);
  const w = config.codedWidth!;
  const h = config.codedHeight!;
  assertDecodable(w, h, samples.length, s.budget);
  const fit = meta(s, 'mp4', w, h, samples.length);
  // Only the first frames (in showing order) when there are more than the budget's.
  const cutoff = fit.frames < times.length ? times[fit.frames] : Infinity;
  const sc = new Scaler(fit.width, fit.height);
  let index = 0;
  let error: unknown = null;
  const decoder = new VideoDecoder({
    output: (frame) => {
      if (frame.timestamp >= cutoff - 0.5 || cancelled.has(s.id)) {
        frame.close();
        return;
      }
      const bitmap = sc.bitmap(frame);
      post({ type: 'frame', id: s.id, index: index++, bitmap, ts: frame.timestamp }, [bitmap]);
      frame.close();
    },
    error: (e) => (error = e),
  });
  decoder.configure(config);
  const until = cutoff === Infinity ? Infinity : cutoff;
  for (const x of samples) {
    check(s.id);
    if (error) break;
    // Past the cutoff, a frame can only matter as a reference for earlier ones: one GOP more.
    if (x.cts * scale >= until && x.is_sync) break;
    while (decoder.decodeQueueSize > 12) await idle();
    decoder.decode(new EncodedVideoChunk({ type: x.is_sync ? 'key' : 'delta', timestamp: x.cts * scale, duration: x.duration * scale, data: x.data! }));
  }
  if (!error) await decoder.flush().catch((e) => (error = e));
  decoder.close();
  if (error && !index) return fallback();
  post({ type: 'done', id: s.id, times: times.slice(0, fit.frames), end: cutoff === Infinity ? end : cutoff });
}

// ---- Encoding ----

type Encoding = {
  format: 'webp' | 'gif';
  w: number;
  h: number;
  quality: number;
  count: number;
  canvas: OffscreenCanvas;
  ctx: OffscreenCanvasRenderingContext2D;
  frames: MuxFrame[];
  still: Blob | null;
  gif: ReturnType<typeof GIFEncoder> | null;
  done: number;
};
const encodings = new Map<number, Encoding>();

async function encodeFrame(id: number, bitmap: ImageBitmap, delay: number) {
  const e = encodings.get(id);
  if (!e) return bitmap.close();
  e.ctx.clearRect(0, 0, e.w, e.h);
  e.ctx.drawImage(bitmap, 0, 0);
  bitmap.close();
  if (e.format === 'webp') {
    const blob = await e.canvas.convertToBlob({ type: 'image/webp', quality: e.quality });
    if (blob.type !== 'image/webp') throw new Failure('no-webp');
    if (e.count === 1) e.still = blob;
    else e.frames.push({ chunks: pictureChunks(new Uint8Array(await blob.arrayBuffer())), w: e.w, h: e.h, delay });
  } else {
    const data = e.ctx.getImageData(0, 0, e.w, e.h).data;
    const palette = quantize(data, 256, { format: 'rgba4444', oneBitAlpha: true });
    const index = applyPalette(data, palette, 'rgba4444');
    const transparentIndex = palette.findIndex((p) => p[3] === 0);
    e.gif!.writeFrame(index, e.w, e.h, { palette, delay, repeat: 0, transparent: transparentIndex >= 0, transparentIndex: Math.max(0, transparentIndex) });
  }
  e.done++;
}

function finish(id: number) {
  const e = encodings.get(id);
  encodings.delete(id);
  if (!e) return;
  let blob: Blob;
  if (e.format === 'webp') blob = e.still ?? new Blob([muxAnimation(e.w, e.h, e.frames) as BlobPart], { type: 'image/webp' });
  else {
    e.gif!.finish();
    blob = new Blob([e.gif!.bytes() as BlobPart], { type: 'image/gif' });
  }
  post({ type: 'encoded', id, blob });
}

async function canWebp(): Promise<boolean> {
  try {
    const c = new OffscreenCanvas(2, 2);
    c.getContext('2d')!.fillRect(0, 0, 1, 1);
    return (await c.convertToBlob({ type: 'image/webp' })).type === 'image/webp';
  } catch {
    return false;
  }
}

// One job at a time per id; frames of an encoding are handled in order.
let queue: Promise<unknown> = Promise.resolve();

self.onmessage = (ev: MessageEvent<WorkerIn>) => {
  const m = ev.data;
  if (m.type === 'cancel') {
    cancelled.add(m.id);
    encodings.delete(m.id);
    return;
  }
  const run = async () => {
    try {
      if (m.type === 'probe') post({ type: 'probe', id: m.id, webp: await canWebp() });
      else if (m.type === 'decode') await decode(m.id, m.file, m.budget);
      else if (m.type === 'encode-start') {
        const canvas = new OffscreenCanvas(m.width, m.height);
        const ctx = canvas.getContext('2d', { willReadFrequently: m.format === 'gif' })!;
        encodings.set(m.id, {
          format: m.format,
          w: m.width,
          h: m.height,
          quality: m.quality,
          count: m.count,
          canvas,
          ctx,
          frames: [],
          still: null,
          gif: m.format === 'gif' ? GIFEncoder() : null,
          done: 0,
        });
      } else if (m.type === 'encode-frame') {
        if (cancelled.has(m.id)) return m.bitmap.close();
        await encodeFrame(m.id, m.bitmap, m.delay);
        post({ type: 'progress', id: m.id, done: encodings.get(m.id)?.done ?? 0 });
      } else if (m.type === 'encode-end') {
        if (!cancelled.has(m.id)) finish(m.id);
      }
    } catch (e) {
      encodings.delete(m.id);
      if (e instanceof Cancel) return;
      post({ type: 'error', id: m.id, code: e instanceof Failure && e.message === 'no-webp' ? 'no-webp' : 'failed', message: e instanceof Failure ? e.message : 'The editor couldn’t read that file.' });
    }
  };
  queue = queue.then(run, run);
};
