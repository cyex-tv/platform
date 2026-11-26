// GraphQL operations: search. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The header search: the top emotes by the name matcher and the top people, in one request.
export const QUICK_SEARCH = graphql(`
  query QuickSearch($query: String!) {
    quickSearch(query: $query) {
      emotes {
        ...EmoteFields
      }
      users {
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
      }
    }
  }
`);

// What the empty header search lists: the channels you can manage and your role on each.
export const EDITABLE_CHANNELS = graphql(`
  query EditableChannels {
    myEditableChannels {
      role
      channel {
        platform
        login
        account {
          id
          displayName
          avatar
        }
      }
    }
  }
`);
