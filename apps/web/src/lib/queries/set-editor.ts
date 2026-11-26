// GraphQL operations: set editor. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const SET_FIELDS = graphql(`
  fragment SetFields on EmoteSet {
    id
    name
    kind
    visibility
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
    editorSets {
      ...SetFields
    }
    editorOf {
      account {
        id
        displayName
      }
      permissions
    }
  }
`);

// A set without its emotes (a set can hold a thousand): the editor reads those a page at a time.
export const EDIT_SET = graphql(`
  query EditSet($id: String!) {
    set(id: $id) {
      id
      name
      kind
      visibility
      capacity
      slotsUsed
      owner {
        id
        displayName
        avatar
      }
    }
    setEditors(setId: $id) {
      id
      displayName
      avatar
    }
    editorOf {
      account {
        id
      }
      permissions
      allSets
      setIds
    }
    myChannels {
      platform
      platformChannelId
      login
      activeSetId
      account {
        id
      }
    }
    editorChannels {
      platform
      platformChannelId
      login
      activeSetId
      account {
        id
      }
    }
  }
`);

// One page of a set's emotes, oldest first, narrowed by the filter on the server.
export const SET_EMOTES = graphql(`
  query SetEmotes($id: String!, $query: String, $page: Int, $perPage: Int) {
    setEmotes(id: $id, query: $query, page: $page, perPage: $perPage) {
      total
      items {
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

// Who is in the set, no pictures: ids and names, for "Added" and name clashes in Add emotes.
export const SET_MEMBERS = graphql(`
  query SetMembers($id: String!, $page: Int, $perPage: Int) {
    setEmotes(id: $id, page: $page, perPage: $perPage) {
      total
      items {
        name
        emote {
          id
        }
      }
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
      visibility
      capacity
    }
  }
`);

export const DELETE_SET = graphql(`
  mutation DeleteSet($id: String!) {
    deleteSet(id: $id)
  }
`);

/** What you may set a set's capacity to; null when you can't change it. */
export const SET_CAPACITY_RANGE = graphql(`
  query SetCapacityRange($id: String!) {
    setCapacityRange(id: $id) {
      min
      max
      ownerMax
      used
      staff
    }
  }
`);

/** What a set's right-click menu offers you: its capacity range (null: not yours to change), your grants and the channels you can point at it. */
export const SET_MENU = graphql(`
  query SetMenu($id: String!) {
    setCapacityRange(id: $id) {
      min
      max
      ownerMax
      used
      staff
    }
    editorOf {
      account {
        id
      }
      permissions
      allSets
      setIds
    }
    myChannels {
      platform
      platformChannelId
      login
      activeSetId
      account {
        id
      }
    }
    editorChannels {
      platform
      platformChannelId
      login
      activeSetId
      account {
        id
      }
    }
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

export const REMOVE_SET_EDITOR = graphql(`
  mutation RemoveSetEditor($setId: String!, $accountId: String!) {
    removeSetEditor(setId: $setId, accountId: $accountId)
  }
`);
