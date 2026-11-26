// GraphQL operations: account. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const ME = graphql(`
  query Me {
    me {
      id
      createdAt
      permissions
      displayPlatform
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
