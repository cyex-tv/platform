// GraphQL operations: emote pages. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const REPORT_EMOTE = graphql(`
  mutation ReportEmote($id: String!, $reason: String!) {
    reportEmote(id: $id, reason: $reason) {
      id
    }
  }
`);

export const MY_SET_NAMES = graphql(`
  query MySetNames {
    mySets {
      id
      name
      kind
      emotes {
        emote {
          id
        }
      }
    }
  }
`);

// A creator's public emotes (the creator page): the search, narrowed to one owner.
export const SEARCH_BY_OWNER = graphql(`
  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort, $after: String) {
    searchEmotes(query: "", owner: $owner, page: $page, perPage: $perPage, sort: $sort, after: $after) {
      total
      page
      perPage
      next
      items {
        ...EmoteFields
      }
    }
  }
`);

// The admin account page: the account as staff see it, its uploads, and its reports.
export const ADMIN_ACCOUNT = graphql(`
  query AdminAccount($accountId: String!) {
    adminAccount(accountId: $accountId) {
      id
      displayName
      avatar
      createdAt
      uploadsBlocked
      ticketsBlocked
      blocked
      uploadsBlockedUntil
      ticketsBlockedUntil
      blockedUntil
      identities {
        platform
        platformUserId
        login
        displayName
        avatar
      }
    }
  }
`);

export const TRANSFER_EMOTE = graphql(`
  mutation TransferEmote($id: String!, $toAccountId: String!) {
    transferEmote(id: $id, toAccountId: $toAccountId) {
      ...EmoteFields
    }
  }
`);

export const ACCOUNT_EMOTES = graphql(`
  query AccountEmotes($accountId: String!, $page: Int, $perPage: Int, $after: String) {
    accountEmotes(accountId: $accountId, page: $page, perPage: $perPage, after: $after) {
      total
      next
      items {
        ...EmoteFields
      }
    }
  }
`);

export const ACCOUNT_REPORTS = graphql(`
  query AccountReports($accountId: String!) {
    accountReports(accountId: $accountId) {
      about {
        ...ReportFields
      }
      by {
        ...ReportFields
      }
    }
  }
`);

// A public account by id (the creator page).
export const ACCOUNT = graphql(`
  query PublicAccount($id: String!) {
    account(id: $id) {
      id
      displayName
      avatar
    }
  }
`);

// Channels that have an emote in their active set (the emote page's "Used in").
export const EMOTE_CHANNELS = graphql(`
  query EmoteChannels($id: String!, $limit: Int, $offset: Int) {
    emoteChannels(id: $id, limit: $limit, offset: $offset) {
      platform
      login
      account {
        id
        displayName
        avatar
      }
    }
  }
`);

// Decide an emote's public listing and/or personal use (ADR 0038). The edit dialog changes a
// decision already made, so it sends onlyPending: false.
export const EMOTE_REVIEW_APPROVALS = graphql(`
  mutation EmoteReviewApprovals($ids: [String!]!, $input: ApprovalsInput!, $idempotencyKey: String) {
    reviewApprovals(ids: $ids, input: $input, idempotencyKey: $idempotencyKey) {
      done
      failed {
        id
        code
        message
      }
    }
  }
`);
