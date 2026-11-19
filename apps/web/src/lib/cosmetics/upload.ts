// Badge and paint pictures before and after `POST /v1/cosmetics/images`: the API's rules checked
// on the page first (so a wrong file says why at once instead of after an upload), and the wait
// until the worker has made the files, read back through GraphQL every time (an upload of bytes
// the API already has answers at once with ready set, in the REST shape, which is not the one the
// site renders from).
import { ApiError } from '$lib/api';

export type CosmeticUploadKind = 'badge' | 'paint';

/** What the API takes (crates/media and cosmetics::check_dimensions). */
export const PICTURE_RULES = {
  types: ['image/png', 'image/apng', 'image/gif', 'image/webp'],
  maxBytes: 5 * 1024 * 1024,
  badgeMinSide: 72,
  paintMinHeight: 64,
} as const;

/** The `accept` list for a picture input. */
export const PICTURE_ACCEPT = PICTURE_RULES.types.join(',');

const mb = (n: number) => (n / 1024 / 1024).toFixed(1).replace('.', ',');

/** Why a file can't be a badge or paint picture (type and size); null when it can be sent. */
export function checkFile(file: Pick<File, 'type' | 'size'>): string | null {
  if (!(PICTURE_RULES.types as readonly string[]).includes(file.type)) return 'Choose a PNG, APNG, GIF or WebP file.';
  if (file.size > PICTURE_RULES.maxBytes) return `Pictures are at most 5 MB (this one is ${mb(file.size)} MB).`;
  return null;
}

/** Why a picture of this size can't be used for `kind`; null when it can. */
export function checkSize(kind: CosmeticUploadKind, width: number, height: number): string | null {
  if (kind === 'badge' && !(width === height && width >= PICTURE_RULES.badgeMinSide))
    return `Badge pictures are square and at least 72 × 72 px (this one is ${width} × ${height} px).`;
  if (kind === 'paint' && height < PICTURE_RULES.paintMinHeight) return `Paint pictures are at least 64 px tall (this one is ${height} px).`;
  return null;
}

/** Codes that asking again won't fix: stop waiting and say so. */
const FINAL = new Set(['signed_out', 'forbidden', 'bad_origin', 'not_found', 'aborted']);

/**
 * Asks `read(id)` until the picture is ready (every `everyMs`, at most `timeoutMs`), and returns
 * it. A network blip is asked again; signed out, forbidden or gone end the wait with that reason.
 */
export async function waitReady<T extends { ready: boolean }>(
  read: () => Promise<T | null | undefined>,
  opts: { everyMs?: number; timeoutMs?: number; signal?: AbortSignal; sleep?: (ms: number) => Promise<void> } = {},
): Promise<T> {
  const every = opts.everyMs ?? 1500;
  const until = Date.now() + (opts.timeoutMs ?? 120_000);
  const sleep = opts.sleep ?? ((ms: number) => new Promise<void>((r) => setTimeout(r, ms)));
  for (;;) {
    if (opts.signal?.aborted) throw new ApiError('Stopped.', 'aborted');
    try {
      const img = await read();
      if (!img) throw new ApiError('The uploaded picture is gone. Try again.', 'not_found');
      if (img.ready) return img;
    } catch (e) {
      if (e instanceof ApiError && FINAL.has(e.code)) throw e;
    }
    if (Date.now() >= until) throw new ApiError('The picture is taking long to process. Try again in a minute.', 'image_not_ready');
    await sleep(every);
  }
}
