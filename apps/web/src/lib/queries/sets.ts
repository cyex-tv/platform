// GraphQL operations: sets. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The public set page (/sets/<id>): listed and unlisted sets open for anyone; a private one only
// for its owner and editors, and for everyone else it answers as if it did not exist.
export const PUBLIC_SET = graphql(`
  query PublicSet($id: String!) {
    set(id: $id) {
      id
      name
      kind
      visibility
      capacity
      slotsUsed
      createdAt
      owner {
        id
        displayName
        avatar
      }
      emotes {
        name
        zeroWidth
        addedAt
        emote {
          ...EmoteFields
        }
      }
    }
  }
`);

// A set's editors: answered for its owner, its editors and staff (`sets.edit_any`); others get an error.
export const SET_EDITORS = graphql(`
  query SetEditors($id: String!) {
    setEditors(setId: $id) {
      id
      displayName
      avatar
    }
  }
`);

export const CLONE_SET = graphql(`
  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {
    cloneSet(id: $id, name: $name, visibility: $visibility) {
      copied
      skipped
      set {
        id
        name
      }
    }
  }
`);
