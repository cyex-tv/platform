// GraphQL operations: my channels. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const MY_CHANNELS = graphql(`
  query MyChannels {
    myChannels {
      platform
      platformChannelId
      login
      activeSetId
      allowPersonalEmotes
      account {
        id
        displayName
      }
    }
    mySets {
      id
      name
      kind
      owner {
        id
      }
    }
    editorChannels {
      platform
      platformChannelId
      login
      activeSetId
      allowPersonalEmotes
      account {
        id
        displayName
      }
    }
    editorSets {
      id
      name
      kind
      owner {
        id
      }
    }
    editorOf {
      account {
        id
      }
      permissions
    }
  }
`);

export const UPDATE_CHANNEL = graphql(`
  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {
    updateChannelSettings(platform: $platform, id: $id, input: $input) {
      activeSetId
      allowPersonalEmotes
    }
  }
`);
