// GraphQL operations: audit. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const AUDIT = graphql(`
  query Audit(
    $targetKind: String
    $targetId: String
    $actorId: String
    $action: String
    $since: String
    $until: String
    $before: String
    $limit: Int
  ) {
    auditLog(
      targetKind: $targetKind
      targetId: $targetId
      actorId: $actorId
      action: $action
      since: $since
      until: $until
      before: $before
      limit: $limit
    ) {
      id
      actorId
      action
      targetKind
      targetId
      before
      after
      createdAt
    }
  }
`);
