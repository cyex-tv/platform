// GraphQL operations: activity. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// An account's public activity feed (ADR 0016), newest first; page back with `before` set to the
// answer's `next` (a page can be short and still have one).
export const ACCOUNT_ACTIVITY = graphql(`
  query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {
    accountActivityPage(accountId: $accountId, before: $before, limit: $limit, filter: $filter) {
      entries {
        id
        kind
        actor {
          id
          displayName
          avatar
        }
        editor {
          id
          displayName
          avatar
        }
        set {
          id
          name
        }
        emote {
          id
          name
          host
          files {
            name
            width
            height
            isStatic
          }
        }
        fromName
        toName
        badge {
          fromId
          fromName
          toId
          toName
        }
        paint {
          fromId
          fromName
          toId
          toName
        }
        createdAt
      }
      next
    }
  }
`);

export const EMOTE_ACTIVITY = graphql(`
  query EmoteActivity($emoteId: String!, $before: String, $limit: Int, $actorId: String) {
    emoteActivityPage(emoteId: $emoteId, before: $before, limit: $limit, actorId: $actorId) {
      entries {
        id
        kind
        actor {
          id
          displayName
          avatar
        }
        set {
          id
          name
        }
        emote {
          id
          name
          host
          files {
            name
            width
            height
            isStatic
          }
        }
        fromName
        toName
        badge {
          fromId
          fromName
          toId
          toName
        }
        paint {
          fromId
          fromName
          toId
          toName
        }
        createdAt
      }
      next
    }
  }
`);
