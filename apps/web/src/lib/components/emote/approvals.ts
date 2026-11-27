// Pure helpers for the two approvals of an emote (ADR 0038): how they read, who may add what to a
// personal set, and the account a pasted profile link names.
import type { ApprovalState } from '$lib/gql/graphql';

export type Approvals = { publicListing: ApprovalState; personalUse: ApprovalState };

/** How an approval reads on the page. */
export function approvalWord(s: ApprovalState): string {
  return s === 'APPROVED' ? 'Approved' : s === 'DENIED' ? 'Denied' : 'Pending';
}

/** The tone class of an approval: approved is ok, the rest are muted. */
export function approvalTone(s: ApprovalState): 'ok' | 'wait' | 'off' {
  return s === 'APPROVED' ? 'ok' : s === 'DENIED' ? 'off' : 'wait';
}

/** Why an emote can't go in a personal set (null when it can). Without approvals nothing is blocked (the API still checks). */
export function personalBlock(approvals: Pick<Approvals, 'personalUse'> | null | undefined): string | null {
  if (!approvals || approvals.personalUse === 'APPROVED') return null;
  return approvals.personalUse === 'DENIED' ? 'Not approved for personal use' : 'Not approved for personal use yet';
}

/** The message for an add that failed with `personal_use_not_approved`. */
export const PERSONAL_NOT_APPROVED = 'This emote isn’t approved for personal use yet.';

/** The account id in a pasted profile link (https://cyex.tv/users/<id>, with or without the origin), else null. */
export function accountIdFromLink(text: string): string | null {
  const m = /(?:^|\/\/[^/\s]+|^\/)\/?users\/([^/?#\s]+)/i.exec(text.trim());
  if (!m) return null;
  try {
    return decodeURIComponent(m[1]);
  } catch {
    return null;
  }
}
