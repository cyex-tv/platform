// The render worker: the engine renders and encodes here so the page never stalls.
import PL from './engine/index.js';
import { runJob, type JobIn, type JobOut } from './render-job';

const scope = self as unknown as { postMessage(m: JobOut): void; onmessage: ((e: MessageEvent) => void) | null };
const cancelled = new Set<number>();
// A MessageChannel yields a task between frames (nested setTimeout would clamp to 4 ms).
const waiters: (() => void)[] = [];
const channel = new MessageChannel();
channel.port1.onmessage = () => waiters.shift()?.();
const pause = () =>
  new Promise<void>((resolve) => {
    waiters.push(resolve);
    channel.port2.postMessage(0);
  });

scope.onmessage = (e: MessageEvent) => {
  const m = e.data as ({ type: 'render' } & JobIn) | { type: 'cancel'; id: number };
  if (m.type === 'cancel') {
    cancelled.add(m.id);
    return;
  }
  if (m.type === 'render')
    void runJob(PL, m, (out) => scope.postMessage(out), () => cancelled.has(m.id), pause).finally(() => cancelled.delete(m.id));
};
