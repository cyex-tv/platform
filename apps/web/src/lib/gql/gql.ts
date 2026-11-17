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
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": typeof types.EmoteFieldsFragmentDoc,
    "\n  query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int) {\n    searchEmotes(query: $query, animated: $animated, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SearchDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": typeof types.EmoteDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.ChannelDocument,
    "\n  query Set($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      capacity\n      owner {\n        id\n        displayName\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.SetDocument,
    "\n  query GlobalSet {\n    globalSet {\n      id\n      name\n      emotes {\n        name\n        zeroWidth\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": typeof types.GlobalSetDocument,
    "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.MeDocument,
    "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.MyEmotesDocument,
    "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": typeof types.UpdateEmoteDocument,
    "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n": typeof types.DeleteEmoteDocument,
    "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": typeof types.SetFieldsFragmentDoc,
    "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n  }\n": typeof types.MySetsDocument,
    "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      ...SetFields\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": typeof types.EditSetDocument,
    "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n": typeof types.CreateSetDocument,
    "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n    }\n  }\n": typeof types.UpdateSetDocument,
    "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n": typeof types.DeleteSetDocument,
    "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n": typeof types.AddSetEmoteDocument,
    "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n": typeof types.UpdateSetEmoteDocument,
    "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n": typeof types.RemoveSetEmoteDocument,
    "\n  mutation AddSetEditor($setId: String!, $accountId: String!) {\n    addSetEditor(setId: $setId, accountId: $accountId)\n  }\n": typeof types.AddSetEditorDocument,
    "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n": typeof types.RemoveSetEditorDocument,
    "\n  query FindAccount($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.FindAccountDocument,
    "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n  }\n": typeof types.MyChannelsDocument,
    "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n": typeof types.UpdateChannelDocument,
    "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n": typeof types.UnlinkDocument,
    "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n": typeof types.MyNotificationsDocument,
    "\n  mutation MarkRead {\n    markNotificationsRead\n  }\n": typeof types.MarkReadDocument,
};
const documents: Documents = {
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": types.EmoteFieldsFragmentDoc,
    "\n  query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int) {\n    searchEmotes(query: $query, animated: $animated, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SearchDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": types.EmoteDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.ChannelDocument,
    "\n  query Set($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      capacity\n      owner {\n        id\n        displayName\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.SetDocument,
    "\n  query GlobalSet {\n    globalSet {\n      id\n      name\n      emotes {\n        name\n        zeroWidth\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.GlobalSetDocument,
    "\n  query Me {\n    me {\n      id\n      createdAt\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": types.MeDocument,
    "\n  query MyEmotes($page: Int, $perPage: Int) {\n    myEmotes(page: $page, perPage: $perPage) {\n      total\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.MyEmotesDocument,
    "\n  mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {\n    updateEmote(id: $id, input: $input) {\n      ...EmoteFields\n    }\n  }\n": types.UpdateEmoteDocument,
    "\n  mutation DeleteEmote($id: String!) {\n    deleteEmote(id: $id)\n  }\n": types.DeleteEmoteDocument,
    "\n  fragment SetFields on EmoteSet {\n    id\n    name\n    kind\n    capacity\n    owner {\n      id\n      displayName\n      avatar\n    }\n    emotes {\n      name\n      zeroWidth\n      addedAt\n      emote {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SetFieldsFragmentDoc,
    "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n  }\n": types.MySetsDocument,
    "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      ...SetFields\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n": types.EditSetDocument,
    "\n  mutation CreateSet($input: CreateSetInput!) {\n    createSet(input: $input) {\n      id\n    }\n  }\n": types.CreateSetDocument,
    "\n  mutation UpdateSet($id: String!, $input: UpdateSetInput!) {\n    updateSet(id: $id, input: $input) {\n      id\n      name\n    }\n  }\n": types.UpdateSetDocument,
    "\n  mutation DeleteSet($id: String!) {\n    deleteSet(id: $id)\n  }\n": types.DeleteSetDocument,
    "\n  mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {\n    addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n    }\n  }\n": types.AddSetEmoteDocument,
    "\n  mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {\n    updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {\n      name\n      zeroWidth\n    }\n  }\n": types.UpdateSetEmoteDocument,
    "\n  mutation RemoveSetEmote($setId: String!, $emoteId: String!) {\n    removeSetEmote(setId: $setId, emoteId: $emoteId)\n  }\n": types.RemoveSetEmoteDocument,
    "\n  mutation AddSetEditor($setId: String!, $accountId: String!) {\n    addSetEditor(setId: $setId, accountId: $accountId)\n  }\n": types.AddSetEditorDocument,
    "\n  mutation RemoveSetEditor($setId: String!, $accountId: String!) {\n    removeSetEditor(setId: $setId, accountId: $accountId)\n  }\n": types.RemoveSetEditorDocument,
    "\n  query FindAccount($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      login\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.FindAccountDocument,
    "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n  }\n": types.MyChannelsDocument,
    "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n": types.UpdateChannelDocument,
    "\n  mutation Unlink($platform: Platform!) {\n    unlinkIdentity(platform: $platform) {\n      id\n    }\n  }\n": types.UnlinkDocument,
    "\n  query MyNotifications {\n    myNotifications {\n      id\n      kind\n      payload\n      createdAt\n      read\n    }\n  }\n": types.MyNotificationsDocument,
    "\n  mutation MarkRead {\n    markNotificationsRead\n  }\n": types.MarkReadDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n"): typeof import('./graphql').EmoteFieldsFragmentDoc;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int) {\n    searchEmotes(query: $query, animated: $animated, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n"): typeof import('./graphql').SearchDocument;
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
export function graphql(source: "\n  query MySets {\n    mySets {\n      ...SetFields\n    }\n  }\n"): typeof import('./graphql').MySetsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query EditSet($id: String!) {\n    set(id: $id) {\n      ...SetFields\n    }\n    setEditors(setId: $id) {\n      id\n      displayName\n      avatar\n    }\n  }\n"): typeof import('./graphql').EditSetDocument;
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
export function graphql(source: "\n  query MyChannels {\n    myChannels {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n    }\n    mySets {\n      id\n      name\n      kind\n      owner {\n        id\n      }\n    }\n  }\n"): typeof import('./graphql').MyChannelsDocument;
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {\n    updateChannelSettings(platform: $platform, id: $id, input: $input) {\n      activeSetId\n      allowPersonalEmotes\n    }\n  }\n"): typeof import('./graphql').UpdateChannelDocument;
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
export function graphql(source: "\n  mutation MarkRead {\n    markNotificationsRead\n  }\n"): typeof import('./graphql').MarkReadDocument;


export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}
