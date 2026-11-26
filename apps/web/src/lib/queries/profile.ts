// GraphQL operations: profile. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The profile (your profile is your dashboard): roles, editors, the accounts you edit.

export const ACCOUNT_ROLES = graphql(`
  query AccountRoles($accountId: String!) {
    accountRoles(accountId: $accountId) {
      id
      name
      position
    }
  }
`);

export const ACCOUNT_EDITORS = graphql(`
  query AccountEditors($accountId: String) {
    accountEditors(accountId: $accountId) {
      account {
        id
        displayName
        avatar
      }
      permissions
    }
  }
`);

export const EDITOR_OF = graphql(`
  query EditorOf {
    editorOf {
      account {
        id
        displayName
        avatar
      }
      permissions
      allSets
    }
  }
`);

// A profile's listed sets as cards, for anyone (the Emote sets section of someone else's profile).
export const ACCOUNT_SETS = graphql(`
  query AccountSets($accountId: String!) {
    accountSets(accountId: $accountId, preview: 12) {
      id
      name
      kind
      capacity
      slotsUsed
      active
      preview {
        emote {
          ...EmoteFields
        }
      }
    }
  }
`);

export const PROFILE_MINE = graphql(`
  query ProfileMine {
    myChannels {
      platform
      platformChannelId
      login
      activeSetId
    }
    mySets {
      ...SetFields
    }
  }
`);

export const PROFILE_EDITED = graphql(`
  query ProfileEdited {
    editorChannels {
      platform
      platformChannelId
      login
      activeSetId
      account {
        id
      }
    }
    editorSets {
      ...SetFields
    }
  }
`);
