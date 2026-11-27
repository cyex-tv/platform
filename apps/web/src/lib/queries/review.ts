// GraphQL operations: review. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Admin: review, reports, users, roles, audit.

export const ADMIN_COUNTS = graphql(`
  query AdminCounts($review: Boolean!, $reports: Boolean!) {
    reviewQueue(page: 1, perPage: 1) @include(if: $review) {
      total
    }
    reportQueue(page: 1, perPage: 1) @include(if: $reports) {
      total
    }
    globalSet {
      id
    }
  }
`);

export const REVIEW_QUEUE = graphql(`
  query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {
    reviewQueue(page: $page, perPage: $perPage, filter: $filter) {
      total
      counts {
        public
        limited
        personal
      }
      items {
        ...EmoteFields
      }
    }
  }
`);

export const APPROVE = graphql(`
  mutation Approve($id: String!, $visibility: EmoteVisibility) {
    approveEmote(id: $id, visibility: $visibility) {
      id
      state
      visibility
    }
  }
`);

export const EMOTE_HISTORY = graphql(`
  query EmoteHistory($emoteId: String!) {
    emoteHistory(emoteId: $emoteId) {
      id
      actorId
      action
      before
      after
      createdAt
    }
  }
`);

export const EMOTE_NOTES = graphql(`
  query EmoteNotes($emoteId: String!) {
    emoteNotes(emoteId: $emoteId) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const ADD_EMOTE_NOTE = graphql(`
  mutation AddEmoteNote($emoteId: String!, $body: String!) {
    addEmoteNote(emoteId: $emoteId, body: $body) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const DELETE_EMOTE_NOTE = graphql(`
  mutation DeleteEmoteNote($id: String!) {
    deleteEmoteNote(id: $id)
  }
`);

export const REJECT = graphql(`
  mutation Reject($id: String!, $reason: String!) {
    rejectEmote(id: $id, reason: $reason) {
      id
      state
    }
  }
`);
