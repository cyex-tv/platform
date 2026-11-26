// GraphQL operations: staff. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The staff account dashboard and the entitlement graph (ADR 0010 amendment 2026/11/19).

export const STAFF_NOTES = graphql(`
  query StaffNotes($accountId: String!) {
    staffNotes(accountId: $accountId) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const ADD_STAFF_NOTE = graphql(`
  mutation AddStaffNote($accountId: String!, $body: String!) {
    addStaffNote(accountId: $accountId, body: $body) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const DELETE_STAFF_NOTE = graphql(`
  mutation DeleteStaffNote($id: String!) {
    deleteStaffNote(id: $id)
  }
`);

export const ACCOUNT_SESSIONS = graphql(`
  query AccountSessions($accountId: String!) {
    accountSessions(accountId: $accountId) {
      id
      kind
      device
      createdAt
      lastSeenAt
      expiresAt
    }
  }
`);

export const ACCOUNT_SUBSCRIPTIONS = graphql(`
  query AccountSubscriptions($accountId: String!) {
    accountSubscriptions(accountId: $accountId) {
      id
      plan
      state
      startsAt
      endsAt
      source
    }
  }
`);

export const GRANT_FIELDS = graphql(`
  fragment GrantFields on EntitlementGrant {
    kind
    itemId
    itemName
    accountId
    source
    reason
    grantedBy
    grantedAt
    orderId
    orderName
    orderStatus
    codeBatchId
    codeBatchName
    revokedAt
    revokedBy
    revokeReason
    active
  }
`);

export const ENTITLEMENT_GRAPH = graphql(`
  query EntitlementGraph($accountId: String!) {
    entitlementGraph(accountId: $accountId) {
      ...GrantFields
    }
  }
`);

export const ENTITLEMENT_HOLDERS = graphql(`
  query EntitlementHolders($kind: GrantKind!, $id: String!) {
    entitlementHolders(kind: $kind, id: $id) {
      ...GrantFields
    }
  }
`);
