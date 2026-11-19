// Saving to cyex.tv: the paints catalog, createPaint / updatePaint, and paint pictures
// (POST /v1/cosmetics/images kind=paint, then cosmeticImage until its files are made).
// The studio talks to an interface, so the demo page and tests can run it without the API.
import { ApiError, gql, uploadCosmeticImage } from '$lib/api';
import { waitReady } from '$lib/cosmetics/upload';
import type { PaintInput, StudioPaintFieldsFragment } from '$lib/gql/graphql';
import type { StudioPaint } from './format';
import { STUDIO_COSMETIC_IMAGE, STUDIO_CREATE_PAINT, STUDIO_PAINTS, STUDIO_UPDATE_PAINT } from './queries';

/** `still` is the picture's first frame, for previews under reduced motion. */
export type SavedPaint = { id: string; paint: StudioPaint; updatedAt: string; still: string | null };
export type Picture = { id: string; url: string | null; still: string | null };
export type UploadState = 'uploading' | 'processing' | 'ready';

export interface StudioBackend {
  listPaints(): Promise<SavedPaint[]>;
  /** Creates the paint (`id` null) or replaces it. */
  save(paint: StudioPaint, id: string | null): Promise<SavedPaint>;
  /** Uploads a paint picture and waits until its files are made. */
  uploadPicture(file: File, onState?: (s: UploadState) => void, signal?: AbortSignal): Promise<Picture>;
}

type ImageFields = NonNullable<StudioPaintFieldsFragment['image']>;

/** The address of the picture to preview: the largest WebP (animated, or its still first frame), else any file. */
export function pictureUrl(img: Pick<ImageFields, 'host' | 'files'> | null | undefined, still = false): string | null {
  if (!img?.files?.length) return null;
  const want = img.files.filter((f) => f.isStatic === still);
  const pool = want.length ? want : img.files;
  const webp = pool.filter((f) => f.format === 'WEBP');
  const pick = [...(webp.length ? webp : pool)].sort((a, b) => b.scale - a.scale)[0];
  return `${img.host.replace(/\/+$/, '')}/${pick.name}`;
}

/** An API paint as the studio's model. */
export function fromApi(p: StudioPaintFieldsFragment): SavedPaint {
  return {
    id: p.id,
    updatedAt: p.updatedAt,
    still: p.image ? pictureUrl(p.image, true) : null,
    paint: {
      name: p.name,
      kind: p.kind,
      angle: p.angle,
      repeat: p.repeat,
      shape: p.shape,
      color: p.color ?? null,
      stops: p.stops.map((s) => ({ at: s.at, color: s.color })),
      shadows: p.shadows.map((s) => ({ x: s.x, y: s.y, radius: s.radius, color: s.color })),
      image: p.image ? { id: p.image.id, url: pictureUrl(p.image) } : null,
    },
  };
}

/** The studio's paint as `PaintInput` (a full replacement, as updatePaint wants). */
export function toInput(p: StudioPaint): PaintInput {
  const image = p.kind === 'IMAGE';
  return {
    name: p.name.trim(),
    kind: p.kind,
    angle: p.kind === 'LINEAR_GRADIENT' ? Math.round(p.angle) : 90,
    repeat: p.repeat,
    shape: p.shape,
    color: p.color ?? null,
    stops: image ? [] : p.stops.map((s) => ({ at: s.at, color: s.color })),
    shadows: p.shadows.map((s) => ({ x: s.x, y: s.y, radius: s.radius, color: s.color })),
    imageId: image ? (p.image?.id ?? null) : null,
  };
}

const sleep = (ms: number, signal?: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const t = setTimeout(resolve, ms);
    signal?.addEventListener('abort', () => (clearTimeout(t), reject(new ApiError('Stopped.', 'aborted'))), { once: true });
  });

/** The live API. */
export function apiBackend(opts: { pollMs?: number; timeoutMs?: number } = {}): StudioBackend {
  const pollMs = opts.pollMs ?? 1000;
  const timeoutMs = opts.timeoutMs ?? 120_000;
  return {
    async listPaints() {
      const { cosmetics } = await gql(STUDIO_PAINTS, {});
      return cosmetics.paints.map(fromApi);
    },
    async save(paint, id) {
      const input = toInput(paint);
      const saved = id ? (await gql(STUDIO_UPDATE_PAINT, { id, input })).updatePaint : (await gql(STUDIO_CREATE_PAINT, { input })).createPaint;
      return fromApi(saved);
    },
    async uploadPicture(file, onState, signal) {
      onState?.('uploading');
      // The POST answers in the REST shape (and, for bytes cyex.tv has already, ready at once):
      // only its ID is used, and the picture is read back through GraphQL, as the site renders it.
      const up = await uploadCosmeticImage(file, 'paint', signal);
      onState?.('processing');
      const img = await waitReady(async () => (await gql(STUDIO_COSMETIC_IMAGE, { id: up.id })).cosmeticImage, {
        everyMs: pollMs,
        timeoutMs,
        signal,
        sleep: (ms) => sleep(ms, signal),
      });
      onState?.('ready');
      return { id: img.id, url: pictureUrl(img), still: pictureUrl(img, true) };
    },
  };
}
