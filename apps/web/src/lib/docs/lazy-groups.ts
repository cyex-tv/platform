// Draws a closed group's body on demand. initDocs (behaviour.ts) calls `load(id)` for every group
// that is open, and when a link or the address points into one; this mounts GroupBody into the
// group's (empty, server-rendered) body and resolves when its sections are in the DOM.
import { mount, unmount } from 'svelte';
import GroupBody from './GroupBody.svelte';
import { isLazy } from './lazy';

export function lazyGroups(root: ParentNode, known: string[], base: string, assets = '') {
  const started = new Map<string, Promise<void>>();
  const mounted: ReturnType<typeof mount>[] = [];
  return {
    load(id: string): Promise<void> {
      if (!isLazy(id)) return Promise.resolve();
      const target = root.querySelector<HTMLElement>(`#body-${CSS.escape(id)}[data-lazy] > .collapse-inner`);
      if (!target) return Promise.resolve();
      let p = started.get(id);
      if (!p) {
        p = new Promise<void>((resolve) => {
          mounted.push(mount(GroupBody, { target, props: { id, known, base, assets, onready: resolve } }));
        });
        started.set(id, p);
      }
      return p;
    },
    destroy() {
      for (const m of mounted) unmount(m);
      mounted.length = 0;
      started.clear();
    },
  };
}
