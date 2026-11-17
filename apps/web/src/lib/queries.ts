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
      createdAt
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

export const SET_FIELDS = graphql(`
  fragment SetFields on EmoteSet {
    id
    name
    kind
    capacity
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
`);

export const MY_SETS = graphql(`
  query MySets {
    mySets {
      ...SetFields
    }
  }
`);

export const EDIT_SET = graphql(`
  query EditSet($id: String!) {
    set(id: $id) {
      ...SetFields
    }
    setEditors(setId: $id) {
      id
      displayName
      avatar
    }
  }
`);

export const CREATE_SET = graphql(`
  mutation CreateSet($input: CreateSetInput!) {
    createSet(input: $input) {
      id
    }
  }
`);

export const UPDATE_SET = graphql(`
  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {
    updateSet(id: $id, input: $input) {
      id
      name
    }
  }
`);

export const DELETE_SET = graphql(`
  mutation DeleteSet($id: String!) {
    deleteSet(id: $id)
  }
`);

export const ADD_SET_EMOTE = graphql(`
  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {
    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {
      name
    }
  }
`);

export const UPDATE_SET_EMOTE = graphql(`
  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {
    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {
      name
      zeroWidth
    }
  }
`);

export const REMOVE_SET_EMOTE = graphql(`
  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {
    removeSetEmote(setId: $setId, emoteId: $emoteId)
  }
`);

export const ADD_SET_EDITOR = graphql(`
  mutation AddSetEditor($setId: String!, $accountId: String!) {
    addSetEditor(setId: $setId, accountId: $accountId)
  }
`);

export const REMOVE_SET_EDITOR = graphql(`
  mutation RemoveSetEditor($setId: String!, $accountId: String!) {
    removeSetEditor(setId: $setId, accountId: $accountId)
  }
`);

export const FIND_ACCOUNT = graphql(`
  query FindAccount($platform: Platform!, $login: String!) {
    channelByLogin(platform: $platform, login: $login) {
      login
      account {
        id
        displayName
        avatar
      }
    }
  }
`);

export const MY_CHANNELS = graphql(`
  query MyChannels {
    myChannels {
      platform
      platformChannelId
      login
      activeSetId
      allowPersonalEmotes
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

export const UPDATE_CHANNEL = graphql(`
  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {
    updateChannelSettings(platform: $platform, id: $id, input: $input) {
      activeSetId
      allowPersonalEmotes
    }
  }
`);

export const UNLINK = graphql(`
  mutation Unlink($platform: Platform!) {
    unlinkIdentity(platform: $platform) {
      id
    }
  }
`);

export const MY_NOTIFICATIONS = graphql(`
  query MyNotifications {
    myNotifications {
      id
      kind
      payload
      createdAt
      read
    }
  }
`);

export const MARK_READ = graphql(`
  mutation MarkRead {
    markNotificationsRead
  }
`);
