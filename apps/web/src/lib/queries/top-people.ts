// GraphQL operations: top people. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The person filters' "most important people" (UserFilter): who has the most entries in a list,
// offered when the filter is focused and empty. Each is visible as its list is.
export const PERSON_COUNT = graphql(`
  fragment PersonCountFields on PersonCount {
    account {
      id
      displayName
      avatar
    }
    identities {
      platform
      login
      displayName
    }
    count
  }
`);

export const ACTIVITY_TOP_ACTORS = graphql(`
  query ActivityTopActors($accountId: String, $emoteId: String, $filter: ActivityFilter, $limit: Int) {
    activityTopActors(accountId: $accountId, emoteId: $emoteId, filter: $filter, limit: $limit) {
      ...PersonCountFields
    }
  }
`);

export const AUDIT_TOP_PEOPLE = graphql(`
  query AuditTopPeople(
    $by: AuditPersonRole
    $targetKind: String
    $targetId: String
    $actorId: String
    $action: String
    $since: String
    $until: String
    $limit: Int
  ) {
    auditTopPeople(
      by: $by
      targetKind: $targetKind
      targetId: $targetId
      actorId: $actorId
      action: $action
      since: $since
      until: $until
      limit: $limit
    ) {
      ...PersonCountFields
    }
  }
`);

export const ORDER_TOP_BUYERS = graphql(`
  query OrderTopBuyers($status: OrderStatus, $itemId: String, $limit: Int) {
    orderTopBuyers(status: $status, itemId: $itemId, limit: $limit) {
      ...PersonCountFields
    }
  }
`);

export const REVIEW_TOP_UPLOADERS = graphql(`
  query ReviewTopUploaders($filter: ReviewQueueFilter, $limit: Int) {
    reviewTopUploaders(filter: $filter, limit: $limit) {
      ...PersonCountFields
    }
  }
`);

export const ENTITLEMENT_TOP_HOLDERS = graphql(`
  query EntitlementTopHolders($limit: Int) {
    entitlementTopHolders(limit: $limit) {
      ...PersonCountFields
    }
  }
`);
