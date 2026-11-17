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
    "\n  query Me {\n    me {\n      id\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": typeof types.MeDocument,
};
const documents: Documents = {
    "\n  fragment EmoteFields on Emote {\n    id\n    name\n    state\n    animated\n    tags\n    host\n    createdAt\n    flags {\n      zeroWidth\n      nsfw\n      private\n    }\n    owner {\n      id\n      displayName\n      avatar\n    }\n    files {\n      name\n      format\n      scale\n      width\n      height\n      size\n      isStatic\n    }\n  }\n": types.EmoteFieldsFragmentDoc,
    "\n  query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int) {\n    searchEmotes(query: $query, animated: $animated, page: $page, perPage: $perPage) {\n      total\n      page\n      perPage\n      items {\n        ...EmoteFields\n      }\n    }\n  }\n": types.SearchDocument,
    "\n  query Emote($id: String!) {\n    emote(id: $id) {\n      ...EmoteFields\n    }\n  }\n": types.EmoteDocument,
    "\n  query Channel($platform: Platform!, $login: String!) {\n    channelByLogin(platform: $platform, login: $login) {\n      platform\n      platformChannelId\n      login\n      activeSetId\n      allowPersonalEmotes\n      account {\n        id\n        displayName\n        avatar\n      }\n    }\n  }\n": types.ChannelDocument,
    "\n  query Set($id: String!) {\n    set(id: $id) {\n      id\n      name\n      kind\n      capacity\n      owner {\n        id\n        displayName\n      }\n      emotes {\n        name\n        zeroWidth\n        addedAt\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.SetDocument,
    "\n  query GlobalSet {\n    globalSet {\n      id\n      name\n      emotes {\n        name\n        zeroWidth\n        emote {\n          ...EmoteFields\n        }\n      }\n    }\n  }\n": types.GlobalSetDocument,
    "\n  query Me {\n    me {\n      id\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n": types.MeDocument,
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
export function graphql(source: "\n  query Me {\n    me {\n      id\n      permissions\n      identities {\n        platform\n        platformUserId\n        login\n        displayName\n        avatar\n      }\n    }\n  }\n"): typeof import('./graphql').MeDocument;


export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}
