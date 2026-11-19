// Whether this device gets the 3D logo, or keeps the static one. Pure over what it's given, so
// it's tested without a browser; `readEnv` reads the real values once the page has settled.

export type Env = {
  reducedMotion: boolean;
  saveData: boolean;
  /** navigator.deviceMemory in GB, where the browser tells (Chromium only). */
  memory: number | undefined;
  /** navigator.hardwareConcurrency, where the browser tells. */
  cores: number | undefined;
  webgl: boolean;
};

export function capable(env: Env): boolean {
  if (env.reducedMotion || env.saveData) return false;
  if (env.memory !== undefined && env.memory < 4) return false;
  if (env.cores !== undefined && env.cores < 4) return false;
  return env.webgl; // read last: it's the one check that costs something
}

/** WebGL 2 or 1 without a software fallback; the probe context is given back at once. */
function hasWebgl(): boolean {
  try {
    const c = document.createElement('canvas');
    const opts = { failIfMajorPerformanceCaveat: true };
    const gl = (c.getContext('webgl2', opts) ?? c.getContext('webgl', opts)) as WebGLRenderingContext | null;
    if (!gl) return false;
    gl.getExtension('WEBGL_lose_context')?.loseContext();
    return true;
  } catch {
    return false;
  }
}

/**
 * Whether the reader wants things still: the system setting, or the site's own (a
 * `data-motion="reduce"` attribute on <html>, where the site sets one). Read now.
 */
export function motionReduced(): boolean {
  return document.documentElement.dataset.motion === 'reduce' || matchMedia('(prefers-reduced-motion: reduce)').matches;
}

type Nav = Navigator & { connection?: { saveData?: boolean }; deviceMemory?: number };

export function readEnv(): Env {
  const nav = navigator as Nav;
  return {
    reducedMotion: motionReduced(),
    saveData: nav.connection?.saveData === true,
    memory: typeof nav.deviceMemory === 'number' ? nav.deviceMemory : undefined,
    cores: typeof nav.hardwareConcurrency === 'number' && nav.hardwareConcurrency > 0 ? nav.hardwareConcurrency : undefined,
    // Last, so a reader who said no in a cheaper way never pays for a probe context.
    get webgl() {
      return hasWebgl();
    },
  };
}
