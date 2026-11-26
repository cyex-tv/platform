/* eslint-disable */
import * as types from './graphql';



/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 * Learn more about it here: https://the-guild.dev/graphql/codegen/plugins/presets/preset-client#reducing-bundle-size
 */
type Documents = {
    "\n  fragment StudioPaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n    updatedAt\n  }\n": typeof types.StudioPaintFieldsFragmentDoc,
    "\n  query StudioPaints {\n    cosmetics {\n      paints {\n        ...StudioPaintFields\n      }\n    }\n  }\n": typeof types.StudioPaintsDocument,
    "\n  mutation StudioCreatePaint($input: PaintInput!) {\n    createPaint(input: $input) {\n      ...StudioPaintFields\n    }\n  }\n": typeof types.StudioCreatePaintDocument,
    "\n  mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {\n    updatePaint(id: $id, input: $input) {\n      ...StudioPaintFields\n    }\n  }\n": typeof types.StudioUpdatePaintDocument,
    "\n  query StudioCosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n  }\n": typeof types.StudioCosmeticImageDocument,
    "\n  mutation SetDisplayPlatform($platform: Platform!) {\n    setDisplayPlatform(platform: $platform) {\n      id\n      displayPlatform\n    }\n  }\n": typeof types.SetDisplayPlatformDocument,
    "\n  mutation SignOutEverywhere {\n    signOutEverywhere\n  }\n": typeof types.SignOutEverywhereDocument,
    "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n": typeof types.UnlinkDocument,
    "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n": typeof types.MyNotificationsDocument,
    "\n  mutation MarkRead($ids: [String!]) {\n    markNotificationsRead(ids: $ids)\n  }\n": typeof types.MarkReadDocument,
    "\n  query UnreadCount {\n    unreadNotificationCount\n  }\n": typeof types.UnreadCountDocument,
    "\n  query MySessions {\n    mySessions {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n      current\n    }\n  }\n": typeof types.MySessionsDocument,
    "\n  query MyConsoleToken {\n    myConsoleToken {\n      id\n      createdAt\n      expiresAt\n    }\n  }\n": typeof types.MyConsoleTokenDocument,
    "\n  mutation RevokeSession($id: String!) {\n    revokeSession(id: $id)\n  }\n": typeof types.RevokeSessionDocument,
    "\n  mutation RevokeOtherSessions {\n    revokeOtherSessions\n  }\n": typeof types.RevokeOtherSessionsDocument,
    "\n  mutation ExportMyData {\n    exportMyData\n  }\n": typeof types.ExportMyDataDocument,
    "\n  mutation DeleteAccount($confirm: String!) {\n    deleteAccount(confirm: $confirm)\n  }\n": typeof types.DeleteAccountDocument,
    "\n  query PendingDeletion {\n    pendingDeletion {\n      requestedAt\n      eraseAfter\n      displayName\n      avatar\n    }\n  }\n": typeof types.PendingDeletionDocument,
    "\n  mutation RestoreAccount {\n    restoreAccount\n  }\n": typeof types.RestoreAccountDocument,
    "\n  query MyRoles {\n    me {\n      id\n      roles {\n        id\n        name\n        public\n        invisible\n      }\n    }\n  }\n": typeof types.MyRolesDocument,
    "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      displayPlatform\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.MeDocument,
    "\n  query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {\n    accountActivityPage(accountId: $accountId, before: $before, limit: $limit, filter: $filter) {\n      entries {\n        id\n        kind\n        actor {\n          id\n          displayName\n          avatar\n        }\n        editor {\n          id\n          displayName\n          avatar\n        }\n        set {\n          id\n          name\n        }\n        emote {\n          id\n          name\n          host\n          files {\n            name\n            width\n            height\n            isStatic\n          }\n        }\n        fromName\n        toName\n        badge {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        paint {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        createdAt\n      }\n      next\n    }\n  }\n": typeof types.AccountActivityDocument,
    "\n  query EmoteActivity($emoteId: String!, $before: String, $limit: Int, $actorId: String) {\n    emoteActivityPage(emoteId: $emoteId, before: $before, limit: $limit, actorId: $actorId) {\n      entries {\n        id\n        kind\n        actor {\n          id\n          displayName\n          avatar\n        }\n        set {\n          id\n          name\n        }\n        emote {\n          id\n          name\n          host\n          files {\n            name\n            width\n            height\n            isStatic\n          }\n        }\n        fromName\n        toName\n        badge {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        paint {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        createdAt\n      }\n      next\n    }\n  }\n": typeof types.EmoteActivityDocument,
    "\n  query User($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.UserDocument,
    "\n  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {\n    accountStatus(accountId: $accountId) @include(if: $status) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n    accountRoles(accountId: $accountId) @include(if: $roles) {\n      ...RoleFields\n    }\n  }\n": typeof types.UserDetailsDocument,
    "\n  query AccountPermissions($accountId: String!) {\n    accountPermissions(accountId: $accountId) {\n      permission\n      granted\n      effect\n      roleId\n      roleName\n      owner\n    }\n  }\n": typeof types.AccountPermissionsDocument,
    "\n  query AllRoles {\n    roles {\n      ...RoleFields\n    }\n  }\n": typeof types.AllRolesDocument,
    "\n  mutation PlaceRestriction($accountId: String!, $input: RestrictionInput!) {\n    placeRestriction(accountId: $accountId, input: $input) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n  }\n": typeof types.PlaceRestrictionDocument,
    "\n  mutation LiftRestriction($accountId: String!, $kind: RestrictionKind!, $reason: String) {\n    liftRestriction(accountId: $accountId, kind: $kind, reason: $reason) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n  }\n": typeof types.LiftRestrictionDocument,
    "\n  query AccountRestrictions($accountId: String!) {\n    accountRestrictions(accountId: $accountId) {\n      id\n      kind\n      reason\n      endsAt\n      createdBy\n      createdAt\n      liftedAt\n      liftedBy\n      active\n    }\n  }\n": typeof types.AccountRestrictionsDocument,
    "\n  mutation AssignRole($accountId: String!, $roleId: String!) {\n    assignRole(accountId: $accountId, roleId: $roleId)\n  }\n": typeof types.AssignRoleDocument,
    "\n  mutation UnassignRole($accountId: String!, $roleId: String!) {\n    unassignRole(accountId: $accountId, roleId: $roleId)\n  }\n": typeof types.UnassignRoleDocument,
    "\n  query Audit(\n    $targetKind: String\n    $targetId: String\n    $actorId: String\n    $action: String\n    $since: String\n    $until: String\n    $before: String\n    $limit: Int\n  ) {\n    auditLog(\n      targetKind: $targetKind\n      targetId: $targetId\n      actorId: $actorId\n      action: $action\n      since: $since\n      until: $until\n      before: $before\n      limit: $limit\n    ) {\n      id\n      actorId\n      action\n      targetKind\n      targetId\n      before\n      after\n      createdAt\n    }\n  }\n": typeof types.AuditDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.ChannelDocument,
    "\n  fragment CosmeticImageFields on CosmeticImage {\n    id\n    kind\n    host\n    animated\n    ready\n    width\n    height\n    files {\n      name\n      width\n      height\n      isStatic\n    }\n  }\n": typeof types.CosmeticImageFieldsFragmentDoc,
    "\n  fragment BadgeFields on Badge {\n    id\n    name\n    tooltip\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.BadgeFieldsFragmentDoc,
    "\n  fragment PaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.PaintFieldsFragmentDoc,
    "\n  query Cosmetics {\n    cosmetics {\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n": typeof types.CosmeticsDocument,
    "\n  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {\n    userCosmetics(platform: $platform, userIds: $userIds) {\n      platformUserId\n      badgeId\n      paintId\n    }\n  }\n": typeof types.UserCosmeticsDocument,
    "\n  query AccountCosmetics($accountIds: [String!]!) {\n    accountCosmetics(accountIds: $accountIds) {\n      accountId\n      badge {\n        ...BadgeFields\n      }\n      paint {\n        ...PaintFields\n      }\n    }\n  }\n": typeof types.AccountCosmeticsDocument,
    "\n  query AccountProfile($accountId: String!) {\n    accountProfile(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      identities {\n        platform\n        login\n        displayName\n      }\n      roles {\n        id\n        name\n      }\n      activeSets {\n        platform\n        login\n        setId\n        setName\n      }\n      editors {\n        id\n        displayName\n        avatar\n      }\n      createdAt\n    }\n  }\n": typeof types.AccountProfileDocument,
    "\n  query MyCosmetics($accountId: String) {\n    myCosmetics(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n      badgeId\n      paintId\n    }\n  }\n": typeof types.MyCosmeticsDocument,
    "\n  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {\n    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {\n      badgeId\n      paintId\n    }\n  }\n": typeof types.SelectCosmeticsDocument,
    "\n  query CosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.CosmeticImageDocument,
    "\n  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {\n    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {\n      total\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        kind\n        cosmeticId\n        source\n        reason\n        grantedBy {\n          id\n          displayName\n        }\n        grantedAt\n      }\n    }\n  }\n": typeof types.EntitlementsDocument,
    "\n  mutation CreateBadge($input: CreateBadgeInput!) {\n    createBadge(input: $input) {\n      ...BadgeFields\n    }\n  }\n": typeof types.CreateBadgeDocument,
    "\n  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {\n    updateBadge(id: $id, input: $input) {\n      ...BadgeFields\n    }\n  }\n": typeof types.UpdateBadgeDocument,
    "\n  mutation RemakeCosmeticImage($id: String!) {\n    remakeCosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.RemakeCosmeticImageDocument,
    "\n  mutation DeleteBadge($id: String!) {\n    deleteBadge(id: $id)\n  }\n": typeof types.DeleteBadgeDocument,
    "\n  mutation DeletePaint($id: String!) {\n    deletePaint(id: $id)\n  }\n": typeof types.DeletePaintDocument,
    "\n  mutation GrantCosmetic($input: GrantCosmeticInput!) {\n    grantCosmetic(input: $input) {\n      cosmeticId\n    }\n  }\n": typeof types.GrantCosmeticDocument,
    "\n  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {\n    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)\n  }\n": typeof types.RevokeCosmeticDocument,
    "\n  fragment AccountEditorFields on AccountEditor {\n    account {\n      id\n      displayName\n      avatar\n    }\n    permissions\n    allSets\n    setIds\n    grantedBy {\n      id\n      displayName\n    }\n    grantedAt\n  }\n": typeof types.AccountEditorFieldsFragmentDoc,
    "\n  query Editors {\n    accountEditors {\n      ...AccountEditorFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      allSets\n      setIds\n      grantedAt\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n  }\n": typeof types.EditorsDocument,
    "\n  mutation AddAccountEditor($input: AddAccountEditorInput!) {\n    addAccountEditor(input: $input) {\n      ...AccountEditorFields\n    }\n  }\n": typeof types.AddAccountEditorDocument,
    "\n  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) {\n    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions, setIds: $setIds) {\n      ...AccountEditorFields\n    }\n  }\n": typeof types.UpdateAccountEditorDocument,
    "\n  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {\n    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)\n  }\n": typeof types.RemoveAccountEditorDocument,
    "\n  mutation ReportEmote($id: String!, $reason: String!) {\n    reportEmote(id: $id, reason: $reason) {\n      id\n    }\n  }\n": typeof types.ReportEmoteDocument,
    "\n  query MySetNames {\n    mySets {\n      id\n      name\n      kind\n      emotes {\n        emote {\n          id\n        }\n      }\n    }\n  }\n": typeof types.MySetNamesDocument,
    "\n  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort, $after: String) {\n    searchEmotes(query: \"\", owner: $owner, page: $page, perPage: $perPage, sort: $sort, after: $after) {\n      total\n      page\n      perPage\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SearchByOwnerDocument,
    "\n  query AdminAccount($accountId: String!) {\n    adminAccount(accountId: $accountId) {\n      id\n      displayName\n      avatar\n      createdAt\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.AdminAccountDocument,
    "\n  mutation TransferEmote($id: String!, $toAccountId: String!) {\n    transferEmote(id: $id, toAccountId: $toAccountId) {\n      ...EmoteFields\n    }\n  }\n": typeof types.TransferEmoteDocument,
    "\n  query AccountEmotes($accountId: String!, $page: Int, $perPage: Int, $after: String) {\n    accountEmotes(accountId: $accountId, page: $page, perPage: $perPage, after: $after) {\n      total\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.AccountEmotesDocument,
    "\n  query AccountReports($accountId: String!) {\n    accountReports(accountId: $accountId) {\n      about {\n        ...ReportFields\n      }\n      by {\n        ...ReportFields\n      }\n    }\n  }\n": typeof types.AccountReportsDocument,
    "\n  query PublicAccount($id: String!) {\n    account(id: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": typeof types.PublicAccountDocument,
    "\n  query EmoteChannels($id: String!, $limit: Int, $offset: Int) {\n    emoteChannels(id: $id, limit: $limit, offset: $offset) {\n      platform\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.EmoteChannelsDocument,
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    processing\n    animated\n    tags\n    host\n    createdAt\n    channelCount\n    visibility\n    flags {\n      zeroWidth\n      private\n      wide\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": typeof types.EmoteFieldsFragmentDoc,
    "\n  query Search(\n    $query: String!\n    $animated: Boolean\n    $page: Int\n    $perPage: Int\n    $sort: EmoteSort\n    $flags: [EmoteFlag!]\n    $tags: [String!]\n    $exact: Boolean\n    $window: TrendingWindow\n    $after: String\n  ) {\n    searchEmotes(\n      query: $query\n      animated: $animated\n      page: $page\n      perPage: $perPage\n      sort: $sort\n      flags: $flags\n      tags: $tags\n      exact: $exact\n      window: $window\n      after: $after\n    ) {\n      total\n      page\n      perPage\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SearchDocument,
    "\n  query FindPeople($query: String!, $page: Int, $perPage: Int, $after: String) {\n    searchAccounts(query: $query, page: $page, perPage: $perPage, after: $after) {\n      total\n      page\n      perPage\n      next\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n": typeof types.FindPeopleDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": typeof types.EmoteDocument,
    "\n  query EmoteTrending($id: String!, $window: TrendingWindow) {\n    emoteTrending(id: $id, window: $window) {\n      rank\n      channels\n      window\n    }\n  }\n": typeof types.EmoteTrendingDocument,
    "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    editorSets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n": typeof types.MyChannelsDocument,
    "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n": typeof types.UpdateChannelDocument,
    "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.MyEmotesDocument,
    "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": typeof types.UpdateEmoteDocument,
    "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n": typeof types.DeleteEmoteDocument,
    "\n  query PersonalLists {\n    hiddenEmotes {\n      emoteId\n    }\n    hiddenCreators {\n      accountId\n    }\n    favouriteEmoteIds\n  }\n": typeof types.PersonalListsDocument,
    "\n  query HiddenLists {\n    hiddenEmotes {\n      emoteId\n      hiddenAt\n      emote {\n        ...EmoteFields\n      }\n    }\n    hiddenCreators {\n      accountId\n      hiddenAt\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.HiddenListsDocument,
    "\n  query MyFavourites($query: String, $page: Int, $perPage: Int) {\n    myFavourites(query: $query, page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.MyFavouritesDocument,
    "\n  mutation HideEmote($id: String!) {\n    hideEmote(id: $id)\n  }\n": typeof types.HideEmoteDocument,
    "\n  mutation UnhideEmote($id: String!) {\n    unhideEmote(id: $id)\n  }\n": typeof types.UnhideEmoteDocument,
    "\n  mutation HideCreator($accountId: String!) {\n    hideCreator(accountId: $accountId)\n  }\n": typeof types.HideCreatorDocument,
    "\n  mutation UnhideCreator($accountId: String!) {\n    unhideCreator(accountId: $accountId)\n  }\n": typeof types.UnhideCreatorDocument,
    "\n  mutation FavouriteEmote($id: String!) {\n    favouriteEmote(id: $id)\n  }\n": typeof types.FavouriteEmoteDocument,
    "\n  mutation UnfavouriteEmote($id: String!) {\n    unfavouriteEmote(id: $id)\n  }\n": typeof types.UnfavouriteEmoteDocument,
    "\n  query AccountRoles($accountId: String!) {\n    accountRoles(accountId: $accountId) {\n      id\n      name\n      position\n    }\n  }\n": typeof types.AccountRolesDocument,
    "\n  query AccountEditors($accountId: String) {\n    accountEditors(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n": typeof types.AccountEditorsDocument,
    "\n  query EditorOf {\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      allSets\n    }\n  }\n": typeof types.EditorOfDocument,
    "\n  query AccountSets($accountId: String!) {\n    accountSets(accountId: $accountId, preview: 12) {\n      id\n      name\n      kind\n      capacity\n      slotsUsed\n      active\n      preview {\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.AccountSetsDocument,
    "\n  query ProfileMine {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n    }\n    mySets {\n      ...SetFields\n    }\n  }\n": typeof types.ProfileMineDocument,
    "\n  query ProfileEdited {\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorSets {\n      ...SetFields\n    }\n  }\n": typeof types.ProfileEditedDocument,
    "\n  fragment ReportFields on Report {\n    id\n    reason\n    reporterId\n    state\n    resolution\n    createdAt\n    resolvedAt\n    emote {\n      ...EmoteFields\n    }\n  }\n": typeof types.ReportFieldsFragmentDoc,
    "\n  query ReportQueue($page: Int, $perPage: Int) {\n    reportQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        id\n        reason\n        reporterId\n        state\n        createdAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.ReportQueueDocument,
    "\n  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {\n    resolveReport(id: $id, action: $action, note: $note) {\n      id\n      state\n    }\n  }\n": typeof types.ResolveReportDocument,
    "\n  mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {\n    reviewEmotes(ids: $ids, verdict: $verdict, visibility: $visibility, reason: $reason) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n": typeof types.ReviewEmotesDocument,
    "\n  mutation RequeueEmotes($ids: [String!]!) {\n    requeueEmotes(ids: $ids) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n": typeof types.RequeueEmotesDocument,
    "\n  mutation SetEmoteFlags($id: String!, $input: EmoteFlagsInput!) {\n    setEmoteFlags(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": typeof types.SetEmoteFlagsDocument,
    "\n  mutation DismissReports($ids: [String!]!, $note: String) {\n    dismissReports(ids: $ids, note: $note) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n": typeof types.DismissReportsDocument,
    "\n  query EmoteDuplicates($emoteId: String!) {\n    emoteDuplicates(emoteId: $emoteId) {\n      sameFile\n      similarity\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.EmoteDuplicatesDocument,
    "\n  query ReviewReasons {\n    reviewReasons\n  }\n": typeof types.ReviewReasonsDocument,
    "\n  mutation SetReviewReasons($reasons: [String!]!) {\n    setReviewReasons(reasons: $reasons)\n  }\n": typeof types.SetReviewReasonsDocument,
    "\n  query AccountsByLogin($logins: [String!]!) {\n    accountsByLogin(logins: $logins) {\n      login\n      problem\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.AccountsByLoginDocument,
    "\n  query EmotesWithSameFile($sha256: String!) {\n    emotesWithSameFile(sha256: $sha256) {\n      id\n      name\n      owner {\n        id\n        displayName\n      }\n    }\n  }\n": typeof types.EmotesWithSameFileDocument,
    "\n  query AdminCounts($review: Boolean!, $reports: Boolean!) {\n    reviewQueue(page: 1, perPage: 1) @include(if: $review) {\n      total\n    }\n    reportQueue(page: 1, perPage: 1) @include(if: $reports) {\n      total\n    }\n    globalSet {\n      id\n    }\n  }\n": typeof types.AdminCountsDocument,
    "\n  query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {\n    reviewQueue(page: $page, perPage: $perPage, filter: $filter) {\n      total\n      counts {\n        public\n        limited\n      }\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.ReviewQueueDocument,
    "\n  mutation Approve($id: String!, $visibility: EmoteVisibility) {\n    approveEmote(id: $id, visibility: $visibility) {\n      id\n      state\n      visibility\n    }\n  }\n": typeof types.ApproveDocument,
    "\n  query EmoteHistory($emoteId: String!) {\n    emoteHistory(emoteId: $emoteId) {\n      id\n      actorId\n      action\n      before\n      after\n      createdAt\n    }\n  }\n": typeof types.EmoteHistoryDocument,
    "\n  query EmoteNotes($emoteId: String!) {\n    emoteNotes(emoteId: $emoteId) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": typeof types.EmoteNotesDocument,
    "\n  mutation AddEmoteNote($emoteId: String!, $body: String!) {\n    addEmoteNote(emoteId: $emoteId, body: $body) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": typeof types.AddEmoteNoteDocument,
    "\n  mutation DeleteEmoteNote($id: String!) {\n    deleteEmoteNote(id: $id)\n  }\n": typeof types.DeleteEmoteNoteDocument,
    "\n  mutation Reject($id: String!, $reason: String!) {\n    rejectEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n": typeof types.RejectDocument,
    "\n  fragment RoleFields on Role {\n    id\n    name\n    description\n    position\n    permissions\n    denies\n    public\n    invisible\n    builtIn\n    editable\n    assignable\n  }\n": typeof types.RoleFieldsFragmentDoc,
    "\n  query Roles {\n    roles {\n      ...RoleFields\n    }\n    permissions\n  }\n": typeof types.RolesDocument,
    "\n  fragment RoleLimitFields on RoleLimit {\n    key\n    group\n    default\n    max\n    windowSeconds\n    value\n  }\n": typeof types.RoleLimitFieldsFragmentDoc,
    "\n  query LimitDefaults {\n    limitDefaults {\n      ...RoleLimitFields\n    }\n  }\n": typeof types.LimitDefaultsDocument,
    "\n  query RoleLimits($id: String!) {\n    roleLimits(id: $id) {\n      ...RoleLimitFields\n    }\n  }\n": typeof types.RoleLimitsDocument,
    "\n  mutation SetRoleLimits($id: String!, $limits: [RoleLimitInput!]!) {\n    setRoleLimits(id: $id, limits: $limits) {\n      ...RoleLimitFields\n    }\n  }\n": typeof types.SetRoleLimitsDocument,
    "\n  query UploadQuota($ownerId: String) {\n    uploadQuota(ownerId: $ownerId) {\n      accountId\n      pending\n      pendingMax\n      uploadsThisHour\n      uploadsPerHour\n      maxBytes\n      maxSide\n      maxFrames\n      maxFps\n      maxAspect\n    }\n  }\n": typeof types.UploadQuotaDocument,
    "\n  fragment MediaLimitFields on MediaLimits {\n    maxBytes\n    maxSide\n    maxFrames\n    maxFps\n    maxTotalPixels\n    maxDurationMs\n    minSide\n    maxAspect\n    maxServedSide\n  }\n": typeof types.MediaLimitFieldsFragmentDoc,
    "\n  query PublicLimits {\n    limits {\n      emote { ...MediaLimitFields }\n      badge { ...MediaLimitFields }\n      paint { ...MediaLimitFields }\n      names {\n        emote { min max }\n        set { min max }\n        badge { min max }\n        paint { min max }\n        tooltip { min max }\n      }\n      tags { max maxLen }\n      rates { key count windowSeconds }\n      account { key default effective windowSeconds }\n    }\n  }\n": typeof types.PublicLimitsDocument,
    "\n  mutation CreateRole($input: CreateRoleInput!) {\n    createRole(input: $input) {\n      ...RoleFields\n    }\n  }\n": typeof types.CreateRoleDocument,
    "\n  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {\n    updateRole(id: $id, input: $input) {\n      ...RoleFields\n    }\n  }\n": typeof types.UpdateRoleDocument,
    "\n  mutation DeleteRole($id: String!) {\n    deleteRole(id: $id)\n  }\n": typeof types.DeleteRoleDocument,
    "\n  query QuickSearch($query: String!) {\n    quickSearch(query: $query) {\n      emotes {\n        ...EmoteFields\n      }\n      users {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n": typeof types.QuickSearchDocument,
    "\n  query EditableChannels {\n    myEditableChannels {\n      role\n      channel {\n        platform\n        login\n        account {\n          id\n          displayName\n          avatar\n        }\n      }\n    }\n  }\n": typeof types.EditableChannelsDocument,
    "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    visibility\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SetFieldsFragmentDoc,
    "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n    editorSets {\n      ...SetFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n      }\n      permissions\n    }\n  }\n": typeof types.MySetsDocument,
    "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      owner {\n        id\n        displayName\n        avatar\n      }\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n      allSets\n      setIds\n    }\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n  }\n": typeof types.EditSetDocument,
    "\n  query SetEmotes($id: String!, $query: String, $page: Int, $perPage: Int) {\n    setEmotes(id: $id, query: $query, page: $page, perPage: $perPage) {\n      total\n      items {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.SetEmotesDocument,
    "\n  query SetMembers($id: String!, $page: Int, $perPage: Int) {\n    setEmotes(id: $id, page: $page, perPage: $perPage) {\n      total\n      items {\n        name\n        emote {\n          id\n        }\n      }\n    }\n  }\n": typeof types.SetMembersDocument,
    "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n": typeof types.CreateSetDocument,
    "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n      visibility\n      capacity\n    }\n  }\n": typeof types.UpdateSetDocument,
    "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n": typeof types.DeleteSetDocument,
    "\n  query SetCapacityRange($id: String!) {\n    setCapacityRange(id: $id) {\n      min\n      max\n      ownerMax\n      used\n      staff\n    }\n  }\n": typeof types.SetCapacityRangeDocument,
    "\n  query SetMenu($id: String!) {\n    setCapacityRange(id: $id) {\n      min\n      max\n      ownerMax\n      used\n      staff\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n      allSets\n      setIds\n    }\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n  }\n": typeof types.SetMenuDocument,
    "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n": typeof types.AddSetEmoteDocument,
    "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n": typeof types.UpdateSetEmoteDocument,
    "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n": typeof types.RemoveSetEmoteDocument,
    "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n": typeof types.RemoveSetEditorDocument,
    "\n  query PublicSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      createdAt\n      owner {\n        id\n        displayName\n        avatar\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.PublicSetDocument,
    "\n  query SetEditors($id: String!) {\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": typeof types.SetEditorsDocument,
    "\n  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {\n    cloneSet(id: $id, name: $name, visibility: $visibility) {\n      copied\n      skipped\n      set {\n        id\n        name\n      }\n    }\n  }\n": typeof types.CloneSetDocument,
    "\n  fragment ShopItemFields on ShopItem {\n    id\n    name\n    description\n    priceCents\n    currency\n    paddlePriceId\n    badges {\n      ...BadgeFields\n    }\n    paints {\n      ...PaintFields\n    }\n    activeFrom\n    activeTo\n    stockLimit\n    stockLeft\n    visible\n    sortOrder\n    available\n    owned\n    deleted\n  }\n": typeof types.ShopItemFieldsFragmentDoc,
    "\n  query ShopItems($all: Boolean) {\n    shopItems(all: $all) {\n      ...ShopItemFields\n    }\n  }\n": typeof types.ShopItemsDocument,
    "\n  query ShopItem($id: String!) {\n    shopItem(id: $id) {\n      ...ShopItemFields\n    }\n  }\n": typeof types.ShopItemDocument,
    "\n  fragment OrderFields on Order {\n    id\n    buyer {\n      id\n      displayName\n      avatar\n    }\n    recipient {\n      id\n      displayName\n      avatar\n    }\n    gift\n    itemId\n    itemName\n    status\n    amountCents\n    currency\n    paidTotalCents\n    paidCurrency\n    refundReason\n    createdAt\n    paidAt\n    refundedAt\n    failedAt\n  }\n": typeof types.OrderFieldsFragmentDoc,
    "\n  mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {\n    startCheckout(itemId: $itemId, recipient: $recipient, idempotencyKey: $idempotencyKey) {\n      order {\n        ...OrderFields\n      }\n      transactionId\n      checkoutUrl\n      clientToken\n      environment\n    }\n  }\n": typeof types.StartCheckoutDocument,
    "\n  query Order($id: String!) {\n    order(id: $id) {\n      ...OrderFields\n    }\n  }\n": typeof types.OrderDocument,
    "\n  query MyOrders($page: Int, $perPage: Int) {\n    myOrders(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...OrderFields\n      }\n    }\n  }\n": typeof types.MyOrdersDocument,
    "\n  mutation RedeemCode($code: String!, $idempotencyKey: String) {\n    redeemCode(code: $code, idempotencyKey: $idempotencyKey) {\n      name\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n": typeof types.RedeemCodeDocument,
    "\n  mutation CreateShopItem($input: ShopItemInput!) {\n    createShopItem(input: $input) {\n      ...ShopItemFields\n    }\n  }\n": typeof types.CreateShopItemDocument,
    "\n  mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {\n    updateShopItem(id: $id, input: $input) {\n      ...ShopItemFields\n    }\n  }\n": typeof types.UpdateShopItemDocument,
    "\n  mutation DeleteShopItem($id: String!) {\n    deleteShopItem(id: $id)\n  }\n": typeof types.DeleteShopItemDocument,
    "\n  query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {\n    orders(status: $status, accountId: $accountId, itemId: $itemId, page: $page, perPage: $perPage) {\n      total\n      items {\n        ...OrderFields\n        providerTransactionId\n        refundRejectedAt\n      }\n    }\n  }\n": typeof types.AdminOrdersDocument,
    "\n  mutation RefundOrder($id: String!, $reason: String!) {\n    refundOrder(id: $id, reason: $reason) {\n      ...OrderFields\n      providerTransactionId\n      refundRejectedAt\n    }\n  }\n": typeof types.RefundOrderDocument,
    "\n  fragment CodeBatchFields on CodeBatch {\n    id\n    name\n    prefix\n    itemId\n    itemName\n    badgeIds\n    paintIds\n    codeCount\n    usesPerCode\n    redemptions\n    expiresAt\n    disabledAt\n    createdBy {\n      id\n      displayName\n    }\n    createdAt\n  }\n": typeof types.CodeBatchFieldsFragmentDoc,
    "\n  query CodeBatches($page: Int, $perPage: Int) {\n    codeBatches(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...CodeBatchFields\n      }\n    }\n  }\n": typeof types.CodeBatchesDocument,
    "\n  mutation CreateCodeBatch($input: CodeBatchInput!) {\n    createCodeBatch(input: $input) {\n      batch {\n        ...CodeBatchFields\n      }\n      codes\n    }\n  }\n": typeof types.CreateCodeBatchDocument,
    "\n  mutation DisableCodeBatch($id: String!) {\n    disableCodeBatch(id: $id) {\n      ...CodeBatchFields\n    }\n  }\n": typeof types.DisableCodeBatchDocument,
    "\n  query ShopCounts($items: Boolean!, $orders: Boolean!, $codes: Boolean!) {\n    shopItems(all: true) @include(if: $items) {\n      id\n    }\n    orders(page: 1, perPage: 1) @include(if: $orders) {\n      total\n    }\n    codeBatches(page: 1, perPage: 1) @include(if: $codes) {\n      total\n    }\n  }\n": typeof types.ShopCountsDocument,
    "\n  fragment AnnouncementFields on Announcement {\n    id\n    kind\n    text\n    linkUrl\n    linkLabel\n    badgeId\n    startsAt\n    endsAt\n    endedAt\n    createdAt\n    updatedAt\n    live\n    emote {\n      id\n      name\n      host\n      animated\n      files {\n        name\n        width\n        height\n        isStatic\n      }\n    }\n  }\n": typeof types.AnnouncementFieldsFragmentDoc,
    "\n  query ActiveAnnouncement {\n    activeAnnouncement {\n      ...AnnouncementFields\n    }\n  }\n": typeof types.ActiveAnnouncementDocument,
    "\n  query Announcements {\n    announcements {\n      ...AnnouncementFields\n    }\n  }\n": typeof types.AnnouncementsDocument,
    "\n  mutation CreateAnnouncement($input: AnnouncementInput!) {\n    createAnnouncement(input: $input) {\n      ...AnnouncementFields\n    }\n  }\n": typeof types.CreateAnnouncementDocument,
    "\n  mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {\n    updateAnnouncement(id: $id, input: $input) {\n      ...AnnouncementFields\n    }\n  }\n": typeof types.UpdateAnnouncementDocument,
    "\n  mutation EndAnnouncement($id: String!) {\n    endAnnouncement(id: $id) {\n      ...AnnouncementFields\n    }\n  }\n": typeof types.EndAnnouncementDocument,
    "\n  query NotificationPrefs {\n    myNotificationPrefs {\n      kind\n      notify\n      toast\n      required\n    }\n  }\n": typeof types.NotificationPrefsDocument,
    "\n  mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {\n    setNotificationPrefs(prefs: $prefs) {\n      kind\n      notify\n      toast\n      required\n    }\n  }\n": typeof types.SetNotificationPrefsDocument,
    "\n  query StaffNotes($accountId: String!) {\n    staffNotes(accountId: $accountId) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": typeof types.StaffNotesDocument,
    "\n  mutation AddStaffNote($accountId: String!, $body: String!) {\n    addStaffNote(accountId: $accountId, body: $body) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": typeof types.AddStaffNoteDocument,
    "\n  mutation DeleteStaffNote($id: String!) {\n    deleteStaffNote(id: $id)\n  }\n": typeof types.DeleteStaffNoteDocument,
    "\n  query AccountSessions($accountId: String!) {\n    accountSessions(accountId: $accountId) {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n    }\n  }\n": typeof types.AccountSessionsDocument,
    "\n  query AccountSubscriptions($accountId: String!) {\n    accountSubscriptions(accountId: $accountId) {\n      id\n      plan\n      state\n      startsAt\n      endsAt\n      source\n    }\n  }\n": typeof types.AccountSubscriptionsDocument,
    "\n  fragment GrantFields on EntitlementGrant {\n    kind\n    itemId\n    itemName\n    accountId\n    source\n    reason\n    grantedBy\n    grantedAt\n    orderId\n    orderName\n    orderStatus\n    codeBatchId\n    codeBatchName\n    revokedAt\n    revokedBy\n    revokeReason\n    active\n  }\n": typeof types.GrantFieldsFragmentDoc,
    "\n  query EntitlementGraph($accountId: String!) {\n    entitlementGraph(accountId: $accountId) {\n      ...GrantFields\n    }\n  }\n": typeof types.EntitlementGraphDocument,
    "\n  query EntitlementHolders($kind: GrantKind!, $id: String!) {\n    entitlementHolders(kind: $kind, id: $id) {\n      ...GrantFields\n    }\n  }\n": typeof types.EntitlementHoldersDocument,
    "\n  fragment PersonCountFields on PersonCount {\n    account {\n      id\n      displayName\n      avatar\n    }\n    identities {\n      platform\n      login\n      displayName\n    }\n    count\n  }\n": typeof types.PersonCountFieldsFragmentDoc,
    "\n  query ActivityTopActors($accountId: String, $emoteId: String, $filter: ActivityFilter, $limit: Int) {\n    activityTopActors(accountId: $accountId, emoteId: $emoteId, filter: $filter, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": typeof types.ActivityTopActorsDocument,
    "\n  query AuditTopPeople(\n    $by: AuditPersonRole\n    $targetKind: String\n    $targetId: String\n    $actorId: String\n    $action: String\n    $since: String\n    $until: String\n    $limit: Int\n  ) {\n    auditTopPeople(\n      by: $by\n      targetKind: $targetKind\n      targetId: $targetId\n      actorId: $actorId\n      action: $action\n      since: $since\n      until: $until\n      limit: $limit\n    ) {\n      ...PersonCountFields\n    }\n  }\n": typeof types.AuditTopPeopleDocument,
    "\n  query OrderTopBuyers($status: OrderStatus, $itemId: String, $limit: Int) {\n    orderTopBuyers(status: $status, itemId: $itemId, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": typeof types.OrderTopBuyersDocument,
    "\n  query ReviewTopUploaders($filter: ReviewQueueFilter, $limit: Int) {\n    reviewTopUploaders(filter: $filter, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": typeof types.ReviewTopUploadersDocument,
    "\n  query EntitlementTopHolders($limit: Int) {\n    entitlementTopHolders(limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": typeof types.EntitlementTopHoldersDocument,
};
const documents: Documents = {
    "\n  fragment StudioPaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n    updatedAt\n  }\n": types.StudioPaintFieldsFragmentDoc,
    "\n  query StudioPaints {\n    cosmetics {\n      paints {\n        ...StudioPaintFields\n      }\n    }\n  }\n": types.StudioPaintsDocument,
    "\n  mutation StudioCreatePaint($input: PaintInput!) {\n    createPaint(input: $input) {\n      ...StudioPaintFields\n    }\n  }\n": types.StudioCreatePaintDocument,
    "\n  mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {\n    updatePaint(id: $id, input: $input) {\n      ...StudioPaintFields\n    }\n  }\n": types.StudioUpdatePaintDocument,
    "\n  query StudioCosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n  }\n": types.StudioCosmeticImageDocument,
    "\n  mutation SetDisplayPlatform($platform: Platform!) {\n    setDisplayPlatform(platform: $platform) {\n      id\n      displayPlatform\n    }\n  }\n": types.SetDisplayPlatformDocument,
    "\n  mutation SignOutEverywhere {\n    signOutEverywhere\n  }\n": types.SignOutEverywhereDocument,
    "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n": types.UnlinkDocument,
    "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n": types.MyNotificationsDocument,
    "\n  mutation MarkRead($ids: [String!]) {\n    markNotificationsRead(ids: $ids)\n  }\n": types.MarkReadDocument,
    "\n  query UnreadCount {\n    unreadNotificationCount\n  }\n": types.UnreadCountDocument,
    "\n  query MySessions {\n    mySessions {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n      current\n    }\n  }\n": types.MySessionsDocument,
    "\n  query MyConsoleToken {\n    myConsoleToken {\n      id\n      createdAt\n      expiresAt\n    }\n  }\n": types.MyConsoleTokenDocument,
    "\n  mutation RevokeSession($id: String!) {\n    revokeSession(id: $id)\n  }\n": types.RevokeSessionDocument,
    "\n  mutation RevokeOtherSessions {\n    revokeOtherSessions\n  }\n": types.RevokeOtherSessionsDocument,
    "\n  mutation ExportMyData {\n    exportMyData\n  }\n": types.ExportMyDataDocument,
    "\n  mutation DeleteAccount($confirm: String!) {\n    deleteAccount(confirm: $confirm)\n  }\n": types.DeleteAccountDocument,
    "\n  query PendingDeletion {\n    pendingDeletion {\n      requestedAt\n      eraseAfter\n      displayName\n      avatar\n    }\n  }\n": types.PendingDeletionDocument,
    "\n  mutation RestoreAccount {\n    restoreAccount\n  }\n": types.RestoreAccountDocument,
    "\n  query MyRoles {\n    me {\n      id\n      roles {\n        id\n        name\n        public\n        invisible\n      }\n    }\n  }\n": types.MyRolesDocument,
    "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      displayPlatform\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": types.MeDocument,
    "\n  query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {\n    accountActivityPage(accountId: $accountId, before: $before, limit: $limit, filter: $filter) {\n      entries {\n        id\n        kind\n        actor {\n          id\n          displayName\n          avatar\n        }\n        editor {\n          id\n          displayName\n          avatar\n        }\n        set {\n          id\n          name\n        }\n        emote {\n          id\n          name\n          host\n          files {\n            name\n            width\n            height\n            isStatic\n          }\n        }\n        fromName\n        toName\n        badge {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        paint {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        createdAt\n      }\n      next\n    }\n  }\n": types.AccountActivityDocument,
    "\n  query EmoteActivity($emoteId: String!, $before: String, $limit: Int, $actorId: String) {\n    emoteActivityPage(emoteId: $emoteId, before: $before, limit: $limit, actorId: $actorId) {\n      entries {\n        id\n        kind\n        actor {\n          id\n          displayName\n          avatar\n        }\n        set {\n          id\n          name\n        }\n        emote {\n          id\n          name\n          host\n          files {\n            name\n            width\n            height\n            isStatic\n          }\n        }\n        fromName\n        toName\n        badge {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        paint {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        createdAt\n      }\n      next\n    }\n  }\n": types.EmoteActivityDocument,
    "\n  query User($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.UserDocument,
    "\n  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {\n    accountStatus(accountId: $accountId) @include(if: $status) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n    accountRoles(accountId: $accountId) @include(if: $roles) {\n      ...RoleFields\n    }\n  }\n": types.UserDetailsDocument,
    "\n  query AccountPermissions($accountId: String!) {\n    accountPermissions(accountId: $accountId) {\n      permission\n      granted\n      effect\n      roleId\n      roleName\n      owner\n    }\n  }\n": types.AccountPermissionsDocument,
    "\n  query AllRoles {\n    roles {\n      ...RoleFields\n    }\n  }\n": types.AllRolesDocument,
    "\n  mutation PlaceRestriction($accountId: String!, $input: RestrictionInput!) {\n    placeRestriction(accountId: $accountId, input: $input) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n  }\n": types.PlaceRestrictionDocument,
    "\n  mutation LiftRestriction($accountId: String!, $kind: RestrictionKind!, $reason: String) {\n    liftRestriction(accountId: $accountId, kind: $kind, reason: $reason) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n  }\n": types.LiftRestrictionDocument,
    "\n  query AccountRestrictions($accountId: String!) {\n    accountRestrictions(accountId: $accountId) {\n      id\n      kind\n      reason\n      endsAt\n      createdBy\n      createdAt\n      liftedAt\n      liftedBy\n      active\n    }\n  }\n": types.AccountRestrictionsDocument,
    "\n  mutation AssignRole($accountId: String!, $roleId: String!) {\n    assignRole(accountId: $accountId, roleId: $roleId)\n  }\n": types.AssignRoleDocument,
    "\n  mutation UnassignRole($accountId: String!, $roleId: String!) {\n    unassignRole(accountId: $accountId, roleId: $roleId)\n  }\n": types.UnassignRoleDocument,
    "\n  query Audit(\n    $targetKind: String\n    $targetId: String\n    $actorId: String\n    $action: String\n    $since: String\n    $until: String\n    $before: String\n    $limit: Int\n  ) {\n    auditLog(\n      targetKind: $targetKind\n      targetId: $targetId\n      actorId: $actorId\n      action: $action\n      since: $since\n      until: $until\n      before: $before\n      limit: $limit\n    ) {\n      id\n      actorId\n      action\n      targetKind\n      targetId\n      before\n      after\n      createdAt\n    }\n  }\n": types.AuditDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.ChannelDocument,
    "\n  fragment CosmeticImageFields on CosmeticImage {\n    id\n    kind\n    host\n    animated\n    ready\n    width\n    height\n    files {\n      name\n      width\n      height\n      isStatic\n    }\n  }\n": types.CosmeticImageFieldsFragmentDoc,
    "\n  fragment BadgeFields on Badge {\n    id\n    name\n    tooltip\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": types.BadgeFieldsFragmentDoc,
    "\n  fragment PaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": types.PaintFieldsFragmentDoc,
    "\n  query Cosmetics {\n    cosmetics {\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n": types.CosmeticsDocument,
    "\n  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {\n    userCosmetics(platform: $platform, userIds: $userIds) {\n      platformUserId\n      badgeId\n      paintId\n    }\n  }\n": types.UserCosmeticsDocument,
    "\n  query AccountCosmetics($accountIds: [String!]!) {\n    accountCosmetics(accountIds: $accountIds) {\n      accountId\n      badge {\n        ...BadgeFields\n      }\n      paint {\n        ...PaintFields\n      }\n    }\n  }\n": types.AccountCosmeticsDocument,
    "\n  query AccountProfile($accountId: String!) {\n    accountProfile(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      identities {\n        platform\n        login\n        displayName\n      }\n      roles {\n        id\n        name\n      }\n      activeSets {\n        platform\n        login\n        setId\n        setName\n      }\n      editors {\n        id\n        displayName\n        avatar\n      }\n      createdAt\n    }\n  }\n": types.AccountProfileDocument,
    "\n  query MyCosmetics($accountId: String) {\n    myCosmetics(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n      badgeId\n      paintId\n    }\n  }\n": types.MyCosmeticsDocument,
    "\n  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {\n    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {\n      badgeId\n      paintId\n    }\n  }\n": types.SelectCosmeticsDocument,
    "\n  query CosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n": types.CosmeticImageDocument,
    "\n  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {\n    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {\n      total\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        kind\n        cosmeticId\n        source\n        reason\n        grantedBy {\n          id\n          displayName\n        }\n        grantedAt\n      }\n    }\n  }\n": types.EntitlementsDocument,
    "\n  mutation CreateBadge($input: CreateBadgeInput!) {\n    createBadge(input: $input) {\n      ...BadgeFields\n    }\n  }\n": types.CreateBadgeDocument,
    "\n  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {\n    updateBadge(id: $id, input: $input) {\n      ...BadgeFields\n    }\n  }\n": types.UpdateBadgeDocument,
    "\n  mutation RemakeCosmeticImage($id: String!) {\n    remakeCosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n": types.RemakeCosmeticImageDocument,
    "\n  mutation DeleteBadge($id: String!) {\n    deleteBadge(id: $id)\n  }\n": types.DeleteBadgeDocument,
    "\n  mutation DeletePaint($id: String!) {\n    deletePaint(id: $id)\n  }\n": types.DeletePaintDocument,
    "\n  mutation GrantCosmetic($input: GrantCosmeticInput!) {\n    grantCosmetic(input: $input) {\n      cosmeticId\n    }\n  }\n": types.GrantCosmeticDocument,
    "\n  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {\n    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)\n  }\n": types.RevokeCosmeticDocument,
    "\n  fragment AccountEditorFields on AccountEditor {\n    account {\n      id\n      displayName\n      avatar\n    }\n    permissions\n    allSets\n    setIds\n    grantedBy {\n      id\n      displayName\n    }\n    grantedAt\n  }\n": types.AccountEditorFieldsFragmentDoc,
    "\n  query Editors {\n    accountEditors {\n      ...AccountEditorFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      allSets\n      setIds\n      grantedAt\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n  }\n": types.EditorsDocument,
    "\n  mutation AddAccountEditor($input: AddAccountEditorInput!) {\n    addAccountEditor(input: $input) {\n      ...AccountEditorFields\n    }\n  }\n": types.AddAccountEditorDocument,
    "\n  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) {\n    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions, setIds: $setIds) {\n      ...AccountEditorFields\n    }\n  }\n": types.UpdateAccountEditorDocument,
    "\n  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {\n    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)\n  }\n": types.RemoveAccountEditorDocument,
    "\n  mutation ReportEmote($id: String!, $reason: String!) {\n    reportEmote(id: $id, reason: $reason) {\n      id\n    }\n  }\n": types.ReportEmoteDocument,
    "\n  query MySetNames {\n    mySets {\n      id\n      name\n      kind\n      emotes {\n        emote {\n          id\n        }\n      }\n    }\n  }\n": types.MySetNamesDocument,
    "\n  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort, $after: String) {\n    searchEmotes(query: \"\", owner: $owner, page: $page, perPage: $perPage, sort: $sort, after: $after) {\n      total\n      page\n      perPage\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SearchByOwnerDocument,
    "\n  query AdminAccount($accountId: String!) {\n    adminAccount(accountId: $accountId) {\n      id\n      displayName\n      avatar\n      createdAt\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": types.AdminAccountDocument,
    "\n  mutation TransferEmote($id: String!, $toAccountId: String!) {\n    transferEmote(id: $id, toAccountId: $toAccountId) {\n      ...EmoteFields\n    }\n  }\n": types.TransferEmoteDocument,
    "\n  query AccountEmotes($accountId: String!, $page: Int, $perPage: Int, $after: String) {\n    accountEmotes(accountId: $accountId, page: $page, perPage: $perPage, after: $after) {\n      total\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.AccountEmotesDocument,
    "\n  query AccountReports($accountId: String!) {\n    accountReports(accountId: $accountId) {\n      about {\n        ...ReportFields\n      }\n      by {\n        ...ReportFields\n      }\n    }\n  }\n": types.AccountReportsDocument,
    "\n  query PublicAccount($id: String!) {\n    account(id: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": types.PublicAccountDocument,
    "\n  query EmoteChannels($id: String!, $limit: Int, $offset: Int) {\n    emoteChannels(id: $id, limit: $limit, offset: $offset) {\n      platform\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.EmoteChannelsDocument,
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    processing\n    animated\n    tags\n    host\n    createdAt\n    channelCount\n    visibility\n    flags {\n      zeroWidth\n      private\n      wide\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": types.EmoteFieldsFragmentDoc,
    "\n  query Search(\n    $query: String!\n    $animated: Boolean\n    $page: Int\n    $perPage: Int\n    $sort: EmoteSort\n    $flags: [EmoteFlag!]\n    $tags: [String!]\n    $exact: Boolean\n    $window: TrendingWindow\n    $after: String\n  ) {\n    searchEmotes(\n      query: $query\n      animated: $animated\n      page: $page\n      perPage: $perPage\n      sort: $sort\n      flags: $flags\n      tags: $tags\n      exact: $exact\n      window: $window\n      after: $after\n    ) {\n      total\n      page\n      perPage\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SearchDocument,
    "\n  query FindPeople($query: String!, $page: Int, $perPage: Int, $after: String) {\n    searchAccounts(query: $query, page: $page, perPage: $perPage, after: $after) {\n      total\n      page\n      perPage\n      next\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n": types.FindPeopleDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": types.EmoteDocument,
    "\n  query EmoteTrending($id: String!, $window: TrendingWindow) {\n    emoteTrending(id: $id, window: $window) {\n      rank\n      channels\n      window\n    }\n  }\n": types.EmoteTrendingDocument,
    "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    editorSets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n": types.MyChannelsDocument,
    "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n": types.UpdateChannelDocument,
    "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.MyEmotesDocument,
    "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": types.UpdateEmoteDocument,
    "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n": types.DeleteEmoteDocument,
    "\n  query PersonalLists {\n    hiddenEmotes {\n      emoteId\n    }\n    hiddenCreators {\n      accountId\n    }\n    favouriteEmoteIds\n  }\n": types.PersonalListsDocument,
    "\n  query HiddenLists {\n    hiddenEmotes {\n      emoteId\n      hiddenAt\n      emote {\n        ...EmoteFields\n      }\n    }\n    hiddenCreators {\n      accountId\n      hiddenAt\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.HiddenListsDocument,
    "\n  query MyFavourites($query: String, $page: Int, $perPage: Int) {\n    myFavourites(query: $query, page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.MyFavouritesDocument,
    "\n  mutation HideEmote($id: String!) {\n    hideEmote(id: $id)\n  }\n": types.HideEmoteDocument,
    "\n  mutation UnhideEmote($id: String!) {\n    unhideEmote(id: $id)\n  }\n": types.UnhideEmoteDocument,
    "\n  mutation HideCreator($accountId: String!) {\n    hideCreator(accountId: $accountId)\n  }\n": types.HideCreatorDocument,
    "\n  mutation UnhideCreator($accountId: String!) {\n    unhideCreator(accountId: $accountId)\n  }\n": types.UnhideCreatorDocument,
    "\n  mutation FavouriteEmote($id: String!) {\n    favouriteEmote(id: $id)\n  }\n": types.FavouriteEmoteDocument,
    "\n  mutation UnfavouriteEmote($id: String!) {\n    unfavouriteEmote(id: $id)\n  }\n": types.UnfavouriteEmoteDocument,
    "\n  query AccountRoles($accountId: String!) {\n    accountRoles(accountId: $accountId) {\n      id\n      name\n      position\n    }\n  }\n": types.AccountRolesDocument,
    "\n  query AccountEditors($accountId: String) {\n    accountEditors(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n": types.AccountEditorsDocument,
    "\n  query EditorOf {\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      allSets\n    }\n  }\n": types.EditorOfDocument,
    "\n  query AccountSets($accountId: String!) {\n    accountSets(accountId: $accountId, preview: 12) {\n      id\n      name\n      kind\n      capacity\n      slotsUsed\n      active\n      preview {\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.AccountSetsDocument,
    "\n  query ProfileMine {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n    }\n    mySets {\n      ...SetFields\n    }\n  }\n": types.ProfileMineDocument,
    "\n  query ProfileEdited {\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorSets {\n      ...SetFields\n    }\n  }\n": types.ProfileEditedDocument,
    "\n  fragment ReportFields on Report {\n    id\n    reason\n    reporterId\n    state\n    resolution\n    createdAt\n    resolvedAt\n    emote {\n      ...EmoteFields\n    }\n  }\n": types.ReportFieldsFragmentDoc,
    "\n  query ReportQueue($page: Int, $perPage: Int) {\n    reportQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        id\n        reason\n        reporterId\n        state\n        createdAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.ReportQueueDocument,
    "\n  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {\n    resolveReport(id: $id, action: $action, note: $note) {\n      id\n      state\n    }\n  }\n": types.ResolveReportDocument,
    "\n  mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {\n    reviewEmotes(ids: $ids, verdict: $verdict, visibility: $visibility, reason: $reason) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n": types.ReviewEmotesDocument,
    "\n  mutation RequeueEmotes($ids: [String!]!) {\n    requeueEmotes(ids: $ids) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n": types.RequeueEmotesDocument,
    "\n  mutation SetEmoteFlags($id: String!, $input: EmoteFlagsInput!) {\n    setEmoteFlags(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": types.SetEmoteFlagsDocument,
    "\n  mutation DismissReports($ids: [String!]!, $note: String) {\n    dismissReports(ids: $ids, note: $note) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n": types.DismissReportsDocument,
    "\n  query EmoteDuplicates($emoteId: String!) {\n    emoteDuplicates(emoteId: $emoteId) {\n      sameFile\n      similarity\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": types.EmoteDuplicatesDocument,
    "\n  query ReviewReasons {\n    reviewReasons\n  }\n": types.ReviewReasonsDocument,
    "\n  mutation SetReviewReasons($reasons: [String!]!) {\n    setReviewReasons(reasons: $reasons)\n  }\n": types.SetReviewReasonsDocument,
    "\n  query AccountsByLogin($logins: [String!]!) {\n    accountsByLogin(logins: $logins) {\n      login\n      problem\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.AccountsByLoginDocument,
    "\n  query EmotesWithSameFile($sha256: String!) {\n    emotesWithSameFile(sha256: $sha256) {\n      id\n      name\n      owner {\n        id\n        displayName\n      }\n    }\n  }\n": types.EmotesWithSameFileDocument,
    "\n  query AdminCounts($review: Boolean!, $reports: Boolean!) {\n    reviewQueue(page: 1, perPage: 1) @include(if: $review) {\n      total\n    }\n    reportQueue(page: 1, perPage: 1) @include(if: $reports) {\n      total\n    }\n    globalSet {\n      id\n    }\n  }\n": types.AdminCountsDocument,
    "\n  query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {\n    reviewQueue(page: $page, perPage: $perPage, filter: $filter) {\n      total\n      counts {\n        public\n        limited\n      }\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.ReviewQueueDocument,
    "\n  mutation Approve($id: String!, $visibility: EmoteVisibility) {\n    approveEmote(id: $id, visibility: $visibility) {\n      id\n      state\n      visibility\n    }\n  }\n": types.ApproveDocument,
    "\n  query EmoteHistory($emoteId: String!) {\n    emoteHistory(emoteId: $emoteId) {\n      id\n      actorId\n      action\n      before\n      after\n      createdAt\n    }\n  }\n": types.EmoteHistoryDocument,
    "\n  query EmoteNotes($emoteId: String!) {\n    emoteNotes(emoteId: $emoteId) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": types.EmoteNotesDocument,
    "\n  mutation AddEmoteNote($emoteId: String!, $body: String!) {\n    addEmoteNote(emoteId: $emoteId, body: $body) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": types.AddEmoteNoteDocument,
    "\n  mutation DeleteEmoteNote($id: String!) {\n    deleteEmoteNote(id: $id)\n  }\n": types.DeleteEmoteNoteDocument,
    "\n  mutation Reject($id: String!, $reason: String!) {\n    rejectEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n": types.RejectDocument,
    "\n  fragment RoleFields on Role {\n    id\n    name\n    description\n    position\n    permissions\n    denies\n    public\n    invisible\n    builtIn\n    editable\n    assignable\n  }\n": types.RoleFieldsFragmentDoc,
    "\n  query Roles {\n    roles {\n      ...RoleFields\n    }\n    permissions\n  }\n": types.RolesDocument,
    "\n  fragment RoleLimitFields on RoleLimit {\n    key\n    group\n    default\n    max\n    windowSeconds\n    value\n  }\n": types.RoleLimitFieldsFragmentDoc,
    "\n  query LimitDefaults {\n    limitDefaults {\n      ...RoleLimitFields\n    }\n  }\n": types.LimitDefaultsDocument,
    "\n  query RoleLimits($id: String!) {\n    roleLimits(id: $id) {\n      ...RoleLimitFields\n    }\n  }\n": types.RoleLimitsDocument,
    "\n  mutation SetRoleLimits($id: String!, $limits: [RoleLimitInput!]!) {\n    setRoleLimits(id: $id, limits: $limits) {\n      ...RoleLimitFields\n    }\n  }\n": types.SetRoleLimitsDocument,
    "\n  query UploadQuota($ownerId: String) {\n    uploadQuota(ownerId: $ownerId) {\n      accountId\n      pending\n      pendingMax\n      uploadsThisHour\n      uploadsPerHour\n      maxBytes\n      maxSide\n      maxFrames\n      maxFps\n      maxAspect\n    }\n  }\n": types.UploadQuotaDocument,
    "\n  fragment MediaLimitFields on MediaLimits {\n    maxBytes\n    maxSide\n    maxFrames\n    maxFps\n    maxTotalPixels\n    maxDurationMs\n    minSide\n    maxAspect\n    maxServedSide\n  }\n": types.MediaLimitFieldsFragmentDoc,
    "\n  query PublicLimits {\n    limits {\n      emote { ...MediaLimitFields }\n      badge { ...MediaLimitFields }\n      paint { ...MediaLimitFields }\n      names {\n        emote { min max }\n        set { min max }\n        badge { min max }\n        paint { min max }\n        tooltip { min max }\n      }\n      tags { max maxLen }\n      rates { key count windowSeconds }\n      account { key default effective windowSeconds }\n    }\n  }\n": types.PublicLimitsDocument,
    "\n  mutation CreateRole($input: CreateRoleInput!) {\n    createRole(input: $input) {\n      ...RoleFields\n    }\n  }\n": types.CreateRoleDocument,
    "\n  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {\n    updateRole(id: $id, input: $input) {\n      ...RoleFields\n    }\n  }\n": types.UpdateRoleDocument,
    "\n  mutation DeleteRole($id: String!) {\n    deleteRole(id: $id)\n  }\n": types.DeleteRoleDocument,
    "\n  query QuickSearch($query: String!) {\n    quickSearch(query: $query) {\n      emotes {\n        ...EmoteFields\n      }\n      users {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n": types.QuickSearchDocument,
    "\n  query EditableChannels {\n    myEditableChannels {\n      role\n      channel {\n        platform\n        login\n        account {\n          id\n          displayName\n          avatar\n        }\n      }\n    }\n  }\n": types.EditableChannelsDocument,
    "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    visibility\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SetFieldsFragmentDoc,
    "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n    editorSets {\n      ...SetFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n      }\n      permissions\n    }\n  }\n": types.MySetsDocument,
    "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      owner {\n        id\n        displayName\n        avatar\n      }\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n      allSets\n      setIds\n    }\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n  }\n": types.EditSetDocument,
    "\n  query SetEmotes($id: String!, $query: String, $page: Int, $perPage: Int) {\n    setEmotes(id: $id, query: $query, page: $page, perPage: $perPage) {\n      total\n      items {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.SetEmotesDocument,
    "\n  query SetMembers($id: String!, $page: Int, $perPage: Int) {\n    setEmotes(id: $id, page: $page, perPage: $perPage) {\n      total\n      items {\n        name\n        emote {\n          id\n        }\n      }\n    }\n  }\n": types.SetMembersDocument,
    "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n": types.CreateSetDocument,
    "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n      visibility\n      capacity\n    }\n  }\n": types.UpdateSetDocument,
    "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n": types.DeleteSetDocument,
    "\n  query SetCapacityRange($id: String!) {\n    setCapacityRange(id: $id) {\n      min\n      max\n      ownerMax\n      used\n      staff\n    }\n  }\n": types.SetCapacityRangeDocument,
    "\n  query SetMenu($id: String!) {\n    setCapacityRange(id: $id) {\n      min\n      max\n      ownerMax\n      used\n      staff\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n      allSets\n      setIds\n    }\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n  }\n": types.SetMenuDocument,
    "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n": types.AddSetEmoteDocument,
    "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n": types.UpdateSetEmoteDocument,
    "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n": types.RemoveSetEmoteDocument,
    "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n": types.RemoveSetEditorDocument,
    "\n  query PublicSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      createdAt\n      owner {\n        id\n        displayName\n        avatar\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.PublicSetDocument,
    "\n  query SetEditors($id: String!) {\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": types.SetEditorsDocument,
    "\n  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {\n    cloneSet(id: $id, name: $name, visibility: $visibility) {\n      copied\n      skipped\n      set {\n        id\n        name\n      }\n    }\n  }\n": types.CloneSetDocument,
    "\n  fragment ShopItemFields on ShopItem {\n    id\n    name\n    description\n    priceCents\n    currency\n    paddlePriceId\n    badges {\n      ...BadgeFields\n    }\n    paints {\n      ...PaintFields\n    }\n    activeFrom\n    activeTo\n    stockLimit\n    stockLeft\n    visible\n    sortOrder\n    available\n    owned\n    deleted\n  }\n": types.ShopItemFieldsFragmentDoc,
    "\n  query ShopItems($all: Boolean) {\n    shopItems(all: $all) {\n      ...ShopItemFields\n    }\n  }\n": types.ShopItemsDocument,
    "\n  query ShopItem($id: String!) {\n    shopItem(id: $id) {\n      ...ShopItemFields\n    }\n  }\n": types.ShopItemDocument,
    "\n  fragment OrderFields on Order {\n    id\n    buyer {\n      id\n      displayName\n      avatar\n    }\n    recipient {\n      id\n      displayName\n      avatar\n    }\n    gift\n    itemId\n    itemName\n    status\n    amountCents\n    currency\n    paidTotalCents\n    paidCurrency\n    refundReason\n    createdAt\n    paidAt\n    refundedAt\n    failedAt\n  }\n": types.OrderFieldsFragmentDoc,
    "\n  mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {\n    startCheckout(itemId: $itemId, recipient: $recipient, idempotencyKey: $idempotencyKey) {\n      order {\n        ...OrderFields\n      }\n      transactionId\n      checkoutUrl\n      clientToken\n      environment\n    }\n  }\n": types.StartCheckoutDocument,
    "\n  query Order($id: String!) {\n    order(id: $id) {\n      ...OrderFields\n    }\n  }\n": types.OrderDocument,
    "\n  query MyOrders($page: Int, $perPage: Int) {\n    myOrders(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...OrderFields\n      }\n    }\n  }\n": types.MyOrdersDocument,
    "\n  mutation RedeemCode($code: String!, $idempotencyKey: String) {\n    redeemCode(code: $code, idempotencyKey: $idempotencyKey) {\n      name\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n": types.RedeemCodeDocument,
    "\n  mutation CreateShopItem($input: ShopItemInput!) {\n    createShopItem(input: $input) {\n      ...ShopItemFields\n    }\n  }\n": types.CreateShopItemDocument,
    "\n  mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {\n    updateShopItem(id: $id, input: $input) {\n      ...ShopItemFields\n    }\n  }\n": types.UpdateShopItemDocument,
    "\n  mutation DeleteShopItem($id: String!) {\n    deleteShopItem(id: $id)\n  }\n": types.DeleteShopItemDocument,
    "\n  query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {\n    orders(status: $status, accountId: $accountId, itemId: $itemId, page: $page, perPage: $perPage) {\n      total\n      items {\n        ...OrderFields\n        providerTransactionId\n        refundRejectedAt\n      }\n    }\n  }\n": types.AdminOrdersDocument,
    "\n  mutation RefundOrder($id: String!, $reason: String!) {\n    refundOrder(id: $id, reason: $reason) {\n      ...OrderFields\n      providerTransactionId\n      refundRejectedAt\n    }\n  }\n": types.RefundOrderDocument,
    "\n  fragment CodeBatchFields on CodeBatch {\n    id\n    name\n    prefix\n    itemId\n    itemName\n    badgeIds\n    paintIds\n    codeCount\n    usesPerCode\n    redemptions\n    expiresAt\n    disabledAt\n    createdBy {\n      id\n      displayName\n    }\n    createdAt\n  }\n": types.CodeBatchFieldsFragmentDoc,
    "\n  query CodeBatches($page: Int, $perPage: Int) {\n    codeBatches(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...CodeBatchFields\n      }\n    }\n  }\n": types.CodeBatchesDocument,
    "\n  mutation CreateCodeBatch($input: CodeBatchInput!) {\n    createCodeBatch(input: $input) {\n      batch {\n        ...CodeBatchFields\n      }\n      codes\n    }\n  }\n": types.CreateCodeBatchDocument,
    "\n  mutation DisableCodeBatch($id: String!) {\n    disableCodeBatch(id: $id) {\n      ...CodeBatchFields\n    }\n  }\n": types.DisableCodeBatchDocument,
    "\n  query ShopCounts($items: Boolean!, $orders: Boolean!, $codes: Boolean!) {\n    shopItems(all: true) @include(if: $items) {\n      id\n    }\n    orders(page: 1, perPage: 1) @include(if: $orders) {\n      total\n    }\n    codeBatches(page: 1, perPage: 1) @include(if: $codes) {\n      total\n    }\n  }\n": types.ShopCountsDocument,
    "\n  fragment AnnouncementFields on Announcement {\n    id\n    kind\n    text\n    linkUrl\n    linkLabel\n    badgeId\n    startsAt\n    endsAt\n    endedAt\n    createdAt\n    updatedAt\n    live\n    emote {\n      id\n      name\n      host\n      animated\n      files {\n        name\n        width\n        height\n        isStatic\n      }\n    }\n  }\n": types.AnnouncementFieldsFragmentDoc,
    "\n  query ActiveAnnouncement {\n    activeAnnouncement {\n      ...AnnouncementFields\n    }\n  }\n": types.ActiveAnnouncementDocument,
    "\n  query Announcements {\n    announcements {\n      ...AnnouncementFields\n    }\n  }\n": types.AnnouncementsDocument,
    "\n  mutation CreateAnnouncement($input: AnnouncementInput!) {\n    createAnnouncement(input: $input) {\n      ...AnnouncementFields\n    }\n  }\n": types.CreateAnnouncementDocument,
    "\n  mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {\n    updateAnnouncement(id: $id, input: $input) {\n      ...AnnouncementFields\n    }\n  }\n": types.UpdateAnnouncementDocument,
    "\n  mutation EndAnnouncement($id: String!) {\n    endAnnouncement(id: $id) {\n      ...AnnouncementFields\n    }\n  }\n": types.EndAnnouncementDocument,
    "\n  query NotificationPrefs {\n    myNotificationPrefs {\n      kind\n      notify\n      toast\n      required\n    }\n  }\n": types.NotificationPrefsDocument,
    "\n  mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {\n    setNotificationPrefs(prefs: $prefs) {\n      kind\n      notify\n      toast\n      required\n    }\n  }\n": types.SetNotificationPrefsDocument,
    "\n  query StaffNotes($accountId: String!) {\n    staffNotes(accountId: $accountId) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": types.StaffNotesDocument,
    "\n  mutation AddStaffNote($accountId: String!, $body: String!) {\n    addStaffNote(accountId: $accountId, body: $body) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n": types.AddStaffNoteDocument,
    "\n  mutation DeleteStaffNote($id: String!) {\n    deleteStaffNote(id: $id)\n  }\n": types.DeleteStaffNoteDocument,
    "\n  query AccountSessions($accountId: String!) {\n    accountSessions(accountId: $accountId) {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n    }\n  }\n": types.AccountSessionsDocument,
    "\n  query AccountSubscriptions($accountId: String!) {\n    accountSubscriptions(accountId: $accountId) {\n      id\n      plan\n      state\n      startsAt\n      endsAt\n      source\n    }\n  }\n": types.AccountSubscriptionsDocument,
    "\n  fragment GrantFields on EntitlementGrant {\n    kind\n    itemId\n    itemName\n    accountId\n    source\n    reason\n    grantedBy\n    grantedAt\n    orderId\n    orderName\n    orderStatus\n    codeBatchId\n    codeBatchName\n    revokedAt\n    revokedBy\n    revokeReason\n    active\n  }\n": types.GrantFieldsFragmentDoc,
    "\n  query EntitlementGraph($accountId: String!) {\n    entitlementGraph(accountId: $accountId) {\n      ...GrantFields\n    }\n  }\n": types.EntitlementGraphDocument,
    "\n  query EntitlementHolders($kind: GrantKind!, $id: String!) {\n    entitlementHolders(kind: $kind, id: $id) {\n      ...GrantFields\n    }\n  }\n": types.EntitlementHoldersDocument,
    "\n  fragment PersonCountFields on PersonCount {\n    account {\n      id\n      displayName\n      avatar\n    }\n    identities {\n      platform\n      login\n      displayName\n    }\n    count\n  }\n": types.PersonCountFieldsFragmentDoc,
    "\n  query ActivityTopActors($accountId: String, $emoteId: String, $filter: ActivityFilter, $limit: Int) {\n    activityTopActors(accountId: $accountId, emoteId: $emoteId, filter: $filter, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": types.ActivityTopActorsDocument,
    "\n  query AuditTopPeople(\n    $by: AuditPersonRole\n    $targetKind: String\n    $targetId: String\n    $actorId: String\n    $action: String\n    $since: String\n    $until: String\n    $limit: Int\n  ) {\n    auditTopPeople(\n      by: $by\n      targetKind: $targetKind\n      targetId: $targetId\n      actorId: $actorId\n      action: $action\n      since: $since\n      until: $until\n      limit: $limit\n    ) {\n      ...PersonCountFields\n    }\n  }\n": types.AuditTopPeopleDocument,
    "\n  query OrderTopBuyers($status: OrderStatus, $itemId: String, $limit: Int) {\n    orderTopBuyers(status: $status, itemId: $itemId, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": types.OrderTopBuyersDocument,
    "\n  query ReviewTopUploaders($filter: ReviewQueueFilter, $limit: Int) {\n    reviewTopUploaders(filter: $filter, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": types.ReviewTopUploadersDocument,
    "\n  query EntitlementTopHolders($limit: Int) {\n    entitlementTopHolders(limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n": types.EntitlementTopHoldersDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment StudioPaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n    updatedAt\n  }\n"): typeof import('./graphql').StudioPaintFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query StudioPaints {\n    cosmetics {\n      paints {\n        ...StudioPaintFields\n      }\n    }\n  }\n"): typeof import('./graphql').StudioPaintsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation StudioCreatePaint($input: PaintInput!) {\n    createPaint(input: $input) {\n      ...StudioPaintFields\n    }\n  }\n"): typeof import('./graphql').StudioCreatePaintDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {\n    updatePaint(id: $id, input: $input) {\n      ...StudioPaintFields\n    }\n  }\n"): typeof import('./graphql').StudioUpdatePaintDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query StudioCosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n  }\n"): typeof import('./graphql').StudioCosmeticImageDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetDisplayPlatform($platform: Platform!) {\n    setDisplayPlatform(platform: $platform) {\n      id\n      displayPlatform\n    }\n  }\n"): typeof import('./graphql').SetDisplayPlatformDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SignOutEverywhere {\n    signOutEverywhere\n  }\n"): typeof import('./graphql').SignOutEverywhereDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n"): typeof import('./graphql').UnlinkDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n"): typeof import('./graphql').MyNotificationsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation MarkRead($ids: [String!]) {\n    markNotificationsRead(ids: $ids)\n  }\n"): typeof import('./graphql').MarkReadDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query UnreadCount {\n    unreadNotificationCount\n  }\n"): typeof import('./graphql').UnreadCountDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MySessions {\n    mySessions {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n      current\n    }\n  }\n"): typeof import('./graphql').MySessionsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyConsoleToken {\n    myConsoleToken {\n      id\n      createdAt\n      expiresAt\n    }\n  }\n"): typeof import('./graphql').MyConsoleTokenDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RevokeSession($id: String!) {\n    revokeSession(id: $id)\n  }\n"): typeof import('./graphql').RevokeSessionDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RevokeOtherSessions {\n    revokeOtherSessions\n  }\n"): typeof import('./graphql').RevokeOtherSessionsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ExportMyData {\n    exportMyData\n  }\n"): typeof import('./graphql').ExportMyDataDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteAccount($confirm: String!) {\n    deleteAccount(confirm: $confirm)\n  }\n"): typeof import('./graphql').DeleteAccountDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PendingDeletion {\n    pendingDeletion {\n      requestedAt\n      eraseAfter\n      displayName\n      avatar\n    }\n  }\n"): typeof import('./graphql').PendingDeletionDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RestoreAccount {\n    restoreAccount\n  }\n"): typeof import('./graphql').RestoreAccountDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyRoles {\n    me {\n      id\n      roles {\n        id\n        name\n        public\n        invisible\n      }\n    }\n  }\n"): typeof import('./graphql').MyRolesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      displayPlatform\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').MeDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {\n    accountActivityPage(accountId: $accountId, before: $before, limit: $limit, filter: $filter) {\n      entries {\n        id\n        kind\n        actor {\n          id\n          displayName\n          avatar\n        }\n        editor {\n          id\n          displayName\n          avatar\n        }\n        set {\n          id\n          name\n        }\n        emote {\n          id\n          name\n          host\n          files {\n            name\n            width\n            height\n            isStatic\n          }\n        }\n        fromName\n        toName\n        badge {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        paint {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        createdAt\n      }\n      next\n    }\n  }\n"): typeof import('./graphql').AccountActivityDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteActivity($emoteId: String!, $before: String, $limit: Int, $actorId: String) {\n    emoteActivityPage(emoteId: $emoteId, before: $before, limit: $limit, actorId: $actorId) {\n      entries {\n        id\n        kind\n        actor {\n          id\n          displayName\n          avatar\n        }\n        set {\n          id\n          name\n        }\n        emote {\n          id\n          name\n          host\n          files {\n            name\n            width\n            height\n            isStatic\n          }\n        }\n        fromName\n        toName\n        badge {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        paint {\n          fromId\n          fromName\n          toId\n          toName\n        }\n        createdAt\n      }\n      next\n    }\n  }\n"): typeof import('./graphql').EmoteActivityDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query User($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').UserDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {\n    accountStatus(accountId: $accountId) @include(if: $status) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n    accountRoles(accountId: $accountId) @include(if: $roles) {\n      ...RoleFields\n    }\n  }\n"): typeof import('./graphql').UserDetailsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountPermissions($accountId: String!) {\n    accountPermissions(accountId: $accountId) {\n      permission\n      granted\n      effect\n      roleId\n      roleName\n      owner\n    }\n  }\n"): typeof import('./graphql').AccountPermissionsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AllRoles {\n    roles {\n      ...RoleFields\n    }\n  }\n"): typeof import('./graphql').AllRolesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation PlaceRestriction($accountId: String!, $input: RestrictionInput!) {\n    placeRestriction(accountId: $accountId, input: $input) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n  }\n"): typeof import('./graphql').PlaceRestrictionDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation LiftRestriction($accountId: String!, $kind: RestrictionKind!, $reason: String) {\n    liftRestriction(accountId: $accountId, kind: $kind, reason: $reason) {\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n    }\n  }\n"): typeof import('./graphql').LiftRestrictionDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountRestrictions($accountId: String!) {\n    accountRestrictions(accountId: $accountId) {\n      id\n      kind\n      reason\n      endsAt\n      createdBy\n      createdAt\n      liftedAt\n      liftedBy\n      active\n    }\n  }\n"): typeof import('./graphql').AccountRestrictionsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AssignRole($accountId: String!, $roleId: String!) {\n    assignRole(accountId: $accountId, roleId: $roleId)\n  }\n"): typeof import('./graphql').AssignRoleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UnassignRole($accountId: String!, $roleId: String!) {\n    unassignRole(accountId: $accountId, roleId: $roleId)\n  }\n"): typeof import('./graphql').UnassignRoleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Audit(\n    $targetKind: String\n    $targetId: String\n    $actorId: String\n    $action: String\n    $since: String\n    $until: String\n    $before: String\n    $limit: Int\n  ) {\n    auditLog(\n      targetKind: $targetKind\n      targetId: $targetId\n      actorId: $actorId\n      action: $action\n      since: $since\n      until: $until\n      before: $before\n      limit: $limit\n    ) {\n      id\n      actorId\n      action\n      targetKind\n      targetId\n      before\n      after\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AuditDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').ChannelDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment CosmeticImageFields on CosmeticImage {\n    id\n    kind\n    host\n    animated\n    ready\n    width\n    height\n    files {\n      name\n      width\n      height\n      isStatic\n    }\n  }\n"): typeof import('./graphql').CosmeticImageFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment BadgeFields on Badge {\n    id\n    name\n    tooltip\n    image {\n      ...CosmeticImageFields\n    }\n  }\n"): typeof import('./graphql').BadgeFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment PaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      ...CosmeticImageFields\n    }\n  }\n"): typeof import('./graphql').PaintFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Cosmetics {\n    cosmetics {\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n"): typeof import('./graphql').CosmeticsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {\n    userCosmetics(platform: $platform, userIds: $userIds) {\n      platformUserId\n      badgeId\n      paintId\n    }\n  }\n"): typeof import('./graphql').UserCosmeticsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountCosmetics($accountIds: [String!]!) {\n    accountCosmetics(accountIds: $accountIds) {\n      accountId\n      badge {\n        ...BadgeFields\n      }\n      paint {\n        ...PaintFields\n      }\n    }\n  }\n"): typeof import('./graphql').AccountCosmeticsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountProfile($accountId: String!) {\n    accountProfile(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      identities {\n        platform\n        login\n        displayName\n      }\n      roles {\n        id\n        name\n      }\n      activeSets {\n        platform\n        login\n        setId\n        setName\n      }\n      editors {\n        id\n        displayName\n        avatar\n      }\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AccountProfileDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyCosmetics($accountId: String) {\n    myCosmetics(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n      badgeId\n      paintId\n    }\n  }\n"): typeof import('./graphql').MyCosmeticsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {\n    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {\n      badgeId\n      paintId\n    }\n  }\n"): typeof import('./graphql').SelectCosmeticsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query CosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n"): typeof import('./graphql').CosmeticImageDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {\n    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {\n      total\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        kind\n        cosmeticId\n        source\n        reason\n        grantedBy {\n          id\n          displayName\n        }\n        grantedAt\n      }\n    }\n  }\n"): typeof import('./graphql').EntitlementsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateBadge($input: CreateBadgeInput!) {\n    createBadge(input: $input) {\n      ...BadgeFields\n    }\n  }\n"): typeof import('./graphql').CreateBadgeDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {\n    updateBadge(id: $id, input: $input) {\n      ...BadgeFields\n    }\n  }\n"): typeof import('./graphql').UpdateBadgeDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RemakeCosmeticImage($id: String!) {\n    remakeCosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n"): typeof import('./graphql').RemakeCosmeticImageDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteBadge($id: String!) {\n    deleteBadge(id: $id)\n  }\n"): typeof import('./graphql').DeleteBadgeDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeletePaint($id: String!) {\n    deletePaint(id: $id)\n  }\n"): typeof import('./graphql').DeletePaintDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation GrantCosmetic($input: GrantCosmeticInput!) {\n    grantCosmetic(input: $input) {\n      cosmeticId\n    }\n  }\n"): typeof import('./graphql').GrantCosmeticDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {\n    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)\n  }\n"): typeof import('./graphql').RevokeCosmeticDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment AccountEditorFields on AccountEditor {\n    account {\n      id\n      displayName\n      avatar\n    }\n    permissions\n    allSets\n    setIds\n    grantedBy {\n      id\n      displayName\n    }\n    grantedAt\n  }\n"): typeof import('./graphql').AccountEditorFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Editors {\n    accountEditors {\n      ...AccountEditorFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      allSets\n      setIds\n      grantedAt\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n  }\n"): typeof import('./graphql').EditorsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AddAccountEditor($input: AddAccountEditorInput!) {\n    addAccountEditor(input: $input) {\n      ...AccountEditorFields\n    }\n  }\n"): typeof import('./graphql').AddAccountEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) {\n    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions, setIds: $setIds) {\n      ...AccountEditorFields\n    }\n  }\n"): typeof import('./graphql').UpdateAccountEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {\n    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)\n  }\n"): typeof import('./graphql').RemoveAccountEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ReportEmote($id: String!, $reason: String!) {\n    reportEmote(id: $id, reason: $reason) {\n      id\n    }\n  }\n"): typeof import('./graphql').ReportEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MySetNames {\n    mySets {\n      id\n      name\n      kind\n      emotes {\n        emote {\n          id\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').MySetNamesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort, $after: String) {\n    searchEmotes(query: \"\", owner: $owner, page: $page, perPage: $perPage, sort: $sort, after: $after) {\n      total\n      page\n      perPage\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SearchByOwnerDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AdminAccount($accountId: String!) {\n    adminAccount(accountId: $accountId) {\n      id\n      displayName\n      avatar\n      createdAt\n      uploadsBlocked\n      ticketsBlocked\n      blocked\n      uploadsBlockedUntil\n      ticketsBlockedUntil\n      blockedUntil\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').AdminAccountDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation TransferEmote($id: String!, $toAccountId: String!) {\n    transferEmote(id: $id, toAccountId: $toAccountId) {\n      ...EmoteFields\n    }\n  }\n"): typeof import('./graphql').TransferEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountEmotes($accountId: String!, $page: Int, $perPage: Int, $after: String) {\n    accountEmotes(accountId: $accountId, page: $page, perPage: $perPage, after: $after) {\n      total\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').AccountEmotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountReports($accountId: String!) {\n    accountReports(accountId: $accountId) {\n      about {\n        ...ReportFields\n      }\n      by {\n        ...ReportFields\n      }\n    }\n  }\n"): typeof import('./graphql').AccountReportsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PublicAccount($id: String!) {\n    account(id: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n"): typeof import('./graphql').PublicAccountDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteChannels($id: String!, $limit: Int, $offset: Int) {\n    emoteChannels(id: $id, limit: $limit, offset: $offset) {\n      platform\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').EmoteChannelsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    processing\n    animated\n    tags\n    host\n    createdAt\n    channelCount\n    visibility\n    flags {\n      zeroWidth\n      private\n      wide\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n"): typeof import('./graphql').EmoteFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Search(\n    $query: String!\n    $animated: Boolean\n    $page: Int\n    $perPage: Int\n    $sort: EmoteSort\n    $flags: [EmoteFlag!]\n    $tags: [String!]\n    $exact: Boolean\n    $window: TrendingWindow\n    $after: String\n  ) {\n    searchEmotes(\n      query: $query\n      animated: $animated\n      page: $page\n      perPage: $perPage\n      sort: $sort\n      flags: $flags\n      tags: $tags\n      exact: $exact\n      window: $window\n      after: $after\n    ) {\n      total\n      page\n      perPage\n      next\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SearchDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query FindPeople($query: String!, $page: Int, $perPage: Int, $after: String) {\n    searchAccounts(query: $query, page: $page, perPage: $perPage, after: $after) {\n      total\n      page\n      perPage\n      next\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').FindPeopleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n"): typeof import('./graphql').EmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteTrending($id: String!, $window: TrendingWindow) {\n    emoteTrending(id: $id, window: $window) {\n      rank\n      channels\n      window\n    }\n  }\n"): typeof import('./graphql').EmoteTrendingDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    editorSets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').MyChannelsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n"): typeof import('./graphql').UpdateChannelDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').MyEmotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n"): typeof import('./graphql').UpdateEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n"): typeof import('./graphql').DeleteEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PersonalLists {\n    hiddenEmotes {\n      emoteId\n    }\n    hiddenCreators {\n      accountId\n    }\n    favouriteEmoteIds\n  }\n"): typeof import('./graphql').PersonalListsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query HiddenLists {\n    hiddenEmotes {\n      emoteId\n      hiddenAt\n      emote {\n        ...EmoteFields\n      }\n    }\n    hiddenCreators {\n      accountId\n      hiddenAt\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').HiddenListsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyFavourites($query: String, $page: Int, $perPage: Int) {\n    myFavourites(query: $query, page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').MyFavouritesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation HideEmote($id: String!) {\n    hideEmote(id: $id)\n  }\n"): typeof import('./graphql').HideEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UnhideEmote($id: String!) {\n    unhideEmote(id: $id)\n  }\n"): typeof import('./graphql').UnhideEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation HideCreator($accountId: String!) {\n    hideCreator(accountId: $accountId)\n  }\n"): typeof import('./graphql').HideCreatorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UnhideCreator($accountId: String!) {\n    unhideCreator(accountId: $accountId)\n  }\n"): typeof import('./graphql').UnhideCreatorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation FavouriteEmote($id: String!) {\n    favouriteEmote(id: $id)\n  }\n"): typeof import('./graphql').FavouriteEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UnfavouriteEmote($id: String!) {\n    unfavouriteEmote(id: $id)\n  }\n"): typeof import('./graphql').UnfavouriteEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountRoles($accountId: String!) {\n    accountRoles(accountId: $accountId) {\n      id\n      name\n      position\n    }\n  }\n"): typeof import('./graphql').AccountRolesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountEditors($accountId: String) {\n    accountEditors(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').AccountEditorsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EditorOf {\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      allSets\n    }\n  }\n"): typeof import('./graphql').EditorOfDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountSets($accountId: String!) {\n    accountSets(accountId: $accountId, preview: 12) {\n      id\n      name\n      kind\n      capacity\n      slotsUsed\n      active\n      preview {\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').AccountSetsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ProfileMine {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n    }\n    mySets {\n      ...SetFields\n    }\n  }\n"): typeof import('./graphql').ProfileMineDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ProfileEdited {\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorSets {\n      ...SetFields\n    }\n  }\n"): typeof import('./graphql').ProfileEditedDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment ReportFields on Report {\n    id\n    reason\n    reporterId\n    state\n    resolution\n    createdAt\n    resolvedAt\n    emote {\n      ...EmoteFields\n    }\n  }\n"): typeof import('./graphql').ReportFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ReportQueue($page: Int, $perPage: Int) {\n    reportQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        id\n        reason\n        reporterId\n        state\n        createdAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').ReportQueueDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {\n    resolveReport(id: $id, action: $action, note: $note) {\n      id\n      state\n    }\n  }\n"): typeof import('./graphql').ResolveReportDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {\n    reviewEmotes(ids: $ids, verdict: $verdict, visibility: $visibility, reason: $reason) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n"): typeof import('./graphql').ReviewEmotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RequeueEmotes($ids: [String!]!) {\n    requeueEmotes(ids: $ids) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n"): typeof import('./graphql').RequeueEmotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetEmoteFlags($id: String!, $input: EmoteFlagsInput!) {\n    setEmoteFlags(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n"): typeof import('./graphql').SetEmoteFlagsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DismissReports($ids: [String!]!, $note: String) {\n    dismissReports(ids: $ids, note: $note) {\n      done\n      failed {\n        id\n        code\n        message\n      }\n    }\n  }\n"): typeof import('./graphql').DismissReportsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteDuplicates($emoteId: String!) {\n    emoteDuplicates(emoteId: $emoteId) {\n      sameFile\n      similarity\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').EmoteDuplicatesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ReviewReasons {\n    reviewReasons\n  }\n"): typeof import('./graphql').ReviewReasonsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetReviewReasons($reasons: [String!]!) {\n    setReviewReasons(reasons: $reasons)\n  }\n"): typeof import('./graphql').SetReviewReasonsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountsByLogin($logins: [String!]!) {\n    accountsByLogin(logins: $logins) {\n      login\n      problem\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').AccountsByLoginDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmotesWithSameFile($sha256: String!) {\n    emotesWithSameFile(sha256: $sha256) {\n      id\n      name\n      owner {\n        id\n        displayName\n      }\n    }\n  }\n"): typeof import('./graphql').EmotesWithSameFileDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AdminCounts($review: Boolean!, $reports: Boolean!) {\n    reviewQueue(page: 1, perPage: 1) @include(if: $review) {\n      total\n    }\n    reportQueue(page: 1, perPage: 1) @include(if: $reports) {\n      total\n    }\n    globalSet {\n      id\n    }\n  }\n"): typeof import('./graphql').AdminCountsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {\n    reviewQueue(page: $page, perPage: $perPage, filter: $filter) {\n      total\n      counts {\n        public\n        limited\n      }\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').ReviewQueueDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Approve($id: String!, $visibility: EmoteVisibility) {\n    approveEmote(id: $id, visibility: $visibility) {\n      id\n      state\n      visibility\n    }\n  }\n"): typeof import('./graphql').ApproveDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteHistory($emoteId: String!) {\n    emoteHistory(emoteId: $emoteId) {\n      id\n      actorId\n      action\n      before\n      after\n      createdAt\n    }\n  }\n"): typeof import('./graphql').EmoteHistoryDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteNotes($emoteId: String!) {\n    emoteNotes(emoteId: $emoteId) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n"): typeof import('./graphql').EmoteNotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AddEmoteNote($emoteId: String!, $body: String!) {\n    addEmoteNote(emoteId: $emoteId, body: $body) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AddEmoteNoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteEmoteNote($id: String!) {\n    deleteEmoteNote(id: $id)\n  }\n"): typeof import('./graphql').DeleteEmoteNoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Reject($id: String!, $reason: String!) {\n    rejectEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n"): typeof import('./graphql').RejectDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment RoleFields on Role {\n    id\n    name\n    description\n    position\n    permissions\n    denies\n    public\n    invisible\n    builtIn\n    editable\n    assignable\n  }\n"): typeof import('./graphql').RoleFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Roles {\n    roles {\n      ...RoleFields\n    }\n    permissions\n  }\n"): typeof import('./graphql').RolesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment RoleLimitFields on RoleLimit {\n    key\n    group\n    default\n    max\n    windowSeconds\n    value\n  }\n"): typeof import('./graphql').RoleLimitFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query LimitDefaults {\n    limitDefaults {\n      ...RoleLimitFields\n    }\n  }\n"): typeof import('./graphql').LimitDefaultsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query RoleLimits($id: String!) {\n    roleLimits(id: $id) {\n      ...RoleLimitFields\n    }\n  }\n"): typeof import('./graphql').RoleLimitsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetRoleLimits($id: String!, $limits: [RoleLimitInput!]!) {\n    setRoleLimits(id: $id, limits: $limits) {\n      ...RoleLimitFields\n    }\n  }\n"): typeof import('./graphql').SetRoleLimitsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query UploadQuota($ownerId: String) {\n    uploadQuota(ownerId: $ownerId) {\n      accountId\n      pending\n      pendingMax\n      uploadsThisHour\n      uploadsPerHour\n      maxBytes\n      maxSide\n      maxFrames\n      maxFps\n      maxAspect\n    }\n  }\n"): typeof import('./graphql').UploadQuotaDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment MediaLimitFields on MediaLimits {\n    maxBytes\n    maxSide\n    maxFrames\n    maxFps\n    maxTotalPixels\n    maxDurationMs\n    minSide\n    maxAspect\n    maxServedSide\n  }\n"): typeof import('./graphql').MediaLimitFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PublicLimits {\n    limits {\n      emote { ...MediaLimitFields }\n      badge { ...MediaLimitFields }\n      paint { ...MediaLimitFields }\n      names {\n        emote { min max }\n        set { min max }\n        badge { min max }\n        paint { min max }\n        tooltip { min max }\n      }\n      tags { max maxLen }\n      rates { key count windowSeconds }\n      account { key default effective windowSeconds }\n    }\n  }\n"): typeof import('./graphql').PublicLimitsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateRole($input: CreateRoleInput!) {\n    createRole(input: $input) {\n      ...RoleFields\n    }\n  }\n"): typeof import('./graphql').CreateRoleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {\n    updateRole(id: $id, input: $input) {\n      ...RoleFields\n    }\n  }\n"): typeof import('./graphql').UpdateRoleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteRole($id: String!) {\n    deleteRole(id: $id)\n  }\n"): typeof import('./graphql').DeleteRoleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query QuickSearch($query: String!) {\n    quickSearch(query: $query) {\n      emotes {\n        ...EmoteFields\n      }\n      users {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').QuickSearchDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EditableChannels {\n    myEditableChannels {\n      role\n      channel {\n        platform\n        login\n        account {\n          id\n          displayName\n          avatar\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').EditableChannelsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    visibility\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SetFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n    editorSets {\n      ...SetFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').MySetsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      owner {\n        id\n        displayName\n        avatar\n      }\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n      allSets\n      setIds\n    }\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n  }\n"): typeof import('./graphql').EditSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query SetEmotes($id: String!, $query: String, $page: Int, $perPage: Int) {\n    setEmotes(id: $id, query: $query, page: $page, perPage: $perPage) {\n      total\n      items {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').SetEmotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query SetMembers($id: String!, $page: Int, $perPage: Int) {\n    setEmotes(id: $id, page: $page, perPage: $perPage) {\n      total\n      items {\n        name\n        emote {\n          id\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').SetMembersDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n"): typeof import('./graphql').CreateSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n      visibility\n      capacity\n    }\n  }\n"): typeof import('./graphql').UpdateSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n"): typeof import('./graphql').DeleteSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query SetCapacityRange($id: String!) {\n    setCapacityRange(id: $id) {\n      min\n      max\n      ownerMax\n      used\n      staff\n    }\n  }\n"): typeof import('./graphql').SetCapacityRangeDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query SetMenu($id: String!) {\n    setCapacityRange(id: $id) {\n      min\n      max\n      ownerMax\n      used\n      staff\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n      allSets\n      setIds\n    }\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n  }\n"): typeof import('./graphql').SetMenuDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n"): typeof import('./graphql').AddSetEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n"): typeof import('./graphql').UpdateSetEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n"): typeof import('./graphql').RemoveSetEmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n"): typeof import('./graphql').RemoveSetEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PublicSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      createdAt\n      owner {\n        id\n        displayName\n        avatar\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').PublicSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query SetEditors($id: String!) {\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n"): typeof import('./graphql').SetEditorsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {\n    cloneSet(id: $id, name: $name, visibility: $visibility) {\n      copied\n      skipped\n      set {\n        id\n        name\n      }\n    }\n  }\n"): typeof import('./graphql').CloneSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment ShopItemFields on ShopItem {\n    id\n    name\n    description\n    priceCents\n    currency\n    paddlePriceId\n    badges {\n      ...BadgeFields\n    }\n    paints {\n      ...PaintFields\n    }\n    activeFrom\n    activeTo\n    stockLimit\n    stockLeft\n    visible\n    sortOrder\n    available\n    owned\n    deleted\n  }\n"): typeof import('./graphql').ShopItemFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ShopItems($all: Boolean) {\n    shopItems(all: $all) {\n      ...ShopItemFields\n    }\n  }\n"): typeof import('./graphql').ShopItemsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ShopItem($id: String!) {\n    shopItem(id: $id) {\n      ...ShopItemFields\n    }\n  }\n"): typeof import('./graphql').ShopItemDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment OrderFields on Order {\n    id\n    buyer {\n      id\n      displayName\n      avatar\n    }\n    recipient {\n      id\n      displayName\n      avatar\n    }\n    gift\n    itemId\n    itemName\n    status\n    amountCents\n    currency\n    paidTotalCents\n    paidCurrency\n    refundReason\n    createdAt\n    paidAt\n    refundedAt\n    failedAt\n  }\n"): typeof import('./graphql').OrderFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {\n    startCheckout(itemId: $itemId, recipient: $recipient, idempotencyKey: $idempotencyKey) {\n      order {\n        ...OrderFields\n      }\n      transactionId\n      checkoutUrl\n      clientToken\n      environment\n    }\n  }\n"): typeof import('./graphql').StartCheckoutDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Order($id: String!) {\n    order(id: $id) {\n      ...OrderFields\n    }\n  }\n"): typeof import('./graphql').OrderDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MyOrders($page: Int, $perPage: Int) {\n    myOrders(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...OrderFields\n      }\n    }\n  }\n"): typeof import('./graphql').MyOrdersDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RedeemCode($code: String!, $idempotencyKey: String) {\n    redeemCode(code: $code, idempotencyKey: $idempotencyKey) {\n      name\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n"): typeof import('./graphql').RedeemCodeDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateShopItem($input: ShopItemInput!) {\n    createShopItem(input: $input) {\n      ...ShopItemFields\n    }\n  }\n"): typeof import('./graphql').CreateShopItemDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {\n    updateShopItem(id: $id, input: $input) {\n      ...ShopItemFields\n    }\n  }\n"): typeof import('./graphql').UpdateShopItemDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteShopItem($id: String!) {\n    deleteShopItem(id: $id)\n  }\n"): typeof import('./graphql').DeleteShopItemDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {\n    orders(status: $status, accountId: $accountId, itemId: $itemId, page: $page, perPage: $perPage) {\n      total\n      items {\n        ...OrderFields\n        providerTransactionId\n        refundRejectedAt\n      }\n    }\n  }\n"): typeof import('./graphql').AdminOrdersDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RefundOrder($id: String!, $reason: String!) {\n    refundOrder(id: $id, reason: $reason) {\n      ...OrderFields\n      providerTransactionId\n      refundRejectedAt\n    }\n  }\n"): typeof import('./graphql').RefundOrderDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment CodeBatchFields on CodeBatch {\n    id\n    name\n    prefix\n    itemId\n    itemName\n    badgeIds\n    paintIds\n    codeCount\n    usesPerCode\n    redemptions\n    expiresAt\n    disabledAt\n    createdBy {\n      id\n      displayName\n    }\n    createdAt\n  }\n"): typeof import('./graphql').CodeBatchFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query CodeBatches($page: Int, $perPage: Int) {\n    codeBatches(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...CodeBatchFields\n      }\n    }\n  }\n"): typeof import('./graphql').CodeBatchesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateCodeBatch($input: CodeBatchInput!) {\n    createCodeBatch(input: $input) {\n      batch {\n        ...CodeBatchFields\n      }\n      codes\n    }\n  }\n"): typeof import('./graphql').CreateCodeBatchDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DisableCodeBatch($id: String!) {\n    disableCodeBatch(id: $id) {\n      ...CodeBatchFields\n    }\n  }\n"): typeof import('./graphql').DisableCodeBatchDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ShopCounts($items: Boolean!, $orders: Boolean!, $codes: Boolean!) {\n    shopItems(all: true) @include(if: $items) {\n      id\n    }\n    orders(page: 1, perPage: 1) @include(if: $orders) {\n      total\n    }\n    codeBatches(page: 1, perPage: 1) @include(if: $codes) {\n      total\n    }\n  }\n"): typeof import('./graphql').ShopCountsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment AnnouncementFields on Announcement {\n    id\n    kind\n    text\n    linkUrl\n    linkLabel\n    badgeId\n    startsAt\n    endsAt\n    endedAt\n    createdAt\n    updatedAt\n    live\n    emote {\n      id\n      name\n      host\n      animated\n      files {\n        name\n        width\n        height\n        isStatic\n      }\n    }\n  }\n"): typeof import('./graphql').AnnouncementFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ActiveAnnouncement {\n    activeAnnouncement {\n      ...AnnouncementFields\n    }\n  }\n"): typeof import('./graphql').ActiveAnnouncementDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Announcements {\n    announcements {\n      ...AnnouncementFields\n    }\n  }\n"): typeof import('./graphql').AnnouncementsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateAnnouncement($input: AnnouncementInput!) {\n    createAnnouncement(input: $input) {\n      ...AnnouncementFields\n    }\n  }\n"): typeof import('./graphql').CreateAnnouncementDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {\n    updateAnnouncement(id: $id, input: $input) {\n      ...AnnouncementFields\n    }\n  }\n"): typeof import('./graphql').UpdateAnnouncementDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation EndAnnouncement($id: String!) {\n    endAnnouncement(id: $id) {\n      ...AnnouncementFields\n    }\n  }\n"): typeof import('./graphql').EndAnnouncementDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query NotificationPrefs {\n    myNotificationPrefs {\n      kind\n      notify\n      toast\n      required\n    }\n  }\n"): typeof import('./graphql').NotificationPrefsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {\n    setNotificationPrefs(prefs: $prefs) {\n      kind\n      notify\n      toast\n      required\n    }\n  }\n"): typeof import('./graphql').SetNotificationPrefsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query StaffNotes($accountId: String!) {\n    staffNotes(accountId: $accountId) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n"): typeof import('./graphql').StaffNotesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AddStaffNote($accountId: String!, $body: String!) {\n    addStaffNote(accountId: $accountId, body: $body) {\n      id\n      authorId\n      body\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AddStaffNoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteStaffNote($id: String!) {\n    deleteStaffNote(id: $id)\n  }\n"): typeof import('./graphql').DeleteStaffNoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountSessions($accountId: String!) {\n    accountSessions(accountId: $accountId) {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n    }\n  }\n"): typeof import('./graphql').AccountSessionsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountSubscriptions($accountId: String!) {\n    accountSubscriptions(accountId: $accountId) {\n      id\n      plan\n      state\n      startsAt\n      endsAt\n      source\n    }\n  }\n"): typeof import('./graphql').AccountSubscriptionsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment GrantFields on EntitlementGrant {\n    kind\n    itemId\n    itemName\n    accountId\n    source\n    reason\n    grantedBy\n    grantedAt\n    orderId\n    orderName\n    orderStatus\n    codeBatchId\n    codeBatchName\n    revokedAt\n    revokedBy\n    revokeReason\n    active\n  }\n"): typeof import('./graphql').GrantFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EntitlementGraph($accountId: String!) {\n    entitlementGraph(accountId: $accountId) {\n      ...GrantFields\n    }\n  }\n"): typeof import('./graphql').EntitlementGraphDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EntitlementHolders($kind: GrantKind!, $id: String!) {\n    entitlementHolders(kind: $kind, id: $id) {\n      ...GrantFields\n    }\n  }\n"): typeof import('./graphql').EntitlementHoldersDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment PersonCountFields on PersonCount {\n    account {\n      id\n      displayName\n      avatar\n    }\n    identities {\n      platform\n      login\n      displayName\n    }\n    count\n  }\n"): typeof import('./graphql').PersonCountFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ActivityTopActors($accountId: String, $emoteId: String, $filter: ActivityFilter, $limit: Int) {\n    activityTopActors(accountId: $accountId, emoteId: $emoteId, filter: $filter, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n"): typeof import('./graphql').ActivityTopActorsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AuditTopPeople(\n    $by: AuditPersonRole\n    $targetKind: String\n    $targetId: String\n    $actorId: String\n    $action: String\n    $since: String\n    $until: String\n    $limit: Int\n  ) {\n    auditTopPeople(\n      by: $by\n      targetKind: $targetKind\n      targetId: $targetId\n      actorId: $actorId\n      action: $action\n      since: $since\n      until: $until\n      limit: $limit\n    ) {\n      ...PersonCountFields\n    }\n  }\n"): typeof import('./graphql').AuditTopPeopleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query OrderTopBuyers($status: OrderStatus, $itemId: String, $limit: Int) {\n    orderTopBuyers(status: $status, itemId: $itemId, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n"): typeof import('./graphql').OrderTopBuyersDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ReviewTopUploaders($filter: ReviewQueueFilter, $limit: Int) {\n    reviewTopUploaders(filter: $filter, limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n"): typeof import('./graphql').ReviewTopUploadersDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EntitlementTopHolders($limit: Int) {\n    entitlementTopHolders(limit: $limit) {\n      ...PersonCountFields\n    }\n  }\n"): typeof import('./graphql').EntitlementTopHoldersDocument;


export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}
