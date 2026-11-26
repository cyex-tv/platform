// The site announcement for the bar under the header, read on the server so the page arrives with
// the bar in place (or without it) and nothing moves after the first paint. The closed list comes
// from this device's cookie. Quick or nothing: a slow or failed answer renders no bar. Pages
// prerendered at build time (help, guidelines, changelog, download, developers) can't know it:
// `undefined` tells the bar to ask from the browser, where app.html's hint holds its place.
import { building } from '$app/environment';
import { fetchWith, gqlPublic } from '$lib/api';
import { ACTIVE_ANNOUNCEMENT } from '$lib/queries';
import { CLOSED_COOKIE, readClosedCookie } from '$lib/shell/announcement';
import type { Announcement } from '$lib/shell/announcement.svelte';
import type { LayoutServerLoad } from './$types';

/** How long the page waits for the announcement before it renders without one. */
const ANNOUNCE_WAIT_MS = 1500;

export const load: LayoutServerLoad = async ({ fetch, cookies }) => {
  if (building) return { announcement: undefined, announceClosed: [] as string[] };
  const announceClosed = readClosedCookie(cookies.get(CLOSED_COOKIE));
  let announcement: Announcement | null | undefined = null;
  try {
    announcement = (await gqlPublic(ACTIVE_ANNOUNCEMENT, {}, fetchWith(AbortSignal.timeout(ANNOUNCE_WAIT_MS), fetch))).activeAnnouncement ?? null;
  } catch {
    /* no bar this time */
  }
  return { announcement, announceClosed };
};
