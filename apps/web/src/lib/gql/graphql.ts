/* eslint-disable */
import type { DocumentTypeDecoration } from '@graphql-typed-document-node/core';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
};

/** What an account may do (`users.restrict`). */
export type AccountStatus = {
  __typename?: 'AccountStatus';
  /** No sign-in, no sessions. */
  blocked: Scalars['Boolean']['output'];
  id: Scalars['String']['output'];
  /** No uploads. */
  uploadsBlocked: Scalars['Boolean']['output'];
};

/** A short reference to an account, for owners and editors. */
export type AccountSummary = {
  __typename?: 'AccountSummary';
  avatar?: Maybe<Scalars['String']['output']>;
  displayName: Scalars['String']['output'];
  id: Scalars['String']['output'];
};

export type AddSetEmoteInput = {
  /** Rename it in this set. Defaults to the emote's own name. */
  name?: InputMaybe<Scalars['String']['input']>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

/** One audited change (ADR 0010). `before` and `after` are JSON, as text. */
export type AuditEntry = {
  __typename?: 'AuditEntry';
  /** For example `role.created` or `set.editor_added`. */
  action: Scalars['String']['output'];
  /** The account that made the change; empty for changes the system made. */
  actorId?: Maybe<Scalars['String']['output']>;
  after?: Maybe<Scalars['String']['output']>;
  before?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  targetId: Scalars['String']['output'];
  /** For example `role`, `account` or `set`. */
  targetKind: Scalars['String']['output'];
};

export type Channel = {
  __typename?: 'Channel';
  account: AccountSummary;
  activeSetId?: Maybe<Scalars['String']['output']>;
  /** Whether other people's personal emotes show in this channel. */
  allowPersonalEmotes: Scalars['Boolean']['output'];
  login: Scalars['String']['output'];
  platform: Platform;
  /** Twitch: the broadcaster's user ID. Kick: `broadcaster_user_id`. */
  platformChannelId: Scalars['String']['output'];
};

export type CreateRoleInput = {
  description?: InputMaybe<Scalars['String']['input']>;
  name: Scalars['String']['input'];
  permissions: Array<Scalars['String']['input']>;
  position: Scalars['Int']['input'];
};

export type CreateSetInput = {
  /** `CHANNEL` or `PERSONAL`. The global set exists already. */
  kind: SetKind;
  name: Scalars['String']['input'];
};

export type Emote = {
  __typename?: 'Emote';
  animated: Scalars['Boolean']['output'];
  createdAt: Scalars['String']['output'];
  files: Array<EmoteFile>;
  flags: EmoteFlags;
  /** For example `https://cdn.cyex.tv/emotes/<sha256>`. */
  host: Scalars['String']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  state: EmoteState;
  tags: Array<Scalars['String']['output']>;
};

/** One file of an emote, served at `{host}/{name}`. */
export type EmoteFile = {
  __typename?: 'EmoteFile';
  format: ImageFormat;
  height: Scalars['Int']['output'];
  /** The first frame of an animated emote. */
  isStatic: Scalars['Boolean']['output'];
  /** For example `2x.webp` or `1x_static.avif`. */
  name: Scalars['String']['output'];
  /** 1 to 4. */
  scale: Scalars['Int']['output'];
  /** Bytes. */
  size: Scalars['Int']['output'];
  width: Scalars['Int']['output'];
};

export type EmoteFlags = {
  __typename?: 'EmoteFlags';
  nsfw: Scalars['Boolean']['output'];
  /** Only its owner can add it to sets. */
  private: Scalars['Boolean']['output'];
  zeroWidth: Scalars['Boolean']['output'];
};

export type EmoteSet = {
  __typename?: 'EmoteSet';
  capacity: Scalars['Int']['output'];
  emotes: Array<SetEmote>;
  id: Scalars['String']['output'];
  kind: SetKind;
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
};

export type EmoteState =
  | 'APPROVED'
  | 'PENDING'
  | 'REJECTED'
  | 'TAKEN_DOWN';

/** One change, as delivered on both sockets. */
export type Event = {
  __typename?: 'Event';
  createdAt: Scalars['String']['output'];
  emoteId?: Maybe<Scalars['String']['output']>;
  eventType: EventType;
  /** Increases with every event. Send the last one you saw to resume. */
  id: Scalars['String']['output'];
  /** The emote's name in the set, for added and renamed. */
  name?: Maybe<Scalars['String']['output']>;
  setId?: Maybe<Scalars['String']['output']>;
  /** The topic it was delivered for, for example `channel:twitch:123`. */
  topic: Scalars['String']['output'];
};

/** What changed. On the event WebSocket these are written `set.emote_added` and so on. */
export type EventType =
  | 'CHANNEL_ACTIVE_SET_CHANGED'
  | 'EMOTE_UPDATED'
  | 'PERSONAL_UPDATED'
  /** Too much was missed: refetch what you show. */
  | 'RESYNC'
  | 'SET_EMOTE_ADDED'
  | 'SET_EMOTE_REMOVED'
  | 'SET_EMOTE_RENAMED';

/** A Twitch or Kick account linked to a cyex.tv account. */
export type Identity = {
  __typename?: 'Identity';
  avatar?: Maybe<Scalars['String']['output']>;
  displayName: Scalars['String']['output'];
  login: Scalars['String']['output'];
  platform: Platform;
  /** The platform's numeric user ID, as text. */
  platformUserId: Scalars['String']['output'];
};

export type ImageFormat =
  | 'AVIF'
  | 'WEBP';

/** The signed-in account. */
export type Me = {
  __typename?: 'Me';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  identities: Array<Identity>;
  /** Permission names from the account's roles (ADR 0010). */
  permissions: Array<Scalars['String']['output']>;
};

export type Mutation = {
  __typename?: 'Mutation';
  /** Make someone an editor of a set. Owner only. */
  addSetEditor: Scalars['Boolean']['output'];
  /** Add an emote to a set. Personal sets take approved emotes only. */
  addSetEmote: SetEmote;
  /** Approve a pending emote. Needs `emotes.review`. Audited. */
  approveEmote: Emote;
  /** Give an account a role. Needs `roles.manage`. Audited. */
  assignRole: Scalars['Boolean']['output'];
  /** Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited. */
  createRole: Role;
  /** Create a channel set, or your personal set (one per account). */
  createSet: EmoteSet;
  /** Delete your emote. It is removed from every set. */
  deleteEmote: Scalars['Boolean']['output'];
  /** Delete a role below your own highest one. Needs `roles.manage`. Audited. */
  deleteRole: Scalars['Boolean']['output'];
  /** Delete a set. Owner only. A channel's active set can't be deleted. */
  deleteSet: Scalars['Boolean']['output'];
  /** Mark all your notifications read. Returns how many were unread. */
  markNotificationsRead: Scalars['Int']['output'];
  /** Reject a pending emote with a reason. Needs `emotes.review`. Audited. */
  rejectEmote: Emote;
  /** Remove an editor. Owner, or the editor themselves. */
  removeSetEditor: Scalars['Boolean']['output'];
  /** Remove an emote from a set. */
  removeSetEmote: Scalars['Boolean']['output'];
  /** Report an emote you can see, with a reason. At most 10 an hour, one open report per emote. */
  reportEmote: Report;
  /**
   * Dismiss a report, or take its emote down (`emotes.takedown`). Needs `emotes.review` or
   * `emotes.takedown`. Audited.
   */
  resolveReport: Report;
  /**
   * Block an account's uploads, or the whole account. Needs `users.restrict`; the owner is
   * protected. Audited.
   */
  restrictAccount: AccountStatus;
  /**
   * Take an emote's image down everywhere: out of every set, its file blocked and deleted,
   * its owners told. Needs `emotes.takedown`. Audited.
   */
  takedownEmote: Emote;
  /**
   * Take a role from an account. The owner and the last `roles.manage` holder are protected.
   * Needs `roles.manage`. Audited.
   */
  unassignRole: Scalars['Boolean']['output'];
  /** Unlink a platform from your account. The last identity can't be removed. */
  unlinkIdentity: Me;
  /** Change a channel's active set and settings. Only the channel's owner. */
  updateChannelSettings: Channel;
  /** Change an emote's name, tags or flags. Owner, or `emotes.edit_any`. */
  updateEmote: Emote;
  /** Change a role below your own highest one. Needs `roles.manage`. Audited. */
  updateRole: Role;
  /** Rename a set or change its capacity. Owner or editor; capacity needs `sets.edit_any`. */
  updateSet: EmoteSet;
  /** Rename an emote in a set, or change its zero-width flag there. */
  updateSetEmote: SetEmote;
};


export type MutationAddSetEditorArgs = {
  accountId: Scalars['String']['input'];
  setId: Scalars['String']['input'];
};


export type MutationAddSetEmoteArgs = {
  emoteId: Scalars['String']['input'];
  input?: InputMaybe<AddSetEmoteInput>;
  setId: Scalars['String']['input'];
};


export type MutationApproveEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationAssignRoleArgs = {
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
};


export type MutationCreateRoleArgs = {
  input: CreateRoleInput;
};


export type MutationCreateSetArgs = {
  input: CreateSetInput;
};


export type MutationDeleteEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteRoleArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteSetArgs = {
  id: Scalars['String']['input'];
};


export type MutationRejectEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationRemoveSetEditorArgs = {
  accountId: Scalars['String']['input'];
  setId: Scalars['String']['input'];
};


export type MutationRemoveSetEmoteArgs = {
  emoteId: Scalars['String']['input'];
  setId: Scalars['String']['input'];
};


export type MutationReportEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationResolveReportArgs = {
  action: ReportAction;
  id: Scalars['String']['input'];
  note?: InputMaybe<Scalars['String']['input']>;
};


export type MutationRestrictAccountArgs = {
  accountId: Scalars['String']['input'];
  blocked?: InputMaybe<Scalars['Boolean']['input']>;
  uploadsBlocked?: InputMaybe<Scalars['Boolean']['input']>;
};


export type MutationTakedownEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationUnassignRoleArgs = {
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
};


export type MutationUnlinkIdentityArgs = {
  platform: Platform;
};


export type MutationUpdateChannelSettingsArgs = {
  id: Scalars['String']['input'];
  input: UpdateChannelSettingsInput;
  platform: Platform;
};


export type MutationUpdateEmoteArgs = {
  id: Scalars['String']['input'];
  input: UpdateEmoteInput;
};


export type MutationUpdateRoleArgs = {
  id: Scalars['String']['input'];
  input: UpdateRoleInput;
};


export type MutationUpdateSetArgs = {
  id: Scalars['String']['input'];
  input: UpdateSetInput;
};


export type MutationUpdateSetEmoteArgs = {
  emoteId: Scalars['String']['input'];
  input: UpdateSetEmoteInput;
  setId: Scalars['String']['input'];
};

/** Something the platform tells you, such as a takedown of your emote. */
export type Notification = {
  __typename?: 'Notification';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** For example `emote.taken_down`. */
  kind: Scalars['String']['output'];
  /** JSON, as text. */
  payload: Scalars['String']['output'];
  read: Scalars['Boolean']['output'];
};

export type PersonalSetEntry = {
  __typename?: 'PersonalSetEntry';
  platformUserId: Scalars['String']['output'];
  set: EmoteSet;
};

/** A streaming platform. */
export type Platform =
  | 'KICK'
  | 'TWITCH';

export type Query = {
  __typename?: 'Query';
  /** An account's roles. Yours, or anyone's with `roles.manage` or `users.view`. */
  accountRoles: Array<Role>;
  /**
   * Audited changes, newest first. Needs `audit.view`. Page back with `before` (an entry ID);
   * `limit` is 1 to 100, default 50.
   */
  auditLog: Array<AuditEntry>;
  /** A channel by its platform ID. */
  channel?: Maybe<Channel>;
  /** A channel by its login or slug. Logins change; store the ID, not the login. */
  channelByLogin?: Maybe<Channel>;
  /** An emote. Pending ones are visible to their owner and to moderators only. */
  emote?: Maybe<Emote>;
  /** The cyex.tv global set, shown in every channel. */
  globalSet: EmoteSet;
  /** The signed-in account. */
  me: Me;
  /** Your channels: one per linked platform you have signed in with. */
  myChannels: Array<Channel>;
  /** Your notifications, newest first (the latest 100). */
  myNotifications: Array<Notification>;
  /** The sets you own, then the ones you edit. Pending emotes included. */
  mySets: Array<EmoteSet>;
  /** Every permission name a role can hold. */
  permissions: Array<Scalars['String']['output']>;
  /** One user's personal set. */
  personalSet?: Maybe<EmoteSet>;
  /** Personal sets for up to 100 chatters at once. */
  personalSets: Array<PersonalSetEntry>;
  /** Open reports, oldest first. Needs `emotes.review` or `emotes.takedown`. */
  reportQueue: ReportQueue;
  /** Pending emotes, oldest first. Needs `emotes.review`. `perPage` is 1 to 100, default 50. */
  reviewQueue: ReviewQueue;
  /** All roles. Needs `roles.manage`. */
  roles: Array<Role>;
  /**
   * Approved, listed emotes by name and tag, best match first, then the most used.
   * `perPage` is 1 to 100, default 50. NSFW emotes only with `nsfw: true`.
   */
  searchEmotes: SearchResults;
  /** A set with its emotes. */
  set?: Maybe<EmoteSet>;
};


export type QueryAccountRolesArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAuditLogArgs = {
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  targetKind?: InputMaybe<Scalars['String']['input']>;
};


export type QueryChannelArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryChannelByLoginArgs = {
  login: Scalars['String']['input'];
  platform: Platform;
};


export type QueryEmoteArgs = {
  id: Scalars['String']['input'];
};


export type QueryMyNotificationsArgs = {
  unreadOnly?: InputMaybe<Scalars['Boolean']['input']>;
};


export type QueryPersonalSetArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryPersonalSetsArgs = {
  platform: Platform;
  userIds: Array<Scalars['String']['input']>;
};


export type QueryReportQueueArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryReviewQueueArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QuerySearchEmotesArgs = {
  animated?: InputMaybe<Scalars['Boolean']['input']>;
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  owner?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query: Scalars['String']['input'];
};


export type QuerySetArgs = {
  id: Scalars['String']['input'];
};

export type Report = {
  __typename?: 'Report';
  createdAt: Scalars['String']['output'];
  /** The reported emote, as moderators see it. Empty if it was deleted since. */
  emote?: Maybe<Emote>;
  id: Scalars['String']['output'];
  reason: Scalars['String']['output'];
  reporterId?: Maybe<Scalars['String']['output']>;
  resolution?: Maybe<Scalars['String']['output']>;
  resolvedAt?: Maybe<Scalars['String']['output']>;
  state: ReportState;
};

/** What a moderator does with a report. */
export type ReportAction =
  | 'DISMISS'
  /** Take the emote down (needs `emotes.takedown`). */
  | 'TAKE_DOWN';

export type ReportQueue = {
  __typename?: 'ReportQueue';
  items: Array<Report>;
  total: Scalars['Int']['output'];
};

export type ReportState =
  /** The emote was taken down. */
  | 'ACTIONED'
  | 'DISMISSED'
  | 'OPEN';

export type ReviewQueue = {
  __typename?: 'ReviewQueue';
  items: Array<Emote>;
  total: Scalars['Int']['output'];
};

export type Role = {
  __typename?: 'Role';
  description?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  permissions: Array<Scalars['String']['output']>;
  /** Higher means more senior. You can only manage roles below your own highest one. */
  position: Scalars['Int']['output'];
};

export type SearchResults = {
  __typename?: 'SearchResults';
  items: Array<Emote>;
  page: Scalars['Int']['output'];
  perPage: Scalars['Int']['output'];
  total: Scalars['Int']['output'];
};

/** An emote as it sits in a set: the name it is typed as there, and who added it. */
export type SetEmote = {
  __typename?: 'SetEmote';
  addedAt: Scalars['String']['output'];
  addedBy?: Maybe<Scalars['String']['output']>;
  emote: Emote;
  /** The name in this set. Differs from `emote.name` when the set renamed it. */
  name: Scalars['String']['output'];
  zeroWidth: Scalars['Boolean']['output'];
};

export type SetKind =
  | 'CHANNEL'
  | 'GLOBAL'
  | 'PERSONAL';

export type Subscription = {
  __typename?: 'Subscription';
  /**
   * Live updates for up to 500 topics: `channel:<platform>:<id>`, `set:<id>`,
   * `user:<platform>:<id>` (ADR 0005). The same events as the event WebSocket.
   */
  events: Event;
};


export type SubscriptionEventsArgs = {
  topics: Array<Scalars['String']['input']>;
};

export type UpdateChannelSettingsInput = {
  /** One of the owner's channel sets. An empty string clears it. */
  activeSetId?: InputMaybe<Scalars['String']['input']>;
  allowPersonalEmotes?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateEmoteInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  private?: InputMaybe<Scalars['Boolean']['input']>;
  tags?: InputMaybe<Array<Scalars['String']['input']>>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateRoleInput = {
  /** An empty string clears it. */
  description?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  permissions?: InputMaybe<Array<Scalars['String']['input']>>;
  position?: InputMaybe<Scalars['Int']['input']>;
};

export type UpdateSetEmoteInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateSetInput = {
  /** Needs `sets.edit_any` to raise above the default. */
  capacity?: InputMaybe<Scalars['Int']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
};

export type EmoteFieldsFragment = { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> };

export type SearchQueryVariables = Exact<{
  query: Scalars['String']['input'];
  animated?: InputMaybe<Scalars['Boolean']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type SearchQuery = { __typename?: 'Query', searchEmotes: { __typename?: 'SearchResults', total: number, page: number, perPage: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type EmoteQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type EmoteQuery = { __typename?: 'Query', emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type ChannelQueryVariables = Exact<{
  platform: Platform;
  login: Scalars['String']['input'];
}>;


export type ChannelQuery = { __typename?: 'Query', channelByLogin?: { __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } | null };

export type SetQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type SetQuery = { __typename?: 'Query', set?: { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } | null };

export type GlobalSetQueryVariables = Exact<{ [key: string]: never; }>;


export type GlobalSetQuery = { __typename?: 'Query', globalSet: { __typename?: 'EmoteSet', id: string, name: string, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { __typename?: 'Query', me: { __typename?: 'Me', id: string, permissions: Array<string>, identities: Array<{ __typename?: 'Identity', platform: Platform, platformUserId: string, login: string, displayName: string, avatar?: string | null }> } };

export class TypedDocumentString<TResult, TVariables>
  extends String
  implements DocumentTypeDecoration<TResult, TVariables>
{
  __apiType?: NonNullable<DocumentTypeDecoration<TResult, TVariables>['__apiType']>;
  private value: string;
  public __meta__?: Record<string, any> | undefined;

  constructor(value: string, __meta__?: Record<string, any> | undefined) {
    super(value);
    this.value = value;
    this.__meta__ = __meta__;
  }

  override toString(): string & DocumentTypeDecoration<TResult, TVariables> {
    return this.value;
  }
}
export const EmoteFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"EmoteFields"}) as unknown as TypedDocumentString<EmoteFieldsFragment, unknown>;
export const SearchDocument = new TypedDocumentString(`
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
}`) as unknown as TypedDocumentString<SearchQuery, SearchQueryVariables>;
export const EmoteDocument = new TypedDocumentString(`
    query Emote($id: String!) {
  emote(id: $id) {
    ...EmoteFields
  }
}
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
}`) as unknown as TypedDocumentString<EmoteQuery, EmoteQueryVariables>;
export const ChannelDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<ChannelQuery, ChannelQueryVariables>;
export const SetDocument = new TypedDocumentString(`
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
}`) as unknown as TypedDocumentString<SetQuery, SetQueryVariables>;
export const GlobalSetDocument = new TypedDocumentString(`
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
}`) as unknown as TypedDocumentString<GlobalSetQuery, GlobalSetQueryVariables>;
export const MeDocument = new TypedDocumentString(`
    query Me {
  me {
    id
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
    `) as unknown as TypedDocumentString<MeQuery, MeQueryVariables>;