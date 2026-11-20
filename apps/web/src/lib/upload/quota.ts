// How many of the account's emotes wait for review, and how many may (uploadQuota): the line the
// upload form shows, and whether it is at the cap, where the API refuses uploads
// (too_many_pending) until some are reviewed.
import type { UploadQuotaQuery } from '$lib/gql/graphql';
import { count } from '$lib/format';

export type Quota = UploadQuotaQuery['uploadQuota'];

export type QuotaView = {
  /** "3 of 20 waiting for review." */
  line: string;
  /** At the cap: nothing more can be uploaded until some are reviewed. */
  full: boolean;
  /** What to say at the cap, as the API says it. */
  refusal: string | null;
  /** Files that still fit, at most. */
  room: number;
};

const emotes = (n: number) => (n === 1 ? '1 emote' : `${count(n)} emotes`);

export function quotaView(q: Quota | null | undefined, forOther = false): QuotaView | null {
  if (!q) return null;
  // No cap (the owners): only the count, never full.
  if (q.pendingMax == null) return { line: `${count(q.pending)} waiting for review.`, full: false, refusal: null, room: Infinity };
  const room = Math.max(0, q.pendingMax - q.pending);
  const full = room === 0;
  const who = forOther ? 'That account has' : 'You have';
  return {
    line: `${count(q.pending)} of ${count(q.pendingMax)} waiting for review.`,
    full,
    refusal: full ? `${who} ${emotes(q.pendingMax)} waiting for review. Upload more once some are reviewed.` : null,
    room,
  };
}
