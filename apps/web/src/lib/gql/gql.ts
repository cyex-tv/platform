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
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    channelCount\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": typeof types.EmoteFieldsFragmentDoc,
    "\n  query Search($query: String!, $animated: Boolean, $nsfw: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort) {\n    searchEmotes(query: $query, animated: $animated, nsfw: $nsfw, page: $page, perPage: $perPage, sort: $sort) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SearchDocument,
    "\n  query FindPeople($query: String!, $page: Int, $perPage: Int) {\n    searchAccounts(query: $query, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n": typeof types.FindPeopleDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": typeof types.EmoteDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.ChannelDocument,
    "\n  query Set($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      capacity\n      owner {\n        id\n        displayName\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.SetDocument,
    "\n  query PublicSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      createdAt\n      owner {\n        id\n        displayName\n        avatar\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.PublicSetDocument,
    "\n  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {\n    cloneSet(id: $id, name: $name, visibility: $visibility) {\n      copied\n      skipped\n      set {\n        id\n        name\n      }\n    }\n  }\n": typeof types.CloneSetDocument,
    "\n  query GlobalSet {\n    globalSet {\n      id\n      name\n      emotes {\n        name\n        zeroWidth\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.GlobalSetDocument,
    "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.MeDocument,
    "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.MyEmotesDocument,
    "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": typeof types.UpdateEmoteDocument,
    "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n": typeof types.DeleteEmoteDocument,
    "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SetFieldsFragmentDoc,
    "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n    editorSets {\n      ...SetFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n      }\n      permissions\n    }\n  }\n": typeof types.MySetsDocument,
    "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      ...SetFields\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n": typeof types.EditSetDocument,
    "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n": typeof types.CreateSetDocument,
    "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n    }\n  }\n": typeof types.UpdateSetDocument,
    "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n": typeof types.DeleteSetDocument,
    "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n": typeof types.AddSetEmoteDocument,
    "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n": typeof types.UpdateSetEmoteDocument,
    "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n": typeof types.RemoveSetEmoteDocument,
    "\n  mutation AddSetEditor($setId: String!, $accountId: String!) {\n    addSetEditor(setId: $setId, accountId: $accountId)\n  }\n": typeof types.AddSetEditorDocument,
    "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n": typeof types.RemoveSetEditorDocument,
    "\n  query FindAccount($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.FindAccountDocument,
    "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    editorSets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n": typeof types.MyChannelsDocument,
    "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n": typeof types.UpdateChannelDocument,
    "\n  fragment AccountEditorFields on AccountEditor {\n    account {\n      id\n      displayName\n      avatar\n    }\n    permissions\n    grantedBy {\n      id\n      displayName\n    }\n    grantedAt\n  }\n": typeof types.AccountEditorFieldsFragmentDoc,
    "\n  query Editors {\n    accountEditors {\n      ...AccountEditorFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      grantedAt\n    }\n  }\n": typeof types.EditorsDocument,
    "\n  mutation AddAccountEditor($input: AddAccountEditorInput!) {\n    addAccountEditor(input: $input) {\n      ...AccountEditorFields\n    }\n  }\n": typeof types.AddAccountEditorDocument,
    "\n  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!) {\n    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions) {\n      ...AccountEditorFields\n    }\n  }\n": typeof types.UpdateAccountEditorDocument,
    "\n  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {\n    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)\n  }\n": typeof types.RemoveAccountEditorDocument,
    "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n": typeof types.UnlinkDocument,
    "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n": typeof types.MyNotificationsDocument,
    "\n  mutation MarkRead($ids: [String!]) {\n    markNotificationsRead(ids: $ids)\n  }\n": typeof types.MarkReadDocument,
    "\n  query UnreadCount {\n    unreadNotificationCount\n  }\n": typeof types.UnreadCountDocument,
    "\n  query MySessions {\n    mySessions {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n      current\n    }\n  }\n": typeof types.MySessionsDocument,
    "\n  mutation RevokeSession($id: String!) {\n    revokeSession(id: $id)\n  }\n": typeof types.RevokeSessionDocument,
    "\n  mutation RevokeOtherSessions {\n    revokeOtherSessions\n  }\n": typeof types.RevokeOtherSessionsDocument,
    "\n  mutation ExportMyData {\n    exportMyData\n  }\n": typeof types.ExportMyDataDocument,
    "\n  mutation DeleteAccount($confirm: String!) {\n    deleteAccount(confirm: $confirm)\n  }\n": typeof types.DeleteAccountDocument,
    "\n  query AdminCounts($review: Boolean!, $reports: Boolean!) {\n    reviewQueue(page: 1, perPage: 1) @include(if: $review) {\n      total\n    }\n    reportQueue(page: 1, perPage: 1) @include(if: $reports) {\n      total\n    }\n    globalSet {\n      id\n    }\n  }\n": typeof types.AdminCountsDocument,
    "\n  query ReviewQueue($page: Int, $perPage: Int) {\n    reviewQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.ReviewQueueDocument,
    "\n  mutation Approve($id: String!) {\n    approveEmote(id: $id) {\n      id\n      state\n    }\n  }\n": typeof types.ApproveDocument,
    "\n  mutation Reject($id: String!, $reason: String!) {\n    rejectEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n": typeof types.RejectDocument,
    "\n  query ReportQueue($page: Int, $perPage: Int) {\n    reportQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        id\n        reason\n        reporterId\n        state\n        createdAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.ReportQueueDocument,
    "\n  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {\n    resolveReport(id: $id, action: $action, note: $note) {\n      id\n      state\n    }\n  }\n": typeof types.ResolveReportDocument,
    "\n  mutation Takedown($id: String!, $reason: String!) {\n    takedownEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n": typeof types.TakedownDocument,
    "\n  fragment RoleFields on Role {\n    id\n    name\n    description\n    position\n    permissions\n    public\n  }\n": typeof types.RoleFieldsFragmentDoc,
    "\n  query Roles {\n    roles {\n      ...RoleFields\n    }\n    permissions\n  }\n": typeof types.RolesDocument,
    "\n  mutation CreateRole($input: CreateRoleInput!) {\n    createRole(input: $input) {\n      ...RoleFields\n    }\n  }\n": typeof types.CreateRoleDocument,
    "\n  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {\n    updateRole(id: $id, input: $input) {\n      ...RoleFields\n    }\n  }\n": typeof types.UpdateRoleDocument,
    "\n  mutation DeleteRole($id: String!) {\n    deleteRole(id: $id)\n  }\n": typeof types.DeleteRoleDocument,
    "\n  query User($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.UserDocument,
    "\n  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {\n    accountStatus(accountId: $accountId) @include(if: $status) {\n      uploadsBlocked\n      blocked\n    }\n    accountRoles(accountId: $accountId) @include(if: $roles) {\n      ...RoleFields\n    }\n  }\n": typeof types.UserDetailsDocument,
    "\n  query AllRoles {\n    roles {\n      ...RoleFields\n    }\n  }\n": typeof types.AllRolesDocument,
    "\n  mutation Restrict($accountId: String!, $uploadsBlocked: Boolean, $blocked: Boolean) {\n    restrictAccount(accountId: $accountId, uploadsBlocked: $uploadsBlocked, blocked: $blocked) {\n      uploadsBlocked\n      blocked\n    }\n  }\n": typeof types.RestrictDocument,
    "\n  mutation AssignRole($accountId: String!, $roleId: String!) {\n    assignRole(accountId: $accountId, roleId: $roleId)\n  }\n": typeof types.AssignRoleDocument,
    "\n  mutation UnassignRole($accountId: String!, $roleId: String!) {\n    unassignRole(accountId: $accountId, roleId: $roleId)\n  }\n": typeof types.UnassignRoleDocument,
    "\n  query Audit($targetKind: String, $targetId: String, $before: String, $limit: Int) {\n    auditLog(targetKind: $targetKind, targetId: $targetId, before: $before, limit: $limit) {\n      id\n      actorId\n      action\n      targetKind\n      targetId\n      before\n      after\n      createdAt\n    }\n  }\n": typeof types.AuditDocument,
    "\n  mutation ReportEmote($id: String!, $reason: String!) {\n    reportEmote(id: $id, reason: $reason) {\n      id\n    }\n  }\n": typeof types.ReportEmoteDocument,
    "\n  query MySetNames {\n    mySets {\n      id\n      name\n      kind\n      emotes {\n        emote {\n          id\n        }\n      }\n    }\n  }\n": typeof types.MySetNamesDocument,
    "\n  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort) {\n    searchEmotes(query: \"\", owner: $owner, page: $page, perPage: $perPage, sort: $sort) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SearchByOwnerDocument,
    "\n  query PublicAccount($id: String!) {\n    account(id: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": typeof types.PublicAccountDocument,
    "\n  query EmoteChannels($id: String!, $limit: Int) {\n    emoteChannels(id: $id, limit: $limit) {\n      platform\n      login\n      account {\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.EmoteChannelsDocument,
    "\n  fragment CosmeticImageFields on CosmeticImage {\n    id\n    kind\n    host\n    animated\n    ready\n    width\n    height\n    files {\n      name\n      width\n      height\n      isStatic\n    }\n  }\n": typeof types.CosmeticImageFieldsFragmentDoc,
    "\n  fragment BadgeFields on Badge {\n    id\n    name\n    tooltip\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.BadgeFieldsFragmentDoc,
    "\n  fragment PaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.PaintFieldsFragmentDoc,
    "\n  query Cosmetics {\n    cosmetics {\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n": typeof types.CosmeticsDocument,
    "\n  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {\n    userCosmetics(platform: $platform, userIds: $userIds) {\n      platformUserId\n      badgeId\n      paintId\n    }\n  }\n": typeof types.UserCosmeticsDocument,
    "\n  query AccountCosmetics($accountIds: [String!]!) {\n    accountCosmetics(accountIds: $accountIds) {\n      accountId\n      badge {\n        ...BadgeFields\n      }\n      paint {\n        ...PaintFields\n      }\n    }\n  }\n": typeof types.AccountCosmeticsDocument,
    "\n  query AccountProfile($accountId: String!) {\n    accountProfile(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      identities {\n        platform\n        login\n        displayName\n      }\n      roles {\n        id\n        name\n      }\n      activeSets {\n        platform\n        login\n        setId\n        setName\n      }\n      createdAt\n    }\n  }\n": typeof types.AccountProfileDocument,
    "\n  query MyCosmetics($accountId: String) {\n    myCosmetics(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n      badgeId\n      paintId\n    }\n  }\n": typeof types.MyCosmeticsDocument,
    "\n  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {\n    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {\n      badgeId\n      paintId\n    }\n  }\n": typeof types.SelectCosmeticsDocument,
    "\n  query CosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n": typeof types.CosmeticImageDocument,
    "\n  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {\n    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {\n      total\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        kind\n        cosmeticId\n        source\n        reason\n        grantedBy {\n          id\n          displayName\n        }\n        grantedAt\n      }\n    }\n  }\n": typeof types.EntitlementsDocument,
    "\n  mutation CreateBadge($input: CreateBadgeInput!) {\n    createBadge(input: $input) {\n      ...BadgeFields\n    }\n  }\n": typeof types.CreateBadgeDocument,
    "\n  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {\n    updateBadge(id: $id, input: $input) {\n      ...BadgeFields\n    }\n  }\n": typeof types.UpdateBadgeDocument,
    "\n  mutation DeleteBadge($id: String!) {\n    deleteBadge(id: $id)\n  }\n": typeof types.DeleteBadgeDocument,
    "\n  mutation DeletePaint($id: String!) {\n    deletePaint(id: $id)\n  }\n": typeof types.DeletePaintDocument,
    "\n  mutation GrantCosmetic($input: GrantCosmeticInput!) {\n    grantCosmetic(input: $input) {\n      cosmeticId\n    }\n  }\n": typeof types.GrantCosmeticDocument,
    "\n  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {\n    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)\n  }\n": typeof types.RevokeCosmeticDocument,
    "\n  query AccountRoles($accountId: String!) {\n    accountRoles(accountId: $accountId) {\n      id\n      name\n      position\n    }\n  }\n": typeof types.AccountRolesDocument,
    "\n  query AccountEditors($accountId: String) {\n    accountEditors(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n": typeof types.AccountEditorsDocument,
    "\n  query EditorOf {\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n": typeof types.EditorOfDocument,
    "\n  query AccountSets($accountId: String!) {\n    accountSets(accountId: $accountId, preview: 12) {\n      id\n      name\n      kind\n      capacity\n      slotsUsed\n      active\n      preview {\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.AccountSetsDocument,
    "\n  query ProfileMine {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n    }\n    mySets {\n      ...SetFields\n    }\n  }\n": typeof types.ProfileMineDocument,
    "\n  query ProfileEdited {\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorSets {\n      ...SetFields\n    }\n  }\n": typeof types.ProfileEditedDocument,
    "\n  query AccountActivity($accountId: String!, $before: String, $limit: Int) {\n    accountActivity(accountId: $accountId, before: $before, limit: $limit) {\n      id\n      kind\n      actor {\n        id\n        displayName\n        avatar\n      }\n      set {\n        id\n        name\n      }\n      emote {\n        id\n        name\n        host\n        files {\n          name\n          width\n          height\n          isStatic\n        }\n      }\n      fromName\n      toName\n      badge {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      paint {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      createdAt\n    }\n  }\n": typeof types.AccountActivityDocument,
    "\n  query EmoteActivity($emoteId: String!, $before: String, $limit: Int) {\n    emoteActivity(emoteId: $emoteId, before: $before, limit: $limit) {\n      id\n      kind\n      actor {\n        id\n        displayName\n        avatar\n      }\n      set {\n        id\n        name\n      }\n      emote {\n        id\n        name\n        host\n        files {\n          name\n          width\n          height\n          isStatic\n        }\n      }\n      fromName\n      toName\n      badge {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      paint {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      createdAt\n    }\n  }\n": typeof types.EmoteActivityDocument,
};
const documents: Documents = {
    "\n  fragment StudioPaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n    updatedAt\n  }\n": types.StudioPaintFieldsFragmentDoc,
    "\n  query StudioPaints {\n    cosmetics {\n      paints {\n        ...StudioPaintFields\n      }\n    }\n  }\n": types.StudioPaintsDocument,
    "\n  mutation StudioCreatePaint($input: PaintInput!) {\n    createPaint(input: $input) {\n      ...StudioPaintFields\n    }\n  }\n": types.StudioCreatePaintDocument,
    "\n  mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {\n    updatePaint(id: $id, input: $input) {\n      ...StudioPaintFields\n    }\n  }\n": types.StudioUpdatePaintDocument,
    "\n  query StudioCosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      id\n      host\n      ready\n      width\n      height\n      animated\n      files {\n        name\n        format\n        scale\n        width\n        height\n        isStatic\n      }\n    }\n  }\n": types.StudioCosmeticImageDocument,
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    channelCount\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": types.EmoteFieldsFragmentDoc,
    "\n  query Search($query: String!, $animated: Boolean, $nsfw: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort) {\n    searchEmotes(query: $query, animated: $animated, nsfw: $nsfw, page: $page, perPage: $perPage, sort: $sort) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SearchDocument,
    "\n  query FindPeople($query: String!, $page: Int, $perPage: Int) {\n    searchAccounts(query: $query, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n": types.FindPeopleDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": types.EmoteDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.ChannelDocument,
    "\n  query Set($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      capacity\n      owner {\n        id\n        displayName\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.SetDocument,
    "\n  query PublicSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      createdAt\n      owner {\n        id\n        displayName\n        avatar\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.PublicSetDocument,
    "\n  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {\n    cloneSet(id: $id, name: $name, visibility: $visibility) {\n      copied\n      skipped\n      set {\n        id\n        name\n      }\n    }\n  }\n": types.CloneSetDocument,
    "\n  query GlobalSet {\n    globalSet {\n      id\n      name\n      emotes {\n        name\n        zeroWidth\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.GlobalSetDocument,
    "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": types.MeDocument,
    "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.MyEmotesDocument,
    "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": types.UpdateEmoteDocument,
    "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n": types.DeleteEmoteDocument,
    "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SetFieldsFragmentDoc,
    "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n    editorSets {\n      ...SetFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n      }\n      permissions\n    }\n  }\n": types.MySetsDocument,
    "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      ...SetFields\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n": types.EditSetDocument,
    "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n": types.CreateSetDocument,
    "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n    }\n  }\n": types.UpdateSetDocument,
    "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n": types.DeleteSetDocument,
    "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n": types.AddSetEmoteDocument,
    "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n": types.UpdateSetEmoteDocument,
    "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n": types.RemoveSetEmoteDocument,
    "\n  mutation AddSetEditor($setId: String!, $accountId: String!) {\n    addSetEditor(setId: $setId, accountId: $accountId)\n  }\n": types.AddSetEditorDocument,
    "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n": types.RemoveSetEditorDocument,
    "\n  query FindAccount($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.FindAccountDocument,
    "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n      }\n    }\n    editorSets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n": types.MyChannelsDocument,
    "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n": types.UpdateChannelDocument,
    "\n  fragment AccountEditorFields on AccountEditor {\n    account {\n      id\n      displayName\n      avatar\n    }\n    permissions\n    grantedBy {\n      id\n      displayName\n    }\n    grantedAt\n  }\n": types.AccountEditorFieldsFragmentDoc,
    "\n  query Editors {\n    accountEditors {\n      ...AccountEditorFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      grantedAt\n    }\n  }\n": types.EditorsDocument,
    "\n  mutation AddAccountEditor($input: AddAccountEditorInput!) {\n    addAccountEditor(input: $input) {\n      ...AccountEditorFields\n    }\n  }\n": types.AddAccountEditorDocument,
    "\n  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!) {\n    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions) {\n      ...AccountEditorFields\n    }\n  }\n": types.UpdateAccountEditorDocument,
    "\n  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {\n    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)\n  }\n": types.RemoveAccountEditorDocument,
    "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n": types.UnlinkDocument,
    "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n": types.MyNotificationsDocument,
    "\n  mutation MarkRead($ids: [String!]) {\n    markNotificationsRead(ids: $ids)\n  }\n": types.MarkReadDocument,
    "\n  query UnreadCount {\n    unreadNotificationCount\n  }\n": types.UnreadCountDocument,
    "\n  query MySessions {\n    mySessions {\n      id\n      kind\n      device\n      createdAt\n      lastSeenAt\n      expiresAt\n      current\n    }\n  }\n": types.MySessionsDocument,
    "\n  mutation RevokeSession($id: String!) {\n    revokeSession(id: $id)\n  }\n": types.RevokeSessionDocument,
    "\n  mutation RevokeOtherSessions {\n    revokeOtherSessions\n  }\n": types.RevokeOtherSessionsDocument,
    "\n  mutation ExportMyData {\n    exportMyData\n  }\n": types.ExportMyDataDocument,
    "\n  mutation DeleteAccount($confirm: String!) {\n    deleteAccount(confirm: $confirm)\n  }\n": types.DeleteAccountDocument,
    "\n  query AdminCounts($review: Boolean!, $reports: Boolean!) {\n    reviewQueue(page: 1, perPage: 1) @include(if: $review) {\n      total\n    }\n    reportQueue(page: 1, perPage: 1) @include(if: $reports) {\n      total\n    }\n    globalSet {\n      id\n    }\n  }\n": types.AdminCountsDocument,
    "\n  query ReviewQueue($page: Int, $perPage: Int) {\n    reviewQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.ReviewQueueDocument,
    "\n  mutation Approve($id: String!) {\n    approveEmote(id: $id) {\n      id\n      state\n    }\n  }\n": types.ApproveDocument,
    "\n  mutation Reject($id: String!, $reason: String!) {\n    rejectEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n": types.RejectDocument,
    "\n  query ReportQueue($page: Int, $perPage: Int) {\n    reportQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        id\n        reason\n        reporterId\n        state\n        createdAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.ReportQueueDocument,
    "\n  mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {\n    resolveReport(id: $id, action: $action, note: $note) {\n      id\n      state\n    }\n  }\n": types.ResolveReportDocument,
    "\n  mutation Takedown($id: String!, $reason: String!) {\n    takedownEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n": types.TakedownDocument,
    "\n  fragment RoleFields on Role {\n    id\n    name\n    description\n    position\n    permissions\n    public\n  }\n": types.RoleFieldsFragmentDoc,
    "\n  query Roles {\n    roles {\n      ...RoleFields\n    }\n    permissions\n  }\n": types.RolesDocument,
    "\n  mutation CreateRole($input: CreateRoleInput!) {\n    createRole(input: $input) {\n      ...RoleFields\n    }\n  }\n": types.CreateRoleDocument,
    "\n  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {\n    updateRole(id: $id, input: $input) {\n      ...RoleFields\n    }\n  }\n": types.UpdateRoleDocument,
    "\n  mutation DeleteRole($id: String!) {\n    deleteRole(id: $id)\n  }\n": types.DeleteRoleDocument,
    "\n  query User($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.UserDocument,
    "\n  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {\n    accountStatus(accountId: $accountId) @include(if: $status) {\n      uploadsBlocked\n      blocked\n    }\n    accountRoles(accountId: $accountId) @include(if: $roles) {\n      ...RoleFields\n    }\n  }\n": types.UserDetailsDocument,
    "\n  query AllRoles {\n    roles {\n      ...RoleFields\n    }\n  }\n": types.AllRolesDocument,
    "\n  mutation Restrict($accountId: String!, $uploadsBlocked: Boolean, $blocked: Boolean) {\n    restrictAccount(accountId: $accountId, uploadsBlocked: $uploadsBlocked, blocked: $blocked) {\n      uploadsBlocked\n      blocked\n    }\n  }\n": types.RestrictDocument,
    "\n  mutation AssignRole($accountId: String!, $roleId: String!) {\n    assignRole(accountId: $accountId, roleId: $roleId)\n  }\n": types.AssignRoleDocument,
    "\n  mutation UnassignRole($accountId: String!, $roleId: String!) {\n    unassignRole(accountId: $accountId, roleId: $roleId)\n  }\n": types.UnassignRoleDocument,
    "\n  query Audit($targetKind: String, $targetId: String, $before: String, $limit: Int) {\n    auditLog(targetKind: $targetKind, targetId: $targetId, before: $before, limit: $limit) {\n      id\n      actorId\n      action\n      targetKind\n      targetId\n      before\n      after\n      createdAt\n    }\n  }\n": types.AuditDocument,
    "\n  mutation ReportEmote($id: String!, $reason: String!) {\n    reportEmote(id: $id, reason: $reason) {\n      id\n    }\n  }\n": types.ReportEmoteDocument,
    "\n  query MySetNames {\n    mySets {\n      id\n      name\n      kind\n      emotes {\n        emote {\n          id\n        }\n      }\n    }\n  }\n": types.MySetNamesDocument,
    "\n  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort) {\n    searchEmotes(query: \"\", owner: $owner, page: $page, perPage: $perPage, sort: $sort) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SearchByOwnerDocument,
    "\n  query PublicAccount($id: String!) {\n    account(id: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": types.PublicAccountDocument,
    "\n  query EmoteChannels($id: String!, $limit: Int) {\n    emoteChannels(id: $id, limit: $limit) {\n      platform\n      login\n      account {\n        displayName\n        avatar\n      }\n    }\n  }\n": types.EmoteChannelsDocument,
    "\n  fragment CosmeticImageFields on CosmeticImage {\n    id\n    kind\n    host\n    animated\n    ready\n    width\n    height\n    files {\n      name\n      width\n      height\n      isStatic\n    }\n  }\n": types.CosmeticImageFieldsFragmentDoc,
    "\n  fragment BadgeFields on Badge {\n    id\n    name\n    tooltip\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": types.BadgeFieldsFragmentDoc,
    "\n  fragment PaintFields on Paint {\n    id\n    name\n    kind\n    angle\n    repeat\n    shape\n    color\n    stops {\n      at\n      color\n    }\n    shadows {\n      x\n      y\n      radius\n      color\n    }\n    image {\n      ...CosmeticImageFields\n    }\n  }\n": types.PaintFieldsFragmentDoc,
    "\n  query Cosmetics {\n    cosmetics {\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n    }\n  }\n": types.CosmeticsDocument,
    "\n  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {\n    userCosmetics(platform: $platform, userIds: $userIds) {\n      platformUserId\n      badgeId\n      paintId\n    }\n  }\n": types.UserCosmeticsDocument,
    "\n  query AccountCosmetics($accountIds: [String!]!) {\n    accountCosmetics(accountIds: $accountIds) {\n      accountId\n      badge {\n        ...BadgeFields\n      }\n      paint {\n        ...PaintFields\n      }\n    }\n  }\n": types.AccountCosmeticsDocument,
    "\n  query AccountProfile($accountId: String!) {\n    accountProfile(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      identities {\n        platform\n        login\n        displayName\n      }\n      roles {\n        id\n        name\n      }\n      activeSets {\n        platform\n        login\n        setId\n        setName\n      }\n      createdAt\n    }\n  }\n": types.AccountProfileDocument,
    "\n  query MyCosmetics($accountId: String) {\n    myCosmetics(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      badges {\n        ...BadgeFields\n      }\n      paints {\n        ...PaintFields\n      }\n      badgeId\n      paintId\n    }\n  }\n": types.MyCosmeticsDocument,
    "\n  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {\n    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {\n      badgeId\n      paintId\n    }\n  }\n": types.SelectCosmeticsDocument,
    "\n  query CosmeticImage($id: String!) {\n    cosmeticImage(id: $id) {\n      ...CosmeticImageFields\n    }\n  }\n": types.CosmeticImageDocument,
    "\n  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {\n    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {\n      total\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        kind\n        cosmeticId\n        source\n        reason\n        grantedBy {\n          id\n          displayName\n        }\n        grantedAt\n      }\n    }\n  }\n": types.EntitlementsDocument,
    "\n  mutation CreateBadge($input: CreateBadgeInput!) {\n    createBadge(input: $input) {\n      ...BadgeFields\n    }\n  }\n": types.CreateBadgeDocument,
    "\n  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {\n    updateBadge(id: $id, input: $input) {\n      ...BadgeFields\n    }\n  }\n": types.UpdateBadgeDocument,
    "\n  mutation DeleteBadge($id: String!) {\n    deleteBadge(id: $id)\n  }\n": types.DeleteBadgeDocument,
    "\n  mutation DeletePaint($id: String!) {\n    deletePaint(id: $id)\n  }\n": types.DeletePaintDocument,
    "\n  mutation GrantCosmetic($input: GrantCosmeticInput!) {\n    grantCosmetic(input: $input) {\n      cosmeticId\n    }\n  }\n": types.GrantCosmeticDocument,
    "\n  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {\n    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)\n  }\n": types.RevokeCosmeticDocument,
    "\n  query AccountRoles($accountId: String!) {\n    accountRoles(accountId: $accountId) {\n      id\n      name\n      position\n    }\n  }\n": types.AccountRolesDocument,
    "\n  query AccountEditors($accountId: String) {\n    accountEditors(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n": types.AccountEditorsDocument,
    "\n  query EditorOf {\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n": types.EditorOfDocument,
    "\n  query AccountSets($accountId: String!) {\n    accountSets(accountId: $accountId, preview: 12) {\n      id\n      name\n      kind\n      capacity\n      slotsUsed\n      active\n      preview {\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.AccountSetsDocument,
    "\n  query ProfileMine {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n    }\n    mySets {\n      ...SetFields\n    }\n  }\n": types.ProfileMineDocument,
    "\n  query ProfileEdited {\n    editorChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      account {\n        id\n      }\n    }\n    editorSets {\n      ...SetFields\n    }\n  }\n": types.ProfileEditedDocument,
    "\n  query AccountActivity($accountId: String!, $before: String, $limit: Int) {\n    accountActivity(accountId: $accountId, before: $before, limit: $limit) {\n      id\n      kind\n      actor {\n        id\n        displayName\n        avatar\n      }\n      set {\n        id\n        name\n      }\n      emote {\n        id\n        name\n        host\n        files {\n          name\n          width\n          height\n          isStatic\n        }\n      }\n      fromName\n      toName\n      badge {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      paint {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      createdAt\n    }\n  }\n": types.AccountActivityDocument,
    "\n  query EmoteActivity($emoteId: String!, $before: String, $limit: Int) {\n    emoteActivity(emoteId: $emoteId, before: $before, limit: $limit) {\n      id\n      kind\n      actor {\n        id\n        displayName\n        avatar\n      }\n      set {\n        id\n        name\n      }\n      emote {\n        id\n        name\n        host\n        files {\n          name\n          width\n          height\n          isStatic\n        }\n      }\n      fromName\n      toName\n      badge {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      paint {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      createdAt\n    }\n  }\n": types.EmoteActivityDocument,
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
export function graphql(source: "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    channelCount\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n"): typeof import('./graphql').EmoteFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Search($query: String!, $animated: Boolean, $nsfw: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort) {\n    searchEmotes(query: $query, animated: $animated, nsfw: $nsfw, page: $page, perPage: $perPage, sort: $sort) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SearchDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query FindPeople($query: String!, $page: Int, $perPage: Int) {\n    searchAccounts(query: $query, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        account {\n          id\n          displayName\n          avatar\n        }\n        identities {\n          platform\n          login\n          displayName\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').FindPeopleDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n"): typeof import('./graphql').EmoteDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').ChannelDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Set($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      capacity\n      owner {\n        id\n        displayName\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').SetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PublicSet($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      visibility\n      capacity\n      slotsUsed\n      createdAt\n      owner {\n        id\n        displayName\n        avatar\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').PublicSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {\n    cloneSet(id: $id, name: $name, visibility: $visibility) {\n      copied\n      skipped\n      set {\n        id\n        name\n      }\n    }\n  }\n"): typeof import('./graphql').CloneSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query GlobalSet {\n    globalSet {\n      id\n      name\n      emotes {\n        name\n        zeroWidth\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n"): typeof import('./graphql').GlobalSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').MeDocument;
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
export function graphql(source: "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SetFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n    editorSets {\n      ...SetFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').MySetsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      ...SetFields\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n    editorOf {\n      account {\n        id\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').EditSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n"): typeof import('./graphql').CreateSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n    }\n  }\n"): typeof import('./graphql').UpdateSetDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n"): typeof import('./graphql').DeleteSetDocument;
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
export function graphql(source: "\n  mutation AddSetEditor($setId: String!, $accountId: String!) {\n    addSetEditor(setId: $setId, accountId: $accountId)\n  }\n"): typeof import('./graphql').AddSetEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n"): typeof import('./graphql').RemoveSetEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query FindAccount($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').FindAccountDocument;
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
export function graphql(source: "\n  fragment AccountEditorFields on AccountEditor {\n    account {\n      id\n      displayName\n      avatar\n    }\n    permissions\n    grantedBy {\n      id\n      displayName\n    }\n    grantedAt\n  }\n"): typeof import('./graphql').AccountEditorFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Editors {\n    accountEditors {\n      ...AccountEditorFields\n    }\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n      grantedAt\n    }\n  }\n"): typeof import('./graphql').EditorsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AddAccountEditor($input: AddAccountEditorInput!) {\n    addAccountEditor(input: $input) {\n      ...AccountEditorFields\n    }\n  }\n"): typeof import('./graphql').AddAccountEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!) {\n    updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions) {\n      ...AccountEditorFields\n    }\n  }\n"): typeof import('./graphql').UpdateAccountEditorDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {\n    removeAccountEditor(ownerId: $ownerId, accountId: $accountId)\n  }\n"): typeof import('./graphql').RemoveAccountEditorDocument;
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
export function graphql(source: "\n  query AdminCounts($review: Boolean!, $reports: Boolean!) {\n    reviewQueue(page: 1, perPage: 1) @include(if: $review) {\n      total\n    }\n    reportQueue(page: 1, perPage: 1) @include(if: $reports) {\n      total\n    }\n    globalSet {\n      id\n    }\n  }\n"): typeof import('./graphql').AdminCountsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ReviewQueue($page: Int, $perPage: Int) {\n    reviewQueue(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').ReviewQueueDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Approve($id: String!) {\n    approveEmote(id: $id) {\n      id\n      state\n    }\n  }\n"): typeof import('./graphql').ApproveDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Reject($id: String!, $reason: String!) {\n    rejectEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n"): typeof import('./graphql').RejectDocument;
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
export function graphql(source: "\n  mutation Takedown($id: String!, $reason: String!) {\n    takedownEmote(id: $id, reason: $reason) {\n      id\n      state\n    }\n  }\n"): typeof import('./graphql').TakedownDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment RoleFields on Role {\n    id\n    name\n    description\n    position\n    permissions\n    public\n  }\n"): typeof import('./graphql').RoleFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Roles {\n    roles {\n      ...RoleFields\n    }\n    permissions\n  }\n"): typeof import('./graphql').RolesDocument;
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
export function graphql(source: "\n  query User($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').UserDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {\n    accountStatus(accountId: $accountId) @include(if: $status) {\n      uploadsBlocked\n      blocked\n    }\n    accountRoles(accountId: $accountId) @include(if: $roles) {\n      ...RoleFields\n    }\n  }\n"): typeof import('./graphql').UserDetailsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AllRoles {\n    roles {\n      ...RoleFields\n    }\n  }\n"): typeof import('./graphql').AllRolesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Restrict($accountId: String!, $uploadsBlocked: Boolean, $blocked: Boolean) {\n    restrictAccount(accountId: $accountId, uploadsBlocked: $uploadsBlocked, blocked: $blocked) {\n      uploadsBlocked\n      blocked\n    }\n  }\n"): typeof import('./graphql').RestrictDocument;
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
export function graphql(source: "\n  query Audit($targetKind: String, $targetId: String, $before: String, $limit: Int) {\n    auditLog(targetKind: $targetKind, targetId: $targetId, before: $before, limit: $limit) {\n      id\n      actorId\n      action\n      targetKind\n      targetId\n      before\n      after\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AuditDocument;
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
export function graphql(source: "\n  query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort) {\n    searchEmotes(query: \"\", owner: $owner, page: $page, perPage: $perPage, sort: $sort) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SearchByOwnerDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query PublicAccount($id: String!) {\n    account(id: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n"): typeof import('./graphql').PublicAccountDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteChannels($id: String!, $limit: Int) {\n    emoteChannels(id: $id, limit: $limit) {\n      platform\n      login\n      account {\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').EmoteChannelsDocument;
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
export function graphql(source: "\n  query AccountProfile($accountId: String!) {\n    accountProfile(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      identities {\n        platform\n        login\n        displayName\n      }\n      roles {\n        id\n        name\n      }\n      activeSets {\n        platform\n        login\n        setId\n        setName\n      }\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AccountProfileDocument;
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
export function graphql(source: "\n  query AccountRoles($accountId: String!) {\n    accountRoles(accountId: $accountId) {\n      id\n      name\n      position\n    }\n  }\n"): typeof import('./graphql').AccountRolesDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AccountEditors($accountId: String) {\n    accountEditors(accountId: $accountId) {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').AccountEditorsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EditorOf {\n    editorOf {\n      account {\n        id\n        displayName\n        avatar\n      }\n      permissions\n    }\n  }\n"): typeof import('./graphql').EditorOfDocument;
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
export function graphql(source: "\n  query AccountActivity($accountId: String!, $before: String, $limit: Int) {\n    accountActivity(accountId: $accountId, before: $before, limit: $limit) {\n      id\n      kind\n      actor {\n        id\n        displayName\n        avatar\n      }\n      set {\n        id\n        name\n      }\n      emote {\n        id\n        name\n        host\n        files {\n          name\n          width\n          height\n          isStatic\n        }\n      }\n      fromName\n      toName\n      badge {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      paint {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      createdAt\n    }\n  }\n"): typeof import('./graphql').AccountActivityDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EmoteActivity($emoteId: String!, $before: String, $limit: Int) {\n    emoteActivity(emoteId: $emoteId, before: $before, limit: $limit) {\n      id\n      kind\n      actor {\n        id\n        displayName\n        avatar\n      }\n      set {\n        id\n        name\n      }\n      emote {\n        id\n        name\n        host\n        files {\n          name\n          width\n          height\n          isStatic\n        }\n      }\n      fromName\n      toName\n      badge {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      paint {\n        fromId\n        fromName\n        toId\n        toName\n      }\n      createdAt\n    }\n  }\n"): typeof import('./graphql').EmoteActivityDocument;


export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}
