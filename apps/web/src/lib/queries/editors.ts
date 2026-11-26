// GraphQL operations: editors. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Account editors (ADR 0014): yours, and the accounts you edit.

export const ACCOUNT_EDITOR_FIELDS = graphql(`
  fragment AccountEditorFields on AccountEditor {
    account {
      id
      displayName
      avatar
    }
    permissions
    allSets
    setIds
    grantedBy {
      id
      displayName
    }
    grantedAt
  }
`);

export const EDITORS = graphql(`
  query Editors {
    accountEditors {
      ...AccountEditorFields
    }
    editorOf {
      account {
        id
        displayName
        avatar
      }
      permissions
      allSets
      setIds
      grantedAt
    }
    mySets {
      id
      name
      kind
      owner {
        id
      }
    }
  }
`);

export const ADD_ACCOUNT_EDITOR = graphql(`
  mutation AddAccountEditor($input: AddAccountEditorInput!) {
    addAccountEditor(input: $input) {
      ...AccountEditorFields
    }
  }
`);

export const UPDATE_ACCOUNT_EDITOR = graphql(`
  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) {
    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions, setIds: $setIds) {
      ...AccountEditorFields
    }
  }
`);

export const REMOVE_ACCOUNT_EDITOR = graphql(`
  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {
    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)
  }
`);
