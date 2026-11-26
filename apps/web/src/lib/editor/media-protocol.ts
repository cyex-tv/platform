// The messages between the editor (media.ts) and its worker (media.worker.ts).
import type { Budget, MediaKind } from './sniff';

export type WorkerIn =
  | { type: 'probe'; id: number }
  | { type: 'decode'; id: number; file: Blob; budget: Budget }
  | { type: 'cancel'; id: number }
  | { type: 'encode-start'; id: number; format: 'webp' | 'gif'; width: number; height: number; quality: number; count: number }
  | { type: 'encode-frame'; id: number; bitmap: ImageBitmap; delay: number }
  | { type: 'encode-end'; id: number };

export type WorkerOut =
  | { type: 'probe'; id: number; webp: boolean }
  | {
      type: 'meta';
      id: number;
      kind: MediaKind;
      width: number;
      height: number;
      frames: number;
      scaledWidth: number;
      scaledHeight: number;
      /** Frames that will be read (fewer than `frames` over the budget). */
      reading: number;
    }
  /** A frame: `delay` in ms (pictures) or `ts` in µs (video; delays come from `done.times`). */
  | { type: 'frame'; id: number; index: number; bitmap: ImageBitmap; delay?: number; ts?: number }
  | { type: 'done'; id: number; times?: number[]; end?: number }
  | { type: 'progress'; id: number; done: number }
  | { type: 'encoded'; id: number; blob: Blob }
  | { type: 'error'; id: number; code: 'video-fallback' | 'no-webp' | 'failed'; message: string };
