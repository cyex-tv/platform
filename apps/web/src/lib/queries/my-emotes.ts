// GraphQL operations: my emotes. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Signed in: your emotes, sets, channels and account.

export const MY_EMOTES = graphql(`
  query MyEmotes($page: Int, $perPage: Int) {
    myEmotes(page: $page, perPage: $perPage) {
      total
      items {
        ...EmoteFields
      }
    }
  }
`);

export const UPDATE_EMOTE = graphql(`
  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {
    updateEmote(id: $id, input: $input) {
      ...EmoteFields
    }
  }
`);

export const DELETE_EMOTE = graphql(`
  mutation DeleteEmote($id: String!) {
    deleteEmote(id: $id)
  }
`);
