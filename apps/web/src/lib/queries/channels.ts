// GraphQL operations: channels. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const CHANNEL = graphql(`
  query Channel($platform: Platform!, $login: String!) {
    channelByLogin(platform: $platform, login: $login) {
      platform
      platformChannelId
      login
      activeSetId
      allowPersonalEmotes
      account {
        id
        displayName
        avatar
      }
    }
  }
`);
