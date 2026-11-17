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
  query Search($query: String!, $animated: Boolean, $nsfw: Boolean, $page: Int, $perPage: Int) {
    searchEmotes(query: $query, animated: $animated, nsfw: $nsfw, page: $page, perPage: $perPage) {
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

// Admin: review, reports, users, roles, audit.

export const ADMIN_COUNTS = graphql(`
  query AdminCounts($review: Boolean!, $reports: Boolean!) {
    reviewQueue(page: 1, perPage: 1) @include(if: $review) {
      total
    }
    reportQueue(page: 1, perPage: 1) @include(if: $reports) {
      total
    }
    globalSet {
      id
    }
  }
`);

export const REVIEW_QUEUE = graphql(`
  query ReviewQueue($page: Int, $perPage: Int) {
    reviewQueue(page: $page, perPage: $perPage) {
      total
      items {
        ...EmoteFields
      }
    }
  }
`);

export const APPROVE = graphql(`
  mutation Approve($id: String!) {
    approveEmote(id: $id) {
      id
      state
    }
  }
`);

export const REJECT = graphql(`
  mutation Reject($id: String!, $reason: String!) {
    rejectEmote(id: $id, reason: $reason) {
      id
      state
    }
  }
`);

export const REPORT_QUEUE = graphql(`
  query ReportQueue($page: Int, $perPage: Int) {
    reportQueue(page: $page, perPage: $perPage) {
      total
      items {
        id
        reason
        reporterId
        state
        createdAt
        emote {
          ...EmoteFields
        }
      }
    }
  }
`);

export const RESOLVE_REPORT = graphql(`
  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {
    resolveReport(id: $id, action: $action, note: $note) {
      id
      state
    }
  }
`);

export const TAKEDOWN = graphql(`
  mutation Takedown($id: String!, $reason: String!) {
    takedownEmote(id: $id, reason: $reason) {
      id
      state
    }
  }
`);

export const ROLE_FIELDS = graphql(`
  fragment RoleFields on Role {
    id
    name
    description
    position
    permissions
  }
`);

export const ROLES = graphql(`
  query Roles {
    roles {
      ...RoleFields
    }
    permissions
  }
`);

export const CREATE_ROLE = graphql(`
  mutation CreateRole($input: CreateRoleInput!) {
    createRole(input: $input) {
      ...RoleFields
    }
  }
`);

export const UPDATE_ROLE = graphql(`
  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {
    updateRole(id: $id, input: $input) {
      ...RoleFields
    }
  }
`);

export const DELETE_ROLE = graphql(`
  mutation DeleteRole($id: String!) {
    deleteRole(id: $id)
  }
`);

export const USER = graphql(`
  query User($platform: Platform!, $login: String!) {
    channelByLogin(platform: $platform, login: $login) {
      platform
      platformChannelId
      login
      account {
        id
        displayName
        avatar
      }
    }
  }
`);

export const USER_DETAILS = graphql(`
  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {
    accountStatus(accountId: $accountId) @include(if: $status) {
      uploadsBlocked
      blocked
    }
    accountRoles(accountId: $accountId) @include(if: $roles) {
      ...RoleFields
    }
  }
`);

export const ALL_ROLES = graphql(`
  query AllRoles {
    roles {
      ...RoleFields
    }
  }
`);

export const RESTRICT = graphql(`
  mutation Restrict($accountId: String!, $uploadsBlocked: Boolean, $blocked: Boolean) {
    restrictAccount(accountId: $accountId, uploadsBlocked: $uploadsBlocked, blocked: $blocked) {
      uploadsBlocked
      blocked
    }
  }
`);

export const ASSIGN_ROLE = graphql(`
  mutation AssignRole($accountId: String!, $roleId: String!) {
    assignRole(accountId: $accountId, roleId: $roleId)
  }
`);

export const UNASSIGN_ROLE = graphql(`
  mutation UnassignRole($accountId: String!, $roleId: String!) {
    unassignRole(accountId: $accountId, roleId: $roleId)
  }
`);

export const AUDIT = graphql(`
  query Audit($targetKind: String, $targetId: String, $before: String, $limit: Int) {
    auditLog(targetKind: $targetKind, targetId: $targetId, before: $before, limit: $limit) {
      id
      actorId
      action
      targetKind
      targetId
      before
      after
      createdAt
    }
  }
`);

export const REPORT_EMOTE = graphql(`
  mutation ReportEmote($id: String!, $reason: String!) {
    reportEmote(id: $id, reason: $reason) {
      id
    }
  }
`);

export const MY_SET_NAMES = graphql(`
  query MySetNames {
    mySets {
      id
      name
      kind
      emotes {
        emote {
          id
        }
      }
    }
  }
`);
