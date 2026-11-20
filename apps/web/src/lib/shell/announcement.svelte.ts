// The live site announcement, asked once per page load (the API shares it for a few seconds), and
// which ones this device closed (localStorage, per announcement). A failed read shows nothing: the
// bar must never complain.
import { gql } from '$lib/api';
import type { AnnouncementFieldsFragment } from '$lib/gql/graphql';
import { ACTIVE_ANNOUNCEMENT } from '$lib/queries';
import { DISMISS_KEY, readClosed, withClosed } from './announcement';

export type Announcement = AnnouncementFieldsFragment;

class Banner {
  current = $state<Announcement | null>(null);
  closed = $state<string[]>([]);
  #asked = false;

  /** Reads the closed list and the live announcement; call from an effect (the browser only). */
  load(force = false) {
    try {
      this.closed = readClosed(localStorage.getItem(DISMISS_KEY));
    } catch {
      this.closed = [];
    }
    if (this.#asked && !force) return;
    this.#asked = true;
    gql(ACTIVE_ANNOUNCEMENT, {})
      .then((r) => (this.current = r.activeAnnouncement ?? null))
      .catch(() => {});
  }

  /** Shown: there is one and this device hasn't closed it. */
  get shown(): Announcement | null {
    const a = this.current;
    return a && !this.closed.includes(a.id) ? a : null;
  }

  close(id: string) {
    this.closed = withClosed(this.closed, id);
    try {
      localStorage.setItem(DISMISS_KEY, JSON.stringify(this.closed));
    } catch {
      /* not remembered: it comes back next visit */
    }
  }
}

export const banner = new Banner();
