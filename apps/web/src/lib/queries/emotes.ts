// GraphQL operations: emotes. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const EMOTE_FIELDS = graphql(`
  fragment EmoteFields on Emote {
    id
    name
    state
    processing
    animated
    tags
    host
    createdAt
    channelCount
    visibility
    flags {
      zeroWidth
      private
      wide
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
  query Search(
    $query: String!
    $animated: Boolean
    $page: Int
    $perPage: Int
    $sort: EmoteSort
    $flags: [EmoteFlag!]
    $tags: [String!]
    $exact: Boolean
    $window: TrendingWindow
  ) {
    searchEmotes(
      query: $query
      animated: $animated
      page: $page
      perPage: $perPage
      sort: $sort
      flags: $flags
      tags: $tags
      exact: $exact
      window: $window
    ) {
      total
      page
      perPage
      items {
        ...EmoteFields
      }
    }
  }
`);

// People by login or display name (the Users tab, the palette, admin). No emote count: the API gives none.
export const SEARCH_ACCOUNTS = graphql(`
  query FindPeople($query: String!, $page: Int, $perPage: Int) {
    searchAccounts(query: $query, page: $page, perPage: $perPage) {
      total
      page
      perPage
      items {
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

export const EMOTE = graphql(`
  query Emote($id: String!) {
    emote(id: $id) {
      ...EmoteFields
    }
  }
`);

// Where an emote stands among the trending ones (the emote page's Trending label): null outside the top 100.
export const EMOTE_TRENDING = graphql(`
  query EmoteTrending($id: String!, $window: TrendingWindow) {
    emoteTrending(id: $id, window: $window) {
      rank
      channels
      window
    }
  }
`);
