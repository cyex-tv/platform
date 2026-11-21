// How much the hero logo draws: everything (bloom), or a lighter step without the bloom pass (a
// glow drawn by the bar itself instead). Low-power devices start on the light step; any device
// steps down for good once its frames run long.

export type Quality = 'high' | 'low';

export type Device = {
  /** navigator.deviceMemory in GB, where the browser tells. */
  memory: number | undefined;
  /** navigator.hardwareConcurrency, where the browser tells. */
  cores: number | undefined;
  /** The page asked for the light step (`?logoquality=low`, for testing). */
  forceLow?: boolean;
};

export function startQuality(d: Device): Quality {
  if (d.forceLow) return 'low';
  if (d.memory !== undefined && d.memory < 8) return 'low';
  if (d.cores !== undefined && d.cores < 6) return 'low';
  return 'high';
}

/**
 * Watches frame times (ms between drawn frames). After `warmup` frames, once the mean of the last
 * `window` frames is over `limit`, it says to step down (once).
 */
export function frameBudget({ limit = 24, window = 60, warmup = 30 } = {}) {
  const times: number[] = [];
  let seen = 0;
  let sum = 0;
  let tripped = false;
  return {
    add(ms: number): boolean {
      if (tripped) return false;
      seen++;
      if (seen <= warmup) return false;
      times.push(ms);
      sum += ms;
      if (times.length > window) sum -= times.shift()!;
      if (times.length === window && sum / window > limit) {
        tripped = true;
        return true;
      }
      return false;
    },
  };
}
