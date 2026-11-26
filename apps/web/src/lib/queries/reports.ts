// GraphQL operations: reports. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const REPORT_FIELDS = graphql(`
  fragment ReportFields on Report {
    id
    reason
    reporterId
    state
    resolution
    createdAt
    resolvedAt
    emote {
      ...EmoteFields
    }
  }
`);

export const REPORT_QUEUE = graphql(`
  query ReportQueue($page: Int, $perPage: Int) {
    reportQueue(page: $page, perPage: $perPage) {
      total
      items {
        id
        reason
        reporterId
        state
        createdAt
        emote {
          ...EmoteFields
        }
      }
    }
  }
`);

export const RESOLVE_REPORT = graphql(`
  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {
    resolveReport(id: $id, action: $action, note: $note) {
      id
      state
    }
  }
`);
