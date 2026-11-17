// Every GraphQL operation the site sends. Their types come from `npm run codegen` (src/lib/gql).
import { graphql } from './gql';

export const EMOTE_FIELDS = graphql(`
  fragment EmoteFields on Emote {
    id
    name
    state
    animated
    tags
    host
    createdAt
    flags {
      zeroWidth
      nsfw
      private
    }
    owner {
      id
      displayName
      avatar
    }
    files {
      name
      format
      scale
      width
      height
      size
      isStatic
    }
  }
`);

export const SEARCH = graphql(`
  query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int) {
    searchEmotes(query: $query, animated: $animated, page: $page, perPage: $perPage) {
      total
      page
      perPage
      items {
        ...EmoteFields
      }
    }
  }
`);

export const EMOTE = graphql(`
  query Emote($id: String!) {
    emote(id: $id) {
      ...EmoteFields
    }
  }
`);

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

export const SET = graphql(`
  query Set($id: String!) {
    set(id: $id) {
      id
      name
      kind
      capacity
      owner {
        id
        displayName
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

export const GLOBAL_SET = graphql(`
  query GlobalSet {
    globalSet {
      id
      name
      emotes {
        name
        zeroWidth
        emote {
          ...EmoteFields
        }
      }
    }
  }
`);

export const ME = graphql(`
  query Me {
    me {
      id
      permissions
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
