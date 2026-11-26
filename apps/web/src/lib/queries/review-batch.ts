// GraphQL operations: review batch. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Review queue: batch verdicts, possible duplicates, ready reasons; bulk report dismissals;
// pasted logins for grants to many; the uploader's same-file check.

export const REVIEW_EMOTES = graphql(`
  mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {
    reviewEmotes(ids: $ids, verdict: $verdict, visibility: $visibility, reason: $reason) {
      done
      failed {
        id
        code
        message
      }
    }
  }
`);

export const REQUEUE_EMOTES = graphql(`
  mutation RequeueEmotes($ids: [String!]!) {
    requeueEmotes(ids: $ids) {
      done
      failed {
        id
        code
        message
      }
    }
  }
`);

export const SET_EMOTE_FLAGS = graphql(`
  mutation SetEmoteFlags($id: String!, $input: EmoteFlagsInput!) {
    setEmoteFlags(id: $id, input: $input) {
      ...EmoteFields
    }
  }
`);

export const DISMISS_REPORTS = graphql(`
  mutation DismissReports($ids: [String!]!, $note: String) {
    dismissReports(ids: $ids, note: $note) {
      done
      failed {
        id
        code
        message
      }
    }
  }
`);

export const EMOTE_DUPLICATES = graphql(`
  query EmoteDuplicates($emoteId: String!) {
    emoteDuplicates(emoteId: $emoteId) {
      sameFile
      similarity
      emote {
        ...EmoteFields
      }
    }
  }
`);

export const REVIEW_REASONS = graphql(`
  query ReviewReasons {
    reviewReasons
  }
`);

export const SET_REVIEW_REASONS = graphql(`
  mutation SetReviewReasons($reasons: [String!]!) {
    setReviewReasons(reasons: $reasons)
  }
`);

export const ACCOUNTS_BY_LOGIN = graphql(`
  query AccountsByLogin($logins: [String!]!) {
    accountsByLogin(logins: $logins) {
      login
      problem
      account {
        id
        displayName
        avatar
      }
    }
  }
`);

export const EMOTES_WITH_SAME_FILE = graphql(`
  query EmotesWithSameFile($sha256: String!) {
    emotesWithSameFile(sha256: $sha256) {
      id
      name
      owner {
        id
        displayName
      }
    }
  }
`);
