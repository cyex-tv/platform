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
    channelCount
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
  query Search($query: String!, $animated: Boolean, $nsfw: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort) {
    searchEmotes(query: $query, animated: $animated, nsfw: $nsfw, page: $page, perPage: $perPage, sort: $sort) {
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
    editorOf {
      account {
        id
      }
      permissions
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

// Account editors (ADR 0014): yours, and the accounts you edit.

export const ACCOUNT_EDITOR_FIELDS = graphql(`
  fragment AccountEditorFields on AccountEditor {
    account {
      id
      displayName
      avatar
    }
    permissions
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
      grantedAt
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
  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!) {
    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions) {
      ...AccountEditorFields
    }
  }
`);

export const REMOVE_ACCOUNT_EDITOR = graphql(`
  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {
    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)
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
  mutation MarkRead($ids: [String!]) {
    markNotificationsRead(ids: $ids)
  }
`);

export const UNREAD_COUNT = graphql(`
  query UnreadCount {
    unreadNotificationCount
  }
`);

export const MY_SESSIONS = graphql(`
  query MySessions {
    mySessions {
      id
      kind
      device
      createdAt
      lastSeenAt
      expiresAt
      current
    }
  }
`);

export const REVOKE_SESSION = graphql(`
  mutation RevokeSession($id: String!) {
    revokeSession(id: $id)
  }
`);

export const REVOKE_OTHER_SESSIONS = graphql(`
  mutation RevokeOtherSessions {
    revokeOtherSessions
  }
`);

export const EXPORT_MY_DATA = graphql(`
  mutation ExportMyData {
    exportMyData
  }
`);

export const DELETE_ACCOUNT = graphql(`
  mutation DeleteAccount($confirm: String!) {
    deleteAccount(confirm: $confirm)
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
    public
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

// A creator's public emotes (the creator page): the search, narrowed to one owner.
export const SEARCH_BY_OWNER = graphql(`
  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort) {
    searchEmotes(query: "", owner: $owner, page: $page, perPage: $perPage, sort: $sort) {
      total
      page
      perPage
      items {
        ...EmoteFields
      }
    }
  }
`);

// A public account by id (the creator page).
export const ACCOUNT = graphql(`
  query PublicAccount($id: String!) {
    account(id: $id) {
      id
      displayName
      avatar
    }
  }
`);

// Channels that have an emote in their active set (the emote page's "Used in").
export const EMOTE_CHANNELS = graphql(`
  query EmoteChannels($id: String!, $limit: Int) {
    emoteChannels(id: $id, limit: $limit) {
      platform
      login
      account {
        displayName
        avatar
      }
    }
  }
`);

// Chat cosmetics (ADR 0015): badges and name paints. The dev mock answers only the cosmetics part
// of an operation, so these stay in operations of their own.

export const COSMETIC_IMAGE_FIELDS = graphql(`
  fragment CosmeticImageFields on CosmeticImage {
    id
    kind
    host
    animated
    ready
    width
    height
    files {
      name
      width
      height
      isStatic
    }
  }
`);

export const BADGE_FIELDS = graphql(`
  fragment BadgeFields on Badge {
    id
    name
    tooltip
    image {
      ...CosmeticImageFields
    }
  }
`);

export const PAINT_FIELDS = graphql(`
  fragment PaintFields on Paint {
    id
    name
    kind
    angle
    repeat
    shape
    color
    stops {
      at
      color
    }
    shadows {
      x
      y
      radius
      color
    }
    image {
      ...CosmeticImageFields
    }
  }
`);

export const COSMETICS = graphql(`
  query Cosmetics {
    cosmetics {
      badges {
        ...BadgeFields
      }
      paints {
        ...PaintFields
      }
    }
  }
`);

export const USER_COSMETICS = graphql(`
  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {
    userCosmetics(platform: $platform, userIds: $userIds) {
      platformUserId
      badgeId
      paintId
    }
  }
`);

// What accounts show (badge and paint, in full), by account id: public, up to 100 at once.
export const ACCOUNT_COSMETICS = graphql(`
  query AccountCosmetics($accountIds: [String!]!) {
    accountCosmetics(accountIds: $accountIds) {
      accountId
      badge {
        ...BadgeFields
      }
      paint {
        ...PaintFields
      }
    }
  }
`);

// An account's public profile: logins, public roles, each channel's active set, member since.
export const ACCOUNT_PROFILE = graphql(`
  query AccountProfile($accountId: String!) {
    accountProfile(accountId: $accountId) {
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
      roles {
        id
        name
      }
      activeSets {
        platform
        login
        setId
        setName
      }
      createdAt
    }
  }
`);

export const MY_COSMETICS = graphql(`
  query MyCosmetics($accountId: String) {
    myCosmetics(accountId: $accountId) {
      account {
        id
        displayName
        avatar
      }
      badges {
        ...BadgeFields
      }
      paints {
        ...PaintFields
      }
      badgeId
      paintId
    }
  }
`);

export const SELECT_COSMETICS = graphql(`
  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {
    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {
      badgeId
      paintId
    }
  }
`);

export const COSMETIC_IMAGE = graphql(`
  query CosmeticImage($id: String!) {
    cosmeticImage(id: $id) {
      ...CosmeticImageFields
    }
  }
`);

export const ENTITLEMENTS = graphql(`
  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {
    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {
      total
      items {
        account {
          id
          displayName
          avatar
        }
        kind
        cosmeticId
        source
        reason
        grantedBy {
          id
          displayName
        }
        grantedAt
      }
    }
  }
`);

export const CREATE_BADGE = graphql(`
  mutation CreateBadge($input: CreateBadgeInput!) {
    createBadge(input: $input) {
      ...BadgeFields
    }
  }
`);

export const UPDATE_BADGE = graphql(`
  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {
    updateBadge(id: $id, input: $input) {
      ...BadgeFields
    }
  }
`);

export const DELETE_BADGE = graphql(`
  mutation DeleteBadge($id: String!) {
    deleteBadge(id: $id)
  }
`);

export const DELETE_PAINT = graphql(`
  mutation DeletePaint($id: String!) {
    deletePaint(id: $id)
  }
`);

export const GRANT_COSMETIC = graphql(`
  mutation GrantCosmetic($input: GrantCosmeticInput!) {
    grantCosmetic(input: $input) {
      cosmeticId
    }
  }
`);

export const REVOKE_COSMETIC = graphql(`
  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {
    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)
  }
`);

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

// An account's public activity feed (ADR 0016), newest first; page back with `before`.
export const ACCOUNT_ACTIVITY = graphql(`
  query AccountActivity($accountId: String!, $before: String, $limit: Int) {
    accountActivity(accountId: $accountId, before: $before, limit: $limit) {
      id
      kind
      actor {
        id
        displayName
        avatar
      }
      set {
        id
        name
      }
      emote {
        id
        name
        host
        files {
          name
          width
          height
          isStatic
        }
      }
      fromName
      toName
      badge {
        fromId
        fromName
        toId
        toName
      }
      paint {
        fromId
        fromName
        toId
        toName
      }
      createdAt
    }
  }
`);

export const EMOTE_ACTIVITY = graphql(`
  query EmoteActivity($emoteId: String!, $before: String, $limit: Int) {
    emoteActivity(emoteId: $emoteId, before: $before, limit: $limit) {
      id
      kind
      actor {
        id
        displayName
        avatar
      }
      set {
        id
        name
      }
      emote {
        id
        name
        host
        files {
          name
          width
          height
          isStatic
        }
      }
      fromName
      toName
      badge {
        fromId
        fromName
        toId
        toName
      }
      paint {
        fromId
        fromName
        toId
        toName
      }
      createdAt
    }
  }
`);
