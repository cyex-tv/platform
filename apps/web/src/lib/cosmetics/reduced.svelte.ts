// Whether the reader asked for reduced motion, for image paints and badges: they switch to their
// still first frame while it is on. The one source is lib/media.svelte (the OS's setting, or
// Settings → Appearance), so there is nothing to start: read `reduced.on`.
import { reducedMotion } from '$lib/media.svelte';

export const reduced = {
  get on(): boolean {
    return reducedMotion.matches;
  },
};
