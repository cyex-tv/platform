// The number waiting in the Review row of the admin rail and overview. Each part of the queue is
// asked for only by someone who may read it (the API checks the same permissions): the main queue
// needs `emotes.review`, the personal-use queue `emotes.review_personal` (ADR 0039).
import type { AdminCountsQuery } from '$lib/gql/graphql';

type Can = (permission: string) => boolean;

/** The variables of `AdminCounts` for what `can` allows. */
export function adminCountVars(can: Can) {
  const review = can('emotes.review');
  const reports = review || can('emotes.takedown') || can('reports.manage');
  return { review, reports, personal: !review && can('emotes.review_personal') };
}

/** Whether `AdminCounts` has anything to ask for. */
export const asksForCounts = (v: ReturnType<typeof adminCountVars>) => v.review || v.reports || v.personal;

/**
 * Waiting for a verdict: reviewers see the pending uploads, plus the personal-use queue if they
 * may review that too; someone with only `emotes.review_personal` sees just the personal queue.
 */
export function reviewWaiting(counts: Pick<AdminCountsQuery, 'reviewQueue' | 'personalQueue'> | null | undefined, can: Can): number | undefined {
  if (can('emotes.review')) {
    const main = counts?.reviewQueue;
    if (!main) return undefined;
    return main.total + (can('emotes.review_personal') ? main.counts.personal : 0);
  }
  return can('emotes.review_personal') ? counts?.personalQueue?.total : undefined;
}
