import type { EmoteState } from '$lib/gql/graphql';

/** How an emote's state reads in the signed-in pages. */
export const STATE_LABEL: Record<EmoteState, string> = {
  PENDING: 'Pending',
  APPROVED: 'Approved',
  REJECTED: 'Rejected',
  TAKEN_DOWN: 'Taken down',
};

export const STATE_HINT: Record<EmoteState, string> = {
  PENDING: 'Waiting for a moderator. You can already use it in your own sets.',
  APPROVED: 'Approved: it works wherever it’s in a set. Its visibility says who can find it.',
  REJECTED: 'A moderator turned it down. Only you can see it.',
  TAKEN_DOWN: 'Removed everywhere after a report.',
};
