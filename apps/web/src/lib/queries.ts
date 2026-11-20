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
    visibility
    flags {
      zeroWidth
      nsfw
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
    $nsfw: Boolean
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
      nsfw: $nsfw
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

export const ME = graphql(`
  query Me {
    me {
      id
      createdAt
      permissions
      displayPlatform
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

export const REMOVE_SET_EDITOR = graphql(`
  mutation RemoveSetEditor($setId: String!, $accountId: String!) {
    removeSetEditor(setId: $setId, accountId: $accountId)
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

// Which linked platform's name and picture you go by (Settings → Account).
export const SET_DISPLAY_PLATFORM = graphql(`
  mutation SetDisplayPlatform($platform: Platform!) {
    setDisplayPlatform(platform: $platform) {
      id
      displayPlatform
    }
  }
`);

// Settings → Account → Security: every session ends, this one too.
export const SIGN_OUT_EVERYWHERE = graphql(`
  mutation SignOutEverywhere {
    signOutEverywhere
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
      transferState
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

// The 30-day undo window (ADR 0019): only the session that signing in to a deleted account gave.
export const PENDING_DELETION = graphql(`
  query PendingDeletion {
    pendingDeletion {
      requestedAt
      eraseAfter
      displayName
      avatar
    }
  }
`);

export const RESTORE_ACCOUNT = graphql(`
  mutation RestoreAccount {
    restoreAccount
  }
`);

// Your own roles, hidden ones too (only for your own profile).
export const MY_ROLES = graphql(`
  query MyRoles {
    me {
      id
      roles {
        id
        name
        public
      }
    }
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
  query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {
    reviewQueue(page: $page, perPage: $perPage, filter: $filter) {
      total
      counts {
        public
        limited
      }
      items {
        ...EmoteFields
      }
    }
  }
`);

export const APPROVE = graphql(`
  mutation Approve($id: String!, $visibility: EmoteVisibility) {
    approveEmote(id: $id, visibility: $visibility) {
      id
      state
      visibility
    }
  }
`);

export const EMOTE_HISTORY = graphql(`
  query EmoteHistory($emoteId: String!) {
    emoteHistory(emoteId: $emoteId) {
      id
      actorId
      action
      before
      after
      createdAt
    }
  }
`);

export const EMOTE_NOTES = graphql(`
  query EmoteNotes($emoteId: String!) {
    emoteNotes(emoteId: $emoteId) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const ADD_EMOTE_NOTE = graphql(`
  mutation AddEmoteNote($emoteId: String!, $body: String!) {
    addEmoteNote(emoteId: $emoteId, body: $body) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const DELETE_EMOTE_NOTE = graphql(`
  mutation DeleteEmoteNote($id: String!) {
    deleteEmoteNote(id: $id)
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

export const REPORT_FIELDS = graphql(`
  fragment ReportFields on Report {
    id
    reason
    reporterId
    state
    resolution
    createdAt
    resolvedAt
    emote {
      ...EmoteFields
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

export const ROLE_FIELDS = graphql(`
  fragment RoleFields on Role {
    id
    name
    description
    position
    permissions
    public
    builtIn
    editable
    assignable
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
      ticketsBlocked
      blocked
      uploadsBlockedUntil
      ticketsBlockedUntil
      blockedUntil
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

export const PLACE_RESTRICTION = graphql(`
  mutation PlaceRestriction($accountId: String!, $input: RestrictionInput!) {
    placeRestriction(accountId: $accountId, input: $input) {
      uploadsBlocked
      ticketsBlocked
      blocked
      uploadsBlockedUntil
      ticketsBlockedUntil
      blockedUntil
    }
  }
`);

export const LIFT_RESTRICTION = graphql(`
  mutation LiftRestriction($accountId: String!, $kind: RestrictionKind!, $reason: String) {
    liftRestriction(accountId: $accountId, kind: $kind, reason: $reason) {
      uploadsBlocked
      ticketsBlocked
      blocked
      uploadsBlockedUntil
      ticketsBlockedUntil
      blockedUntil
    }
  }
`);

export const ACCOUNT_RESTRICTIONS = graphql(`
  query AccountRestrictions($accountId: String!) {
    accountRestrictions(accountId: $accountId) {
      id
      kind
      reason
      endsAt
      createdBy
      createdAt
      liftedAt
      liftedBy
      active
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
  query Audit(
    $targetKind: String
    $targetId: String
    $actorId: String
    $action: String
    $since: String
    $until: String
    $before: String
    $limit: Int
  ) {
    auditLog(
      targetKind: $targetKind
      targetId: $targetId
      actorId: $actorId
      action: $action
      since: $since
      until: $until
      before: $before
      limit: $limit
    ) {
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

// The admin account page: the account as staff see it, its uploads, and its reports.
export const ADMIN_ACCOUNT = graphql(`
  query AdminAccount($accountId: String!) {
    adminAccount(accountId: $accountId) {
      id
      displayName
      avatar
      createdAt
      uploadsBlocked
      ticketsBlocked
      blocked
      uploadsBlockedUntil
      ticketsBlockedUntil
      blockedUntil
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

export const TRANSFER_EMOTE = graphql(`
  mutation TransferEmote($id: String!, $toAccountId: String!) {
    transferEmote(id: $id, toAccountId: $toAccountId) {
      ...EmoteFields
    }
  }
`);

export const TRANSFER_REQUEST_FIELDS = graphql(`
  fragment TransferRequestFields on EmoteTransferRequest {
    id
    emoteId
    emoteName
    state
    createdAt
    expiresAt
    from {
      id
      displayName
    }
    to {
      id
      displayName
    }
    requestedBy {
      id
      displayName
    }
  }
`);

export const EMOTE_TRANSFER_REQUESTS = graphql(`
  query EmoteTransferRequests($emoteId: String) {
    emoteTransferRequests(emoteId: $emoteId) {
      ...TransferRequestFields
    }
  }
`);

export const ACCEPT_EMOTE_TRANSFER = graphql(`
  mutation AcceptEmoteTransfer($id: String!) {
    acceptEmoteTransfer(id: $id) {
      ...EmoteFields
    }
  }
`);

export const DECLINE_EMOTE_TRANSFER = graphql(`
  mutation DeclineEmoteTransfer($id: String!) {
    declineEmoteTransfer(id: $id) {
      ...TransferRequestFields
    }
  }
`);

export const CANCEL_EMOTE_TRANSFER = graphql(`
  mutation CancelEmoteTransfer($id: String!) {
    cancelEmoteTransfer(id: $id) {
      ...TransferRequestFields
    }
  }
`);

export const ACCOUNT_EMOTES = graphql(`
  query AccountEmotes($accountId: String!, $page: Int, $perPage: Int) {
    accountEmotes(accountId: $accountId, page: $page, perPage: $perPage) {
      total
      items {
        ...EmoteFields
      }
    }
  }
`);

export const ACCOUNT_REPORTS = graphql(`
  query AccountReports($accountId: String!) {
    accountReports(accountId: $accountId) {
      about {
        ...ReportFields
      }
      by {
        ...ReportFields
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
  query EmoteChannels($id: String!, $limit: Int, $offset: Int) {
    emoteChannels(id: $id, limit: $limit, offset: $offset) {
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
      editors {
        id
        displayName
        avatar
      }
      createdAt
      collectionPublic
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

// An account's public activity feed (ADR 0016), newest first; page back with `before` set to the
// answer's `next` (a page can be short and still have one).
export const ACCOUNT_ACTIVITY = graphql(`
  query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {
    accountActivityPage(accountId: $accountId, before: $before, limit: $limit, filter: $filter) {
      entries {
        id
        kind
        actor {
          id
          displayName
          avatar
        }
        editor {
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
      next
    }
  }
`);

export const EMOTE_ACTIVITY = graphql(`
  query EmoteActivity($emoteId: String!, $before: String, $limit: Int) {
    emoteActivityPage(emoteId: $emoteId, before: $before, limit: $limit) {
      entries {
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
      next
    }
  }
`);

// The shop, redeem codes and orders (ADR 0016).

export const SHOP_ITEM_FIELDS = graphql(`
  fragment ShopItemFields on ShopItem {
    id
    name
    description
    priceCents
    currency
    paddlePriceId
    badges {
      ...BadgeFields
    }
    paints {
      ...PaintFields
    }
    activeFrom
    activeTo
    stockLimit
    stockLeft
    visible
    sortOrder
    available
    owned
    deleted
  }
`);

export const SHOP_ITEMS = graphql(`
  query ShopItems($all: Boolean) {
    shopItems(all: $all) {
      ...ShopItemFields
    }
  }
`);

export const SHOP_ITEM = graphql(`
  query ShopItem($id: String!) {
    shopItem(id: $id) {
      ...ShopItemFields
    }
  }
`);

export const ORDER_FIELDS = graphql(`
  fragment OrderFields on Order {
    id
    buyer {
      id
      displayName
      avatar
    }
    recipient {
      id
      displayName
      avatar
    }
    gift
    itemId
    itemName
    status
    amountCents
    currency
    paidTotalCents
    paidCurrency
    refundReason
    createdAt
    paidAt
    refundedAt
    failedAt
  }
`);

export const START_CHECKOUT = graphql(`
  mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {
    startCheckout(itemId: $itemId, recipient: $recipient, idempotencyKey: $idempotencyKey) {
      order {
        ...OrderFields
      }
      transactionId
      checkoutUrl
      clientToken
      environment
    }
  }
`);

export const ORDER = graphql(`
  query Order($id: String!) {
    order(id: $id) {
      ...OrderFields
    }
  }
`);

export const MY_ORDERS = graphql(`
  query MyOrders($page: Int, $perPage: Int) {
    myOrders(page: $page, perPage: $perPage) {
      total
      items {
        ...OrderFields
      }
    }
  }
`);

export const REDEEM_CODE = graphql(`
  mutation RedeemCode($code: String!, $idempotencyKey: String) {
    redeemCode(code: $code, idempotencyKey: $idempotencyKey) {
      name
      badges {
        ...BadgeFields
      }
      paints {
        ...PaintFields
      }
    }
  }
`);

export const CREATE_SHOP_ITEM = graphql(`
  mutation CreateShopItem($input: ShopItemInput!) {
    createShopItem(input: $input) {
      ...ShopItemFields
    }
  }
`);

export const UPDATE_SHOP_ITEM = graphql(`
  mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {
    updateShopItem(id: $id, input: $input) {
      ...ShopItemFields
    }
  }
`);

export const DELETE_SHOP_ITEM = graphql(`
  mutation DeleteShopItem($id: String!) {
    deleteShopItem(id: $id)
  }
`);

export const ADMIN_ORDERS = graphql(`
  query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {
    orders(status: $status, accountId: $accountId, itemId: $itemId, page: $page, perPage: $perPage) {
      total
      items {
        ...OrderFields
        providerTransactionId
        refundRejectedAt
      }
    }
  }
`);

export const REFUND_ORDER = graphql(`
  mutation RefundOrder($id: String!, $reason: String!) {
    refundOrder(id: $id, reason: $reason) {
      ...OrderFields
      providerTransactionId
      refundRejectedAt
    }
  }
`);

export const CODE_BATCH_FIELDS = graphql(`
  fragment CodeBatchFields on CodeBatch {
    id
    name
    prefix
    itemId
    itemName
    badgeIds
    paintIds
    codeCount
    usesPerCode
    redemptions
    expiresAt
    disabledAt
    createdBy {
      id
      displayName
    }
    createdAt
  }
`);

export const CODE_BATCHES = graphql(`
  query CodeBatches($page: Int, $perPage: Int) {
    codeBatches(page: $page, perPage: $perPage) {
      total
      items {
        ...CodeBatchFields
      }
    }
  }
`);

export const CREATE_CODE_BATCH = graphql(`
  mutation CreateCodeBatch($input: CodeBatchInput!) {
    createCodeBatch(input: $input) {
      batch {
        ...CodeBatchFields
      }
      codes
    }
  }
`);

export const DISABLE_CODE_BATCH = graphql(`
  mutation DisableCodeBatch($id: String!) {
    disableCodeBatch(id: $id) {
      ...CodeBatchFields
    }
  }
`);

export const SHOP_COUNTS = graphql(`
  query ShopCounts($items: Boolean!, $orders: Boolean!, $codes: Boolean!) {
    shopItems(all: true) @include(if: $items) {
      id
    }
    orders(page: 1, perPage: 1) @include(if: $orders) {
      total
    }
    codeBatches(page: 1, perPage: 1) @include(if: $codes) {
      total
    }
  }
`);

// The header search: the top emotes by the name matcher and the top people, in one request.
export const QUICK_SEARCH = graphql(`
  query QuickSearch($query: String!) {
    quickSearch(query: $query) {
      emotes {
        ...EmoteFields
      }
      users {
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

// What the empty header search lists: the channels you can manage and your role on each.
export const EDITABLE_CHANNELS = graphql(`
  query EditableChannels {
    myEditableChannels {
      role
      channel {
        platform
        login
        account {
          id
          displayName
          avatar
        }
      }
    }
  }
`);

// The staff account dashboard and the entitlement graph (ADR 0010 amendment 2026/11/19).

export const STAFF_NOTES = graphql(`
  query StaffNotes($accountId: String!) {
    staffNotes(accountId: $accountId) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const ADD_STAFF_NOTE = graphql(`
  mutation AddStaffNote($accountId: String!, $body: String!) {
    addStaffNote(accountId: $accountId, body: $body) {
      id
      authorId
      body
      createdAt
    }
  }
`);

export const DELETE_STAFF_NOTE = graphql(`
  mutation DeleteStaffNote($id: String!) {
    deleteStaffNote(id: $id)
  }
`);

export const ACCOUNT_SESSIONS = graphql(`
  query AccountSessions($accountId: String!) {
    accountSessions(accountId: $accountId) {
      id
      kind
      device
      createdAt
      lastSeenAt
      expiresAt
    }
  }
`);

export const ACCOUNT_SUBSCRIPTIONS = graphql(`
  query AccountSubscriptions($accountId: String!) {
    accountSubscriptions(accountId: $accountId) {
      id
      plan
      state
      startsAt
      endsAt
      source
    }
  }
`);

export const GRANT_FIELDS = graphql(`
  fragment GrantFields on EntitlementGrant {
    kind
    itemId
    itemName
    accountId
    source
    reason
    grantedBy
    grantedAt
    orderId
    orderName
    orderStatus
    codeBatchId
    codeBatchName
    revokedAt
    revokedBy
    revokeReason
    active
  }
`);

export const ENTITLEMENT_GRAPH = graphql(`
  query EntitlementGraph($accountId: String!) {
    entitlementGraph(accountId: $accountId) {
      ...GrantFields
    }
  }
`);

export const ENTITLEMENT_HOLDERS = graphql(`
  query EntitlementHolders($kind: GrantKind!, $id: String!) {
    entitlementHolders(kind: $kind, id: $id) {
      ...GrantFields
    }
  }
`);

// Review queue: batch verdicts, possible duplicates, ready reasons; bulk report dismissals;
// pasted logins for grants to many; the uploader's same-file check.

export const REVIEW_EMOTES = graphql(`
  mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {
    reviewEmotes(ids: $ids, verdict: $verdict, visibility: $visibility, reason: $reason) {
      done
      failed {
        id
        code
        message
      }
    }
  }
`);

export const DISMISS_REPORTS = graphql(`
  mutation DismissReports($ids: [String!]!, $note: String) {
    dismissReports(ids: $ids, note: $note) {
      done
      failed {
        id
        code
        message
      }
    }
  }
`);

export const EMOTE_DUPLICATES = graphql(`
  query EmoteDuplicates($emoteId: String!) {
    emoteDuplicates(emoteId: $emoteId) {
      sameFile
      similarity
      emote {
        ...EmoteFields
      }
    }
  }
`);

export const REVIEW_REASONS = graphql(`
  query ReviewReasons {
    reviewReasons
  }
`);

export const SET_REVIEW_REASONS = graphql(`
  mutation SetReviewReasons($reasons: [String!]!) {
    setReviewReasons(reasons: $reasons)
  }
`);

export const ACCOUNTS_BY_LOGIN = graphql(`
  query AccountsByLogin($logins: [String!]!) {
    accountsByLogin(logins: $logins) {
      login
      problem
      account {
        id
        displayName
        avatar
      }
    }
  }
`);

export const EMOTES_WITH_SAME_FILE = graphql(`
  query EmotesWithSameFile($sha256: String!) {
    emotesWithSameFile(sha256: $sha256) {
      id
      name
      owner {
        id
        displayName
      }
    }
  }
`);

// The site announcement under the header, and its admin page.
export const ANNOUNCEMENT_FIELDS = graphql(`
  fragment AnnouncementFields on Announcement {
    id
    kind
    text
    linkUrl
    linkLabel
    badgeId
    startsAt
    endsAt
    endedAt
    createdAt
    updatedAt
    live
    emote {
      id
      name
      host
      animated
      files {
        name
        width
        height
        isStatic
      }
    }
  }
`);

export const ACTIVE_ANNOUNCEMENT = graphql(`
  query ActiveAnnouncement {
    activeAnnouncement {
      ...AnnouncementFields
    }
  }
`);

export const ANNOUNCEMENTS = graphql(`
  query Announcements {
    announcements {
      ...AnnouncementFields
    }
  }
`);

export const CREATE_ANNOUNCEMENT = graphql(`
  mutation CreateAnnouncement($input: AnnouncementInput!) {
    createAnnouncement(input: $input) {
      ...AnnouncementFields
    }
  }
`);

export const UPDATE_ANNOUNCEMENT = graphql(`
  mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {
    updateAnnouncement(id: $id, input: $input) {
      ...AnnouncementFields
    }
  }
`);

export const END_ANNOUNCEMENT = graphql(`
  mutation EndAnnouncement($id: String!) {
    endAnnouncement(id: $id) {
      ...AnnouncementFields
    }
  }
`);

// Settings → Notifications.
export const NOTIFICATION_PREFS = graphql(`
  query NotificationPrefs {
    myNotificationPrefs {
      kind
      notify
      toast
      required
    }
  }
`);

export const SET_NOTIFICATION_PREFS = graphql(`
  mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {
    setNotificationPrefs(prefs: $prefs) {
      kind
      notify
      toast
      required
    }
  }
`);

// The profile's Collection tab.
export const ACCOUNT_COLLECTION = graphql(`
  query AccountCollection($accountId: String!) {
    accountCollection(accountId: $accountId) {
      accountId
      public
      items {
        kind
        cosmeticId
        name
        ownedSince
        source
      }
    }
  }
`);

export const SET_COLLECTION_PUBLIC = graphql(`
  mutation SetCollectionPublic($public: Boolean!) {
    setCollectionPublic(public: $public) {
      accountId
      public
      items {
        kind
        cosmeticId
        name
        ownedSince
        source
      }
    }
  }
`);

// Personal lists: "Hide for me" (by emote or creator) and favourites. Private to you; the site
// filters the public reads with them on its side.
export const PERSONAL_LISTS = graphql(`
  query PersonalLists {
    hiddenEmotes {
      emoteId
    }
    hiddenCreators {
      accountId
    }
    favouriteEmoteIds
  }
`);

export const HIDDEN_LISTS = graphql(`
  query HiddenLists {
    hiddenEmotes {
      emoteId
      hiddenAt
      emote {
        ...EmoteFields
      }
    }
    hiddenCreators {
      accountId
      hiddenAt
      account {
        id
        displayName
        avatar
      }
    }
  }
`);

export const MY_FAVOURITES = graphql(`
  query MyFavourites($query: String, $page: Int, $perPage: Int) {
    myFavourites(query: $query, page: $page, perPage: $perPage) {
      total
      items {
        ...EmoteFields
      }
    }
  }
`);

export const HIDE_EMOTE = graphql(`
  mutation HideEmote($id: String!) {
    hideEmote(id: $id)
  }
`);

export const UNHIDE_EMOTE = graphql(`
  mutation UnhideEmote($id: String!) {
    unhideEmote(id: $id)
  }
`);

export const HIDE_CREATOR = graphql(`
  mutation HideCreator($accountId: String!) {
    hideCreator(accountId: $accountId)
  }
`);

export const UNHIDE_CREATOR = graphql(`
  mutation UnhideCreator($accountId: String!) {
    unhideCreator(accountId: $accountId)
  }
`);

export const FAVOURITE_EMOTE = graphql(`
  mutation FavouriteEmote($id: String!) {
    favouriteEmote(id: $id)
  }
`);

export const UNFAVOURITE_EMOTE = graphql(`
  mutation UnfavouriteEmote($id: String!) {
    unfavouriteEmote(id: $id)
  }
`);
