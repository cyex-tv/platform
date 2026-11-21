// The live site announcement and which ones this device closed. The root layout's server load
// reads both (the API's answer and the closed-list cookie), so the page arrives with the bar in
// place or without it and nothing moves. In the browser this keeps what changed since: a
// staff edit asks again (`load(true)`), closing one writes the cookie and localStorage. A failed
// read shows nothing: the bar must never complain. Never written to on the server (it is shared
// by every request there).
import { gql } from '$lib/api';
import type { AnnouncementFieldsFragment } from '$lib/gql/graphql';
import { ACTIVE_ANNOUNCEMENT } from '$lib/queries';
import { CLOSED_COOKIE, DISMISS_KEY, SEEN_KEY, closedCookie, mergeClosed, readClosed, readClosedCookie, withClosed } from './announcement';

export type Announcement = AnnouncementFieldsFragment;

/** A year, in seconds: how long the closed list is kept. */
const KEEP_S = 365 * 24 * 3600;

function writeCookie(list: readonly string[]) {
  try {
    document.cookie = `${CLOSED_COOKIE}=${closedCookie(list)}; Path=/; Max-Age=${KEEP_S}; SameSite=Lax${location.protocol === 'https:' ? '; Secure' : ''}`;
  } catch {
    /* the server shows it again next visit; the browser still hides it */
  }
}

class Banner {
  /** Undefined until the browser asked for itself (then the server's answer is old). */
  current = $state<Announcement | null | undefined>(undefined);
  /** Undefined until the browser read its own list. */
  closed = $state<string[] | undefined>(undefined);

  /**
   * Reads this device's closed list (localStorage and the cookie, the server's copy) and keeps the
   * cookie complete; with `force` (or on a prerendered page, which can't know), asks the API too.
   * Call from an effect (the browser only).
   */
  load(force = false) {
    let stored: string[] = [];
    try {
      stored = readClosed(localStorage.getItem(DISMISS_KEY));
    } catch {
      /* no storage: the cookie alone */
    }
    const fromCookie = readClosedCookie(document.cookie.match(new RegExp(`(?:^|;\\s*)${CLOSED_COOKIE}=([^;]*)`))?.[1]);
    const all = mergeClosed(fromCookie, stored);
    // Closed before the cookie existed: the server learns it from the next visit on.
    if (all.length !== fromCookie.length) writeCookie(all);
    this.closed = all;
    if (!force) return;
    gql(ACTIVE_ANNOUNCEMENT, {})
      .then((r) => (this.current = r.activeAnnouncement ?? null))
      .catch(() => {});
  }

  /** Remembers which one is live (or that none is), for app.html's hint on prerendered pages. */
  remember(a: Announcement | null) {
    try {
      if (a) localStorage.setItem(SEEN_KEY, a.id);
      else localStorage.removeItem(SEEN_KEY);
    } catch {
      /* no hint next time */
    }
  }

  /** Shown: there is one and this device hasn't closed it. `server` is the layout's answer. */
  shown(server: { announcement?: Announcement | null; announceClosed?: string[] }): Announcement | null {
    const a = this.current !== undefined ? this.current : (server.announcement ?? null);
    // The server's list (from the cookie) and this browser's (cookie and localStorage) together.
    const closed = [...(server.announceClosed ?? []), ...(this.closed ?? [])];
    return a && !closed.includes(a.id) ? a : null;
  }

  close(id: string) {
    const next = withClosed(this.closed ?? [], id);
    this.closed = next;
    writeCookie(next);
    try {
      localStorage.setItem(DISMISS_KEY, JSON.stringify(next));
    } catch {
      /* the cookie still remembers it */
    }
  }
}

export const banner = new Banner();
