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

/** The badges and paints an account owns, when it shows them. */
export type AccountCollection = {
  __typename?: 'AccountCollection';
  accountId: Scalars['String']['output'];
  /** Newest first. */
  items: Array<CollectionItem>;
  /** Shown on the public profile. The owner sees their own collection either way. */
  public: Scalars['Boolean']['output'];
};

/** What one account shows, with the badge and paint themselves (`accountCosmetics`). */
export type AccountCosmetics = {
  __typename?: 'AccountCosmetics';
  accountId: Scalars['String']['output'];
  badge?: Maybe<Badge>;
  paint?: Maybe<Paint>;
};

/** Someone who edits an account, with what they may do. */
export type AccountEditor = {
  __typename?: 'AccountEditor';
  account: AccountSummary;
  /** They may work on every set of the account, new ones included. False: only `setIds`. */
  allSets: Scalars['Boolean']['output'];
  grantedAt: Scalars['String']['output'];
  /** Who added them: the owner, or an editor with `editors`. */
  grantedBy?: Maybe<AccountSummary>;
  permissions: Array<EditorPermission>;
  /** The sets they may work on when `allSets` is false. Empty when it is true. */
  setIds: Array<Scalars['String']['output']>;
  updatedAt: Scalars['String']['output'];
};

/** One `searchAccounts` hit: the account's public summary and its linked logins. */
export type AccountHit = {
  __typename?: 'AccountHit';
  account: AccountSummary;
  /** Twitch first. */
  identities: Array<PublicIdentity>;
};

/**
 * An account's public profile, like 7TV's public user pages. Listed sets come from
 * `accountSets`.
 */
export type AccountProfile = {
  __typename?: 'AccountProfile';
  account: AccountSummary;
  /** Each channel's active set, for channels showing one. */
  activeSets: Array<ActiveChannelSet>;
  /** The account shows the badges and paints it owns (`accountCollection`). Off by default. */
  collectionPublic: Scalars['Boolean']['output'];
  createdAt: Scalars['String']['output'];
  /** The account's editors, oldest first: who they are only, never what they may do. */
  editors: Array<AccountSummary>;
  /** Linked Twitch and Kick accounts: platform, login and display name only. Twitch first. */
  identities: Array<PublicIdentity>;
  /**
   * The colour of the profile picture's ring: the `#rrggbb` of the most senior of the
   * public `roles` that has one; null when none does (the neutral ring). Never from the
   * badge or paint.
   */
  roleColor?: Maybe<Scalars['String']['output']>;
  /** Roles marked public, most senior first. */
  roles: Array<PublicRole>;
};

/**
 * Reports about an account's emotes and reports it filed (`emotes.review` or
 * `emotes.takedown`), newest first, at most 50 each.
 */
export type AccountReports = {
  __typename?: 'AccountReports';
  about: Array<Report>;
  by: Array<Report>;
};

export type AccountResults = {
  __typename?: 'AccountResults';
  items: Array<AccountHit>;
  /**
   * Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
   * page. Opaque: a list's cursor only works for the same list, filters and order.
   */
  next?: Maybe<Scalars['String']['output']>;
  page: Scalars['Int']['output'];
  perPage: Scalars['Int']['output'];
  /** How many accounts match, counted up to 1000. */
  total: Scalars['Int']['output'];
  /** More than `total` match: show it as "1000+". */
  totalCapped: Scalars['Boolean']['output'];
};

/** One place you are signed in. Never holds the token or anything made from it. */
export type AccountSession = {
  __typename?: 'AccountSession';
  /**
   * The app that signed in through `/v1/auth/app` (ADR 0021), such as `cyex-deck`; null for
   * website sessions.
   */
  appClientId?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  /** The session this request came with. */
  current: Scalars['Boolean']['output'];
  /**
   * For example `Firefox on Windows`, `cyex.tv app` or `Unknown device`, from the user agent
   * that signed in; `Console token` for a console token.
   */
  device: Scalars['String']['output'];
  expiresAt: Scalars['String']['output'];
  /** Pass to `revokeSession`. */
  id: Scalars['String']['output'];
  kind: SessionKind;
  /** Updated at most once an hour. */
  lastSeenAt: Scalars['String']['output'];
};

/** What an account may do (`users.restrict`). */
export type AccountStatus = {
  __typename?: 'AccountStatus';
  /** No sign-in, no sessions. */
  blocked: Scalars['Boolean']['output'];
  blockedUntil?: Maybe<Scalars['String']['output']>;
  /** The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  /** No reports or tickets. */
  ticketsBlocked: Scalars['Boolean']['output'];
  ticketsBlockedUntil?: Maybe<Scalars['String']['output']>;
  /** No uploads. */
  uploadsBlocked: Scalars['Boolean']['output'];
  /** When each hold ends; empty while it is off or for good. */
  uploadsBlockedUntil?: Maybe<Scalars['String']['output']>;
};

/**
 * A subscription to a plan. None exist yet; `accountSubscriptions` returns an empty list until
 * they do.
 */
export type AccountSubscription = {
  __typename?: 'AccountSubscription';
  /** Empty while it renews. */
  endsAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  plan: Scalars['String']['output'];
  /** `purchase`, `gift` or `staff`. */
  source: Scalars['String']['output'];
  startsAt: Scalars['String']['output'];
  state: SubscriptionState;
};

/** A short reference to an account, for owners and editors. */
export type AccountSummary = {
  __typename?: 'AccountSummary';
  avatar?: Maybe<Scalars['String']['output']>;
  displayName: Scalars['String']['output'];
  /** The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
};

/** The set a channel shows now, on a profile ("Active emotes"). */
export type ActiveChannelSet = {
  __typename?: 'ActiveChannelSet';
  /** The channel's login. */
  login: Scalars['String']['output'];
  platform: Platform;
  setId: Scalars['String']['output'];
  setName: Scalars['String']['output'];
};

/**
 * A badge or paint change: what it was and what it is now. Empty ends mean none; names are
 * empty when the cosmetic was deleted since.
 */
export type ActivityCosmeticChange = {
  __typename?: 'ActivityCosmeticChange';
  fromId?: Maybe<Scalars['String']['output']>;
  fromName?: Maybe<Scalars['String']['output']>;
  toId?: Maybe<Scalars['String']['output']>;
  toName?: Maybe<Scalars['String']['output']>;
};

/** An emote, as a feed line names it. `name` is what it was called in the set at the time. */
export type ActivityEmote = {
  __typename?: 'ActivityEmote';
  files: Array<EmoteFile>;
  /** For example `https://cdn.cyex.tv/emotes/<sha256>`. */
  host: Scalars['String']['output'];
  /** The emote's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

/** One line of an account's public activity feed (7TV's "Activity" tab). */
export type ActivityEntry = {
  __typename?: 'ActivityEntry';
  /** Who did it: the account or one of its editors. */
  actor: AccountSummary;
  badge?: Maybe<ActivityCosmeticChange>;
  createdAt: Scalars['String']['output'];
  /** The editor added or removed (`EDITOR_ADDED`, `EDITOR_REMOVED`). */
  editor?: Maybe<AccountSummary>;
  emote?: Maybe<ActivityEmote>;
  fromName?: Maybe<Scalars['String']['output']>;
  /** An audit row ID; page back with it. */
  id: Scalars['String']['output'];
  kind: ActivityKind;
  paint?: Maybe<ActivityCosmeticChange>;
  set?: Maybe<ActivitySet>;
  toName?: Maybe<Scalars['String']['output']>;
};

/**
 * Narrows `accountActivityPage`. Every field is optional; an empty filter is the whole feed
 * (without editor lines, as before filters existed).
 */
export type ActivityFilter = {
  /** Only lines by this account (the owner or one of its editors). */
  actorId?: InputMaybe<Scalars['String']['input']>;
  /**
   * Only these groups. Editor lines (`EDITOR_ADDED`, `EDITOR_REMOVED`) show only when this
   * lists `EDITORS`.
   */
  groups?: InputMaybe<Array<ActivityGroup>>;
  /** Only lines at or after this time (RFC 3339). */
  since?: InputMaybe<Scalars['String']['input']>;
  /** Only lines before this time (RFC 3339). */
  until?: InputMaybe<Scalars['String']['input']>;
};

/** A group of activity lines, for filtering a feed. */
export type ActivityGroup =
  /** Badge and paint changes. */
  | 'COSMETICS'
  /** Account editors added and removed. */
  | 'EDITORS'
  /** Emotes added to, removed from or renamed in sets, and uploads that went public. */
  | 'EMOTES'
  /** Sets made or renamed, and active set changes. */
  | 'SETS';

/** What one line of an account's public activity feed says happened. */
export type ActivityKind =
  /** A channel's active set changed (`set` is the new one; `from_name`, `to_name`). */
  | 'ACTIVE_SET_CHANGED'
  /** The badge and/or paint shown changed (`badge`, `paint`). */
  | 'COSMETICS_SELECTED'
  /**
   * Someone became an account editor (`editor`). Never their permissions. Only listed when
   * the filter asks for `EDITORS`.
   */
  | 'EDITOR_ADDED'
  /**
   * An account editor was removed or left (`editor`). Only listed when the filter asks for
   * `EDITORS`.
   */
  | 'EDITOR_REMOVED'
  /** An emote of the account was approved and listed. */
  | 'EMOTE_UPLOADED'
  | 'SET_CREATED'
  | 'SET_EMOTE_ADDED'
  | 'SET_EMOTE_REMOVED'
  /** The emote's name in the set changed (`from_name`, `to_name`). */
  | 'SET_EMOTE_RENAMED'
  /** A set was renamed (`from_name`, `to_name`). */
  | 'SET_RENAMED';

/**
 * A page of an activity feed. A page can hold fewer lines than asked for without being the last
 * one: the feed skips what isn't public.
 */
export type ActivityPage = {
  __typename?: 'ActivityPage';
  entries: Array<ActivityEntry>;
  /** Pass as `before` to get the next page; null when there is none. */
  next?: Maybe<Scalars['String']['output']>;
};

/** A set, as a feed line names it. */
export type ActivitySet = {
  __typename?: 'ActivitySet';
  /** The set's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

/** Who to add as an editor: an account ID, or a Twitch or Kick login. */
export type AddAccountEditorInput = {
  /** The editor's cyex.tv account ID. Or give `platform` and `login`. */
  accountId?: InputMaybe<Scalars['String']['input']>;
  login?: InputMaybe<Scalars['String']['input']>;
  /** The account to add editors to. Empty: your own. */
  ownerId?: InputMaybe<Scalars['String']['input']>;
  /** At least one. */
  permissions: Array<EditorPermission>;
  platform?: InputMaybe<Platform>;
  /** Limit them to these sets of the account. Empty or missing: every set, new ones included. */
  setIds?: InputMaybe<Array<Scalars['String']['input']>>;
};

export type AddSetEmoteInput = {
  /** Rename it in this set. Defaults to the emote's own name. */
  name?: InputMaybe<Scalars['String']['input']>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

/** An account as staff see it (`users.view` or `users.restrict`). */
export type AdminAccount = {
  __typename?: 'AdminAccount';
  avatar?: Maybe<Scalars['String']['output']>;
  blocked: Scalars['Boolean']['output'];
  blockedUntil?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  displayName: Scalars['String']['output'];
  /** The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  identities: Array<Identity>;
  /** No reports or tickets. */
  ticketsBlocked: Scalars['Boolean']['output'];
  ticketsBlockedUntil?: Maybe<Scalars['String']['output']>;
  uploadsBlocked: Scalars['Boolean']['output'];
  /** When each hold ends; empty while it is off or for good. */
  uploadsBlockedUntil?: Maybe<Scalars['String']['output']>;
};

/** One line staff put under the header for everyone. */
export type Announcement = {
  __typename?: 'Announcement';
  /** A badge (from `cosmetics`) shown small beside the line. */
  badgeId?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  /** An emote shown small beside the line, while it is approved and public. */
  emote?: Maybe<Emote>;
  /** When staff ended it early. */
  endedAt?: Maybe<Scalars['String']['output']>;
  /** Empty: until staff end it. */
  endsAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  kind: AnnouncementKind;
  /** The link's words; empty means "Learn more". */
  linkLabel?: Maybe<Scalars['String']['output']>;
  /** An `https://` address or a path on cyex.tv (`/shop`). */
  linkUrl?: Maybe<Scalars['String']['output']>;
  /** Shown right now. */
  live: Scalars['Boolean']['output'];
  /** Empty: from when it was made. */
  startsAt?: Maybe<Scalars['String']['output']>;
  /** The line itself, 1 to 200 characters. Plain text. */
  text: Scalars['String']['output'];
  updatedAt: Scalars['String']['output'];
};

/** What `createAnnouncement` makes and `updateAnnouncement` replaces it with. */
export type AnnouncementInput = {
  /** A badge's ID. */
  badgeId?: InputMaybe<Scalars['String']['input']>;
  /** An approved emote's ID. */
  emoteId?: InputMaybe<Scalars['String']['input']>;
  /** RFC 3339, in the future and after `startsAt`. */
  endsAt?: InputMaybe<Scalars['String']['input']>;
  kind: AnnouncementKind;
  /** Up to 40 characters; only with `linkUrl`. */
  linkLabel?: InputMaybe<Scalars['String']['input']>;
  /** `https://…` or a path on cyex.tv starting with `/`, up to 500 characters. */
  linkUrl?: InputMaybe<Scalars['String']['input']>;
  /** RFC 3339. */
  startsAt?: InputMaybe<Scalars['String']['input']>;
  /** 1 to 200 characters, one line. */
  text: Scalars['String']['input'];
};

/** What an announcement is about: its icon and tone under the header. */
export type AnnouncementKind =
  /** An event or a sale, often with an emote or a badge. */
  | 'CAMPAIGN'
  /** News, a tip. */
  | 'INFO'
  /** Downtime or something not working. */
  | 'MAINTENANCE';

/** A deprecated behaviour of the API. */
export type ApiDeprecation = {
  __typename?: 'ApiDeprecation';
  /** Stable name, also what `extensions.deprecations` lists in a GraphQL answer. */
  id: Scalars['String']['output'];
  /** What to use instead. */
  replacement: Scalars['String']['output'];
  /** The day it was deprecated, `YYYY-MM-DD`. */
  since: Scalars['String']['output'];
  /** The day it stops working, `YYYY-MM-DD`. Answers that used it carry it as `Sunset`. */
  sunset: Scalars['String']['output'];
  /** What is deprecated. */
  what: Scalars['String']['output'];
};

/** One of your developer API keys. The key itself is shown once, when it is made. */
export type ApiKey = {
  __typename?: 'ApiKey';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** When it last made a request, to the minute or so. `None` until it is used. */
  lastUsedAt?: Maybe<Scalars['String']['output']>;
  /** What you called it. */
  name: Scalars['String']['output'];
  /** The start of the key (`cyex_live_` and 8 more characters), to tell keys apart. */
  prefix: Scalars['String']['output'];
  /**
   * Requests it made per UTC day, the last 30 days, oldest first; days without any are left
   * out. Written every minute or so, so the last minute may be missing.
   */
  usage: Array<ApiKeyDay>;
};

/** One day of an API key's requests. */
export type ApiKeyDay = {
  __typename?: 'ApiKeyDay';
  /** The UTC day, `YYYY-MM-DD`. */
  day: Scalars['String']['output'];
  requests: Scalars['Int']['output'];
};

/** Where the limits are published. */
export type ApiLimitsLink = {
  __typename?: 'ApiLimitsLink';
  /** The GraphQL query field. */
  graphqlField: Scalars['String']['output'];
  /** The REST route. */
  rest: Scalars['String']['output'];
};

/** `GET /v1/meta` and the GraphQL `meta` query. */
export type ApiMeta = {
  __typename?: 'ApiMeta';
  /** The API's version (semver). Within v1 it only gains fields, types and endpoints. */
  apiVersion: Scalars['String']['output'];
  /** The git commit it was built from, or `dev`. */
  build: Scalars['String']['output'];
  /** Everything deprecated right now, with the sunset date of each. */
  deprecations: Array<ApiDeprecation>;
  /** Where the limits (sizes, counts, rates) are published. */
  limits: ApiLimitsLink;
};

/** A reviewer's answer to one of the two approvals. */
export type ApprovalDecision =
  | 'APPROVE'
  | 'DENY';

/** Where one of an emote's two approvals stands (ADR 0038): its public listing or its personal use. */
export type ApprovalState =
  | 'APPROVED'
  | 'DENIED'
  /** Waiting for a reviewer. Every upload starts here. */
  | 'PENDING';

/** What `reviewApprovals` decides (ADR 0038). Leave a part out to keep it as it is. */
export type ApprovalsInput = {
  /**
   * Default `true` (what the queues send): a part that is no longer pending is skipped, and an
   * emote with nothing pending left is listed in `failed` as `not_pending`. `false` changes a
   * decision already made (the edit dialog).
   */
  onlyPending?: InputMaybe<Scalars['Boolean']['input']>;
  /** Approve or deny personal use. Needs `emotes.review_personal`. */
  personalUse?: InputMaybe<ApprovalDecision>;
  /** Approve or deny the public listing. Needs `emotes.review`. */
  publicListing?: InputMaybe<ApprovalDecision>;
  /** Why, 1 to 500 characters; the owner sees it. Needed with any `DENY`. */
  reason?: InputMaybe<Scalars['String']['input']>;
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

/** Which side of the audit log `auditTopPeople` counts. */
export type AuditPersonRole =
  /** Who made the changes. */
  | 'ACTOR'
  /** The accounts the changes were about (target kind `account`). */
  | 'TARGET';

/** A chat badge, shown before a chatter's name. */
export type Badge = {
  __typename?: 'Badge';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** Square, 18 px at 1x. */
  image: CosmeticImage;
  name: Scalars['String']['output'];
  tooltip: Scalars['String']['output'];
  updatedAt: Scalars['String']['output'];
};

/** One item a batch left alone. */
export type BatchFailure = {
  __typename?: 'BatchFailure';
  /** A machine-readable reason, as an error's `code` (`not_pending`, `not_open`, `not_found`). */
  code: Scalars['String']['output'];
  id: Scalars['String']['output'];
  message: Scalars['String']['output'];
};

/**
 * What a batch did: the IDs it changed, and the ones it left alone with why. Nothing is half
 * done: the changes commit together.
 */
export type BatchOutcome = {
  __typename?: 'BatchOutcome';
  done: Array<Scalars['String']['output']>;
  failed: Array<BatchFailure>;
};

/** The roles cyex.tv comes with (ADR 0010). */
export type BuiltInRole =
  /** "Admin": can't be deleted. */
  | 'ADMIN'
  /** The top role: every permission, held by the owners only, never granted or edited. */
  | 'ROOT'
  /** "User": every account holds it without an assignment. Starts with no permissions. */
  | 'USER';

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

/** A channel and the sets of its account you may change (`channelEditableSets`). */
export type ChannelEditableSets = {
  __typename?: 'ChannelEditableSets';
  /** The channel's active set can take your changes (it is among `sets`). */
  canEditActiveSet: Scalars['Boolean']['output'];
  channel: Channel;
  /** Your highest role on the channel; null when you have none. */
  role?: Maybe<ChannelRole>;
  /** Oldest first. Empty when you may change none. */
  sets: Array<EditableSet>;
};

/** Who you are on a channel you can manage, highest first. */
export type ChannelRole =
  /** You are an editor of the channel's account. */
  | 'ACCOUNT_EDITOR'
  /** The channel is yours. */
  | 'OWNER'
  /** You edit at least one set of the channel's account. */
  | 'SET_EDITOR';

/**
 * What `startCheckout` gives: open Paddle's checkout for this transaction (Paddle.js
 * `Paddle.Checkout.open({ transactionId })` with `clientToken`, or go to `checkoutUrl`). The
 * order turns `PAID` when Paddle's webhook says so; poll `order(id)`.
 */
export type Checkout = {
  __typename?: 'Checkout';
  /** Paddle's hosted checkout link, when the Paddle account has a default payment link. */
  checkoutUrl?: Maybe<Scalars['String']['output']>;
  /** The Paddle.js client-side token, when configured. */
  clientToken?: Maybe<Scalars['String']['output']>;
  /** `sandbox` or `production`: pass to `Paddle.Environment.set`. */
  environment: Scalars['String']['output'];
  order: Order;
  transactionId: Scalars['String']['output'];
};

/** What `cloneSet` made. */
export type ClonedSet = {
  __typename?: 'ClonedSet';
  /** Emotes copied into it. */
  copied: Scalars['Int']['output'];
  /** The new channel set. */
  set: EmoteSet;
  /**
   * Emotes left out: over capacity, private emotes of others, emotes not approved, or ones
   * you can't see.
   */
  skipped: Scalars['Int']['output'];
};

export type CodeBatch = {
  __typename?: 'CodeBatch';
  /** Cosmetics a code gives besides the item's. */
  badgeIds: Array<Scalars['String']['output']>;
  codeCount: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  createdBy?: Maybe<AccountSummary>;
  disabledAt?: Maybe<Scalars['String']['output']>;
  expiresAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  /** The item whose cosmetics a code gives, if any. */
  itemId?: Maybe<Scalars['String']['output']>;
  itemName?: Maybe<Scalars['String']['output']>;
  name: Scalars['String']['output'];
  paintIds: Array<Scalars['String']['output']>;
  /** Codes look like `PREFIX-XXXX-XXXX-XXXX-XXXX`. */
  prefix: Scalars['String']['output'];
  /** Uses so far, over every code. */
  redemptions: Scalars['Int']['output'];
  usesPerCode: Scalars['Int']['output'];
};

export type CodeBatchInput = {
  badgeIds?: Array<Scalars['String']['input']>;
  /** 1 to 1000 codes. */
  count: Scalars['Int']['input'];
  /** RFC 3339, in the future. */
  expiresAt?: InputMaybe<Scalars['String']['input']>;
  /** A shop item whose cosmetics a code gives. */
  itemId?: InputMaybe<Scalars['String']['input']>;
  /** For staff, 1 to 64 characters. */
  name: Scalars['String']['input'];
  paintIds?: Array<Scalars['String']['input']>;
  /** 1 to 12 letters or digits (upper-cased). */
  prefix: Scalars['String']['input'];
  /** 1 to 10000, default 1. Each account can use a code once. */
  usesPerCode?: InputMaybe<Scalars['Int']['input']>;
};

export type CodeBatchPage = {
  __typename?: 'CodeBatchPage';
  items: Array<CodeBatch>;
  total: Scalars['Int']['output'];
};

/**
 * One badge or paint in a public collection: what it is, since when and how it came. Never who
 * gave it, why, or what it cost.
 */
export type CollectionItem = {
  __typename?: 'CollectionItem';
  /** The badge's or paint's ID in `cosmetics`. */
  cosmeticId: Scalars['String']['output'];
  /** `BADGE` or `PAINT`. */
  kind: GrantKind;
  name: Scalars['String']['output'];
  ownedSince: Scalars['String']['output'];
  /** `STAFF`, `PURCHASE` (the shop), `CODE` or `EVENT`. */
  source: GrantSource;
};

/**
 * A personal API token (ADR 0028): a Bearer token that acts as you, with your own account's
 * rights, for a GraphQL playground, a script or moving things between your accounts. Only one
 * at a time: making a new one signs the last one out. It lasts 3 days, shows in `mySessions` as
 * `Console token`, and ends when you sign it out there, sign out any other session, or sign out
 * everywhere (ADR 0029). The token is in this answer only; it is never shown again.
 */
export type ConsoleToken = {
  __typename?: 'ConsoleToken';
  /** When it stops working: 3 days after it was made, however much it is used. */
  expiresAt: Scalars['String']['output'];
  /** Its session's ID, as `mySessions` lists it; pass to `revokeSession` to sign it out. */
  sessionId: Scalars['String']['output'];
  /** Send as `Authorization: Bearer <token>`. Anyone holding it can act as you. */
  token: Scalars['String']['output'];
  /** `Bearer`. */
  tokenType: Scalars['String']['output'];
};

/** Every badge and paint, for clients to cache (`GET /v1/cosmetics`). */
export type CosmeticCatalog = {
  __typename?: 'CosmeticCatalog';
  badges: Array<Badge>;
  paints: Array<Paint>;
};

/** A badge's or an image paint's picture, served at `{host}/{name}` like emote files. */
export type CosmeticImage = {
  __typename?: 'CosmeticImage';
  animated: Scalars['Boolean']['output'];
  createdAt: Scalars['String']['output'];
  /**
   * WebP and AVIF at 1x, 2x and 4x, and static first frames for animations. Empty until the
   * worker made them. A badge also has the size ladder (`128`, `256`, `512`) and `original`:
   * the picture at its real size (at most 512 px; a bigger upload is scaled down to 512),
   * with the real `width` and `height` in its entry.
   */
  files: Array<EmoteFile>;
  height: Scalars['Int']['output'];
  /** For example `https://cdn.cyex.tv/cosmetics/badges/<sha256>`. */
  host: Scalars['String']['output'];
  id: Scalars['String']['output'];
  kind: CosmeticKind;
  /** The files are made: only then can a badge or paint use it. */
  ready: Scalars['Boolean']['output'];
  /** The original's size in px. */
  width: Scalars['Int']['output'];
};

/** A badge or a paint. */
export type CosmeticKind =
  | 'BADGE'
  | 'PAINT';

export type CreateBadgeInput = {
  /** A ready badge image from `POST /v1/cosmetics/images`. */
  imageId: Scalars['String']['input'];
  /** 1 to 32 characters, unique. */
  name: Scalars['String']['input'];
  /** 1 to 64 characters. */
  tooltip: Scalars['String']['input'];
};

export type CreateRoleInput = {
  /** Its colour, `#rrggbb` (any case, stored lower case). Default none. */
  color?: InputMaybe<Scalars['String']['input']>;
  /** What it denies. Never also in `permissions` (`bad_permissions`). */
  denies?: InputMaybe<Array<Scalars['String']['input']>>;
  description?: InputMaybe<Scalars['String']['input']>;
  /** Never show it publicly, and leave it out wherever roles rank people. Default false. */
  invisible?: InputMaybe<Scalars['Boolean']['input']>;
  name: Scalars['String']['input'];
  permissions: Array<Scalars['String']['input']>;
  position: Scalars['Int']['input'];
  /** Show it on its holders' public profiles. Default false. */
  public?: InputMaybe<Scalars['Boolean']['input']>;
};

export type CreateSetInput = {
  /** `CHANNEL` or `PERSONAL`. The global set exists already. */
  kind: SetKind;
  name: Scalars['String']['input'];
  /** Make the set for an account you edit with `sets` (ADR 0014). Empty: for yourself. */
  ownerId?: InputMaybe<Scalars['String']['input']>;
  /**
   * Empty or `LISTED`: every set is public. `UNLISTED` and `PRIVATE` need `sets.edit_any`
   * (staff); personal sets can't be `PRIVATE`.
   */
  visibility?: InputMaybe<SetVisibility>;
};

/** A new webhook. */
export type CreateWebhookInput = {
  /**
   * At least one of: `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED`,
   * `CHANNEL_ACTIVE_SET_CHANGED`, `EMOTE_UPDATED`, `COSMETIC_CREATED`, `COSMETIC_UPDATED`,
   * `COSMETIC_DELETED`.
   */
  events: Array<EventType>;
  /** An `https://` URL on a public address. */
  url: Scalars['String']['input'];
};

/** A key just made: the key itself is in this answer only. */
export type CreatedApiKey = {
  __typename?: 'CreatedApiKey';
  apiKey: ApiKey;
  /** Send as `Authorization: Bearer <key>`. It reads public data only. */
  key: Scalars['String']['output'];
};

/** A new batch and its codes in plain text. This is the only time they are shown. */
export type CreatedCodeBatch = {
  __typename?: 'CreatedCodeBatch';
  batch: CodeBatch;
  codes: Array<Scalars['String']['output']>;
};

/** A webhook just made, or its secret just replaced: the secret is in this answer only. */
export type CreatedWebhook = {
  __typename?: 'CreatedWebhook';
  /** Signs every delivery (`X-Cyex-Signature`). Keep it on your server. */
  secret: Scalars['String']['output'];
  webhook: Webhook;
};

/** Another emote that may be the same as one in the review queue. */
export type DuplicateCandidate = {
  __typename?: 'DuplicateCandidate';
  emote: Emote;
  /** Made from the very same file (the same sha256). */
  sameFile: Scalars['Boolean']['output'];
  /** How alike the names are, 0 to 1. */
  similarity: Scalars['Float']['output'];
};

/** A channel you can manage, with your highest role on it. */
export type EditableChannel = {
  __typename?: 'EditableChannel';
  channel: Channel;
  role: ChannelRole;
};

/** A set you may add, rename and remove emotes in. */
export type EditableSet = {
  __typename?: 'EditableSet';
  access: SetAccess;
  /** The channel shows this set now (`channel.activeSetId`). */
  activeHere: Scalars['Boolean']['output'];
  /** The set, without preview emotes. `capacity - slotsUsed` is what still fits. */
  set: SetSummary;
};

/** An account you edit, with what you may do there. */
export type EditedAccount = {
  __typename?: 'EditedAccount';
  account: AccountSummary;
  /** You may work on every set of the account. False: only `setIds`. */
  allSets: Scalars['Boolean']['output'];
  grantedAt: Scalars['String']['output'];
  permissions: Array<EditorPermission>;
  setIds: Array<Scalars['String']['output']>;
};

/** What an account editor may do on the account they edit. The account's owner has all of them. */
export type EditorPermission =
  /** Manage the owner's payment methods. Stored; grants nothing yet. */
  | 'BILLING_PAYMENT'
  /** Manage the owner's subscription. Stored; grants nothing yet. */
  | 'BILLING_SUBSCRIPTION'
  /** Change the owner's channels' settings (and pick their active set, as `Sets` does). */
  | 'CHANNELS'
  /** Add, change and remove editors, never granting more than you hold. */
  | 'EDITORS'
  /** Add, remove, rename and alias emotes in the owner's sets, the active channel sets too. */
  | 'EMOTES'
  /** See, rename, retag and delete the owner's emotes. */
  | 'EMOTE_MANAGE'
  /**
   * Retired (ADR 0014, amendment 2026/11/22): only an emote's owner gives it away, and nobody
   * can ask for someone else's emote to be transferred. Kept so clients that send it still
   * parse; it is never granted, never listed and grants nothing (stored grants ignore it).
   */
  | 'EMOTE_TRANSFER'
  /** Upload emotes for the owner (`POST /v1/emotes` with `owner_id`). */
  | 'EMOTE_UPLOAD'
  /** Pick the badge and paint the owner shows (ADR 0015). Never grants. */
  | 'PROFILE'
  /** Create, rename and delete sets, change their capacity, and pick a channel's active set. */
  | 'SETS';

/** One of the signed-in caller's per-account limits, with the default and what applies to them. */
export type EffectiveLimit = {
  __typename?: 'EffectiveLimit';
  /** What everyone gets. */
  default: Scalars['Int']['output'];
  /** What the caller gets: the most generous override among their roles, else the default. */
  effective: Scalars['Int']['output'];
  /** `uploads_per_hour`, `pending_emotes`, `set_capacity_max` and so on (see `RoleLimit`). */
  key: Scalars['String']['output'];
  /** The window the count is per, in seconds; null for a cap on how many exist at once. */
  windowSeconds?: Maybe<Scalars['Int']['output']>;
};

export type Emote = {
  __typename?: 'Emote';
  animated: Scalars['Boolean']['output'];
  /** Its public listing and personal use approvals (ADR 0038). */
  approvals: EmoteApprovals;
  /** How many channels have this emote in their active set right now. */
  channelCount: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  files: Array<EmoteFile>;
  flags: EmoteFlags;
  /** For example `https://cdn.cyex.tv/emotes/<sha256>`. */
  host: Scalars['String']['output'];
  /** The emote's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  /** `READY`, `PROCESSING` or `FAILED`: whether the image is made (or could not be). */
  processing: EmoteProcessing;
  state: EmoteState;
  tags: Array<Scalars['String']['output']>;
  /**
   * `PUBLIC`, `UNLISTED` or `PRIVATE` (ADR 0020). `flags.private` says the same for
   * `PRIVATE`, for clients from before `UNLISTED`.
   */
  visibility: EmoteVisibility;
};

/** An emote's two approvals (ADR 0038). Each is decided on its own, in review. */
export type EmoteApprovals = {
  __typename?: 'EmoteApprovals';
  /**
   * Whether it may go in a personal set (shown in chat everywhere its owner chats). A personal
   * set takes only emotes whose personal use is `APPROVED`.
   */
  personalUse: ApprovalState;
  /**
   * Whether it may be listed: found by search and browse and shown on its creator's page.
   * Until it is `APPROVED` the emote is unlisted (unless private).
   */
  publicListing: ApprovalState;
};

/** One file of an emote, served at `{host}/{name}`. */
export type EmoteFile = {
  __typename?: 'EmoteFile';
  format: ImageFormat;
  height: Scalars['Int']['output'];
  /** The first frame of an animated emote. */
  isStatic: Scalars['Boolean']['output'];
  /** `image/webp` or `image/avif`. Filled in by the API like `url`. */
  mime: Scalars['String']['output'];
  /** For example `2x.webp` or `1x_static.avif`. */
  name: Scalars['String']['output'];
  /**
   * 1 to 4. A badge's larger sizes (`128`, `256`, `512`) carry their px ÷ 18, rounded: 7, 14
   * and 28; read `width` for the size. **Deprecated for badge and paint files**: `width` is
   * the contract (`file_scale_non_emote` in `/v1/meta`). Emote files keep `scale` as it is.
   * @deprecated For badge and paint files: read `width`. Emote files keep `scale` (1 to 4).
   */
  scale: Scalars['Int']['output'];
  /** Bytes. */
  size: Scalars['Int']['output'];
  /**
   * The full CDN address of this file: `{host}/{name}`. Filled in by the API (empty only in
   * the records the worker writes).
   */
  url: Scalars['String']['output'];
  width: Scalars['Int']['output'];
};

/** A flag `searchEmotes` can require. */
export type EmoteFlag =
  /** Retired: NSFW emotes aren't allowed, so this finds nothing. */
  | 'NSFW'
  /** Approved for personal use (ADR 0038): it can go in a personal set. */
  | 'PERSONAL_USE'
  /** About twice as wide as tall or more. */
  | 'WIDE'
  /** Overlays the emote before it. */
  | 'ZERO_WIDTH';

export type EmoteFlags = {
  __typename?: 'EmoteFlags';
  /**
   * Retired (ADR 0020, amendment 2026/11/22): NSFW emotes aren't allowed on cyex.tv, so this
   * is always `false`.
   * @deprecated NSFW emotes aren't allowed on cyex.tv; always false.
   */
  nsfw: Scalars['Boolean']['output'];
  /** Only its owner can add it to sets. True exactly when `visibility` is `PRIVATE`. */
  private: Scalars['Boolean']['output'];
  /**
   * About twice as wide as tall or more: the 1x file is at least 1.75 times as wide as it is
   * tall. Derived from the files, not set by the uploader.
   */
  wide: Scalars['Boolean']['output'];
  zeroWidth: Scalars['Boolean']['output'];
};

/** What `setEmoteFlags` changes (ADR 0024). Every field is optional: what is left out stays. */
export type EmoteFlagsInput = {
  /**
   * Retired: NSFW emotes aren't allowed. `false` clears an old flag (any change here clears
   * it too); `true` is `nsfw_not_allowed`.
   * @deprecated NSFW emotes aren't allowed on cyex.tv. true is refused (nsfw_not_allowed); saving flags clears an old NSFW flag.
   */
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  /** `UNLISTED` takes a public emote off search, browse and its creator's page. */
  visibility?: InputMaybe<EmoteVisibility>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

/** A note reviewers keep on one emote (`emotes.review`). The uploader never sees it. */
export type EmoteNote = {
  __typename?: 'EmoteNote';
  /** Empty when the author's account is gone. */
  authorId?: Maybe<Scalars['String']['output']>;
  body: Scalars['String']['output'];
  createdAt: Scalars['String']['output'];
  emoteId: Scalars['String']['output'];
  id: Scalars['String']['output'];
};

/** A page of emotes and how many there are in all. */
export type EmotePage = {
  __typename?: 'EmotePage';
  items: Array<Emote>;
  /**
   * Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
   * page. Opaque: a list's cursor only works for the same list, filters and order.
   */
  next?: Maybe<Scalars['String']['output']>;
  total: Scalars['Int']['output'];
};

/**
 * Whether an emote's image is made. Separate from [`EmoteState`] (review), so adding it
 * breaks nobody: a client that doesn't know it keeps showing `Processing` as before.
 */
export type EmoteProcessing =
  /** The transcode ran out of attempts: the emote never gets files. Delete it or upload again. */
  | 'FAILED'
  /** The transcode job is queued or running. */
  | 'PROCESSING'
  /** The files are made. */
  | 'READY';

export type EmoteSet = {
  __typename?: 'EmoteSet';
  capacity: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  emotes: Array<SetEmote>;
  /** The set's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  kind: SetKind;
  /** `visibility` is `LISTED`: shown on the owner's profile. */
  listed: Scalars['Boolean']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  /**
   * How many of `capacity` the set uses. For whoever may edit the set it counts every emote in
   * it, pending ones included, so `capacity - slotsUsed` is what still fits. For anyone else it
   * counts the emotes this answer may show them (approved ones, plus their own pending ones).
   */
  slotsUsed: Scalars['Int']['output'];
  /**
   * Goes up by one or more with every change to what the set shows: a member added, removed
   * or renamed, the set renamed, an emote in it changed (ADR 0035). Keep it with your copy
   * and ask `setChanges` (or `GET /v1/sets/{id}?since=`) for what changed after it.
   */
  version: Scalars['Int']['output'];
  /** Who can find and open it. */
  visibility: SetVisibility;
};

/** How `searchEmotes` orders its results. */
export type EmoteSort =
  /** The newest first. */
  | 'NEW'
  /** Best match first, then the most used. The default. */
  | 'RELEVANCE'
  /** The most used first (channels showing the emote), then by name. */
  | 'TOP'
  /** Most channels added it in the `window` (net of removals), then the most used. */
  | 'TRENDING';

export type EmoteState =
  | 'APPROVED'
  | 'PENDING'
  | 'REJECTED'
  | 'TAKEN_DOWN';

/**
 * An account editor's request to give one of the owner's emotes to another account; the emote
 * moves only when the owner accepts (ADR 0014).
 */
export type EmoteTransferRequest = {
  __typename?: 'EmoteTransferRequest';
  createdAt: Scalars['String']['output'];
  emoteId: Scalars['String']['output'];
  emoteName: Scalars['String']['output'];
  expiresAt: Scalars['String']['output'];
  /** The emote's owner, who accepts or declines. */
  from: AccountSummary;
  id: Scalars['String']['output'];
  requestedBy: AccountSummary;
  resolvedAt?: Maybe<Scalars['String']['output']>;
  state: TransferRequestState;
  to: AccountSummary;
};

/** Where an emote stands among the trending ones. */
export type EmoteTrending = {
  __typename?: 'EmoteTrending';
  /** Channels that added it in the window, net of removals. */
  channels: Scalars['Int']['output'];
  /** 1 is the most added emote in the window. */
  rank: Scalars['Int']['output'];
  /** The window the rank is in. */
  window: TrendingWindow;
};

/** Who can find an emote and add it to sets (ADR 0020). */
export type EmoteVisibility =
  /** Only its owner can add it to sets, and it is left out of listings (`flags.private`). */
  | 'PRIVATE'
  /** Found by search and browse, listed on its creator's page. New emotes start here. */
  | 'PUBLIC'
  /**
   * Anyone with its link can open it and add it to sets; search, browse, activity feeds and
   * its creator's page leave it out for everyone but its owner. Set by staff only (a
   * moderator taking an emote off the listings); creators choose public or private.
   */
  | 'UNLISTED';

/** One account owning one cosmetic. */
export type Entitlement = {
  __typename?: 'Entitlement';
  account: AccountSummary;
  cosmeticId: Scalars['String']['output'];
  grantedAt: Scalars['String']['output'];
  grantedBy?: Maybe<AccountSummary>;
  kind: CosmeticKind;
  reason?: Maybe<Scalars['String']['output']>;
  source: EntitlementSource;
};

/**
 * One badge, paint or role an account holds or held, and where it came from: one edge of the
 * entitlement graph.
 */
export type EntitlementGrant = {
  __typename?: 'EntitlementGrant';
  accountId: Scalars['String']['output'];
  /** Held now. */
  active: Scalars['Boolean']['output'];
  codeBatchId?: Maybe<Scalars['String']['output']>;
  codeBatchName?: Maybe<Scalars['String']['output']>;
  /** Empty when it isn't known (built-in roles; grants older than the audit log). */
  grantedAt?: Maybe<Scalars['String']['output']>;
  /** The staff account that gave it; empty for purchases, codes and the built-in roles. */
  grantedBy?: Maybe<Scalars['String']['output']>;
  itemId: Scalars['String']['output'];
  /** Empty when the item was deleted since. */
  itemName: Scalars['String']['output'];
  kind: GrantKind;
  orderId?: Maybe<Scalars['String']['output']>;
  orderName?: Maybe<Scalars['String']['output']>;
  /** `paid`, `refunded` and so on. */
  orderStatus?: Maybe<Scalars['String']['output']>;
  /** What staff wrote when they gave it. */
  reason?: Maybe<Scalars['String']['output']>;
  revokeReason?: Maybe<Scalars['String']['output']>;
  /** When it was taken away; empty while held. */
  revokedAt?: Maybe<Scalars['String']['output']>;
  /** Who took it away; empty for refunds. */
  revokedBy?: Maybe<Scalars['String']['output']>;
  source: GrantSource;
};

export type EntitlementPage = {
  __typename?: 'EntitlementPage';
  items: Array<Entitlement>;
  total: Scalars['Int']['output'];
};

/**
 * How an account came to own a cosmetic: given by staff, bought in the shop (ADR 0016), from an
 * event, or from a redeem code.
 */
export type EntitlementSource =
  | 'CODE'
  | 'EVENT'
  | 'GRANT'
  | 'PURCHASE';

/** One change, as delivered on both sockets. */
export type Event = {
  __typename?: 'Event';
  /** `user.cosmetics_updated`: the badge shown now; empty for none. */
  badgeId?: Maybe<Scalars['String']['output']>;
  /** `cosmetic.*`: its ID. */
  cosmeticId?: Maybe<Scalars['String']['output']>;
  /** `cosmetic.*`: which kind of cosmetic. */
  cosmeticKind?: Maybe<CosmeticKind>;
  createdAt: Scalars['String']['output'];
  emoteId?: Maybe<Scalars['String']['output']>;
  eventType: EventType;
  /** Increases with every event. Send the last one you saw to resume. */
  id: Scalars['String']['output'];
  /** The emote's name in the set, for added and renamed. */
  name?: Maybe<Scalars['String']['output']>;
  /** `user.cosmetics_updated`: the paint shown now; empty for none. */
  paintId?: Maybe<Scalars['String']['output']>;
  setId?: Maybe<Scalars['String']['output']>;
  /** The topic it was delivered for, for example `channel:twitch:123`. */
  topic: Scalars['String']['output'];
};

/** What changed. On the event WebSocket these are written `set.emote_added` and so on. */
export type EventType =
  | 'CHANNEL_ACTIVE_SET_CHANGED'
  /** A badge or paint was made (topic `cosmetics`, ADR 0015). */
  | 'COSMETIC_CREATED'
  /** Gone: stop showing it on anyone. */
  | 'COSMETIC_DELETED'
  | 'COSMETIC_UPDATED'
  | 'EMOTE_UPDATED'
  | 'PERSONAL_UPDATED'
  /** Too much was missed: refetch what you show. */
  | 'RESYNC'
  /**
   * A listed set was made (`createSet`, `cloneSet`), on its owner's `user:` topics. Unlisted
   * and private sets make no event, so their IDs stay unknown.
   */
  | 'SET_CREATED'
  | 'SET_EMOTE_ADDED'
  | 'SET_EMOTE_REMOVED'
  | 'SET_EMOTE_RENAMED'
  /** The badge or paint a user shows changed (their `user:` topics). */
  | 'USER_COSMETICS_UPDATED';

export type GrantCosmeticInput = {
  accountId: Scalars['String']['input'];
  cosmeticId: Scalars['String']['input'];
  kind: CosmeticKind;
  /** Why, for the audit log. 1 to 500 characters. */
  reason: Scalars['String']['input'];
};

/** What an entitlement is of. */
export type GrantKind =
  | 'BADGE'
  | 'PAINT'
  | 'ROLE';

/** How an account came to hold something. */
export type GrantSource =
  /** A redeemed code (`codeBatchId`). */
  | 'CODE'
  /** The User role every account holds. */
  | 'DEFAULT'
  /** An event. */
  | 'EVENT'
  /** The owners' top role, from the owner list. */
  | 'OWNER'
  /** Bought in the shop (`orderId`). */
  | 'PURCHASE'
  /** Given by staff (a cosmetic grant or a role assignment). */
  | 'STAFF';

/** A creator whose emotes you hid from your own view. `account` is null once the account is gone. */
export type HiddenCreator = {
  __typename?: 'HiddenCreator';
  account?: Maybe<AccountSummary>;
  accountId: Scalars['String']['output'];
  hiddenAt: Scalars['String']['output'];
};

/**
 * An emote you hid from your own view. `emote` is null once the emote is gone or you may no
 * longer see it; unhiding it by `emoteId` still works.
 */
export type HiddenEmote = {
  __typename?: 'HiddenEmote';
  emote?: Maybe<Emote>;
  emoteId: Scalars['String']['output'];
  hiddenAt: Scalars['String']['output'];
};

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

/** Which part of the site a limit covers, for grouping them on the role page. */
export type LimitGroup =
  /** Writes to your account: editors, cosmetics, sets, lists, settings, the shop, your data. */
  | 'ACCOUNT'
  /**
   * How big the files an account uploads may be: bytes, side and frames of emotes, badges
   * and paints.
   */
  | 'MEDIA'
  /** How often an account may call the API and search. */
  | 'REQUESTS'
  /** Uploads an hour, and how many may wait for review. */
  | 'UPLOADS';

/**
 * One login from a pasted list, and the account it names (none when it names no account, or
 * two different ones on Twitch and Kick).
 */
export type LoginMatch = {
  __typename?: 'LoginMatch';
  account?: Maybe<AccountSummary>;
  /** The login as it was asked for, trimmed. */
  login: Scalars['String']['output'];
  /** Why there is no account: `not_found`, `ambiguous` or `bad_login`. */
  problem?: Maybe<Scalars['String']['output']>;
};

/** The signed-in account. */
export type Me = {
  __typename?: 'Me';
  createdAt: Scalars['String']['output'];
  /**
   * The platform whose name and picture the account goes by (`setDisplayPlatform`); null for
   * the default, Twitch first. `identities` lists that one first.
   */
  displayPlatform?: Maybe<Platform>;
  /** The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  identities: Array<Identity>;
  /** Permission names from the account's roles (ADR 0010). */
  permissions: Array<Scalars['String']['output']>;
  /**
   * Every role the account holds, public or not, most senior first. Only you see the ones
   * that aren't public: your profile marks them hidden; others see `AccountProfile.roles`.
   */
  roles: Array<OwnRole>;
};

/**
 * What one kind of picture (emote, badge, paint) may be. Sizes are px, the file size is bytes.
 * Emote numbers are fixed rules, the same for everyone; badge and paint numbers are the
 * configured ones (a role may raise them).
 */
export type MediaLimits = {
  __typename?: 'MediaLimits';
  /**
   * How many times as wide as tall a picture may be (emotes 4, badges 1, paints 8); null for
   * no rule at upload. Emotes have no rule for a tall picture.
   */
  maxAspect?: Maybe<Scalars['Int']['output']>;
  /** The most bytes an uploaded file may be. */
  maxBytes: Scalars['Int']['output'];
  /** The longest one run of an animation may last, in ms; null when there is no cap. */
  maxDurationMs?: Maybe<Scalars['Int']['output']>;
  /**
   * The most frames a second (emotes 50; null for the others). Frames shown for under
   * 1000 / `maxFps` ms (20 ms: a 0 or 10 ms GIF delay) are shown for 100 ms, as browsers do.
   */
  maxFps?: Maybe<Scalars['Int']['output']>;
  /** The most frames an animation may have. */
  maxFrames: Scalars['Int']['output'];
  /** The tallest file the CDN serves for this kind (emotes 128 px, badges the master size). */
  maxServedSide: Scalars['Int']['output'];
  /** The longest side of an upload. */
  maxSide: Scalars['Int']['output'];
  /** Width x height x frames, all frames together: the cap on decoding work. */
  maxTotalPixels: Scalars['Int']['output'];
  /** The smallest side an upload may have (badges: both sides; paints: the height); null for none. */
  minSide?: Maybe<Scalars['Int']['output']>;
};

export type Mutation = {
  __typename?: 'Mutation';
  /**
   * Retired with transfer requests (ADR 0038): always `410 transfer_requests_retired`.
   * @deprecated Retired (ADR 0038): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote.
   */
  acceptEmoteTransfer: Emote;
  /**
   * Make someone an editor of your account, of an account you edit with `EDITORS` (ADR
   * 0014), or of anyone's with `editors.manage_any` (audited with `via`). Give their account ID, or a platform and login. You can only grant permissions you
   * hold. `setIds` limits them to some of the account's sets; leave it out for every set, new
   * ones included. At most 50 editors per account (`too_many_editors`). Audited.
   */
  addAccountEditor: AccountEditor;
  /** Add a reviewers' note to an emote (1 to 2000 characters). Needs `emotes.review`. Audited. */
  addEmoteNote: EmoteNote;
  /**
   * Make someone an editor of a set. Its owner, or their account editors with `EDITORS` and
   * `EMOTES`.
   */
  addSetEditor: Scalars['Boolean']['output'];
  /** Add an emote to a set. Personal sets take approved emotes only. */
  addSetEmote: SetEmote;
  /** Add a staff note to an account (1 to 2000 characters). Needs `users.notes`. Audited. */
  addStaffNote: StaffNote;
  /**
   * Approve a pending emote's public listing (ADR 0038; personal use waits in its own queue, or
   * use `reviewApprovals`). Needs `emotes.review`. Audited. With `visibility`: `UNLISTED`
   * denies the listing instead (the emote can still be used), `PRIVATE` / `PUBLIC` also set
   * the private flag. `idempotencyKey` (optional UUID): a retry with it within 24 hours answers what the
   * first approval did instead of `not_pending`.
   */
  approveEmote: Emote;
  /**
   * Give an account a role below your highest one that grants only what you hold. Needs
   * `roles.manage` or `roles.assign`; the User and top roles can't be given. Audited.
   */
  assignRole: Scalars['Boolean']['output'];
  /**
   * Retired with transfer requests (ADR 0038): always `410 transfer_requests_retired`.
   * @deprecated Retired (ADR 0038): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote.
   */
  cancelEmoteTransfer: EmoteTransferRequest;
  /**
   * Copy a set you can open into a new channel set: yours, or with `ownerId` one for an account
   * you edit with `SETS`. Name defaults to the source's; the copy is public (`UNLISTED` and `PRIVATE` need `sets.edit_any`). Emotes go in
   * as `addSetEmote` would put them, in order, keeping their names: up to the capacity,
   * approved ones only (your own pending ones too), never other people's private emotes.
   * 30 copies an hour. `rate_limited`, `not_found`, `forbidden`, `bad_name`.
   */
  cloneSet: ClonedSet;
  /**
   * Put an announcement under the header: one line, a kind, an optional link, emote or badge,
   * start and end. Needs `site.announce`; audited.
   */
  createAnnouncement: Announcement;
  /**
   * Make a developer API key: a Bearer token for bots and tools that reads public data at a
   * higher rate than signed-out requests. The key is in this answer only. At most 10 per
   * account (`too_many_api_keys`); made on cyex.tv only (`website_only` with a token).
   */
  createApiKey: CreatedApiKey;
  /**
   * Make a badge from a ready badge picture (`POST /v1/cosmetics/images`). Needs
   * `cosmetics.manage`. Audited; publishes `cosmetic.created`.
   */
  createBadge: Badge;
  /**
   * Make a batch of 1 to 1000 redeem codes. The answer holds the codes in plain text; this is
   * the only time they are shown. Needs `codes.manage`. Audited.
   */
  createCodeBatch: CreatedCodeBatch;
  /**
   * Make a personal API token (a console token, ADR 0028): a Bearer token that acts as you,
   * for a GraphQL playground or a script. It has your account's own rights (your roles count;
   * owner-only and website-only actions don't work with it), lasts 3 days and shows in
   * `mySessions` as `Console token`. Signing out any session, or everywhere, ends it
   * (ADR 0029). Only one at a time: making one signs the last one out.
   * The token is in this answer only. Website only (`website_only` with a Bearer token). 5 an
   * hour (`rate_limited`). Audited. Same as `POST /v1/auth/token/export`.
   */
  createConsoleToken: ConsoleToken;
  /** Make a paint. Needs `cosmetics.manage`. Audited; publishes `cosmetic.created`. */
  createPaint: Paint;
  /** Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited. */
  createRole: Role;
  /**
   * Create a channel set, or your personal set (one per account). With `ownerId`, for an
   * account you edit with `SETS`.
   */
  createSet: EmoteSet;
  /** Make a shop item: a bundle of 1 to 10 badges and paints. Needs `shop.manage`. Audited. */
  createShopItem: ShopItem;
  /**
   * Point a channel's events at an https URL. The secret that signs every delivery is in this
   * answer only. At most 5 per channel (`too_many_webhooks`).
   */
  createWebhook: CreatedWebhook;
  /**
   * Retired with transfer requests (ADR 0038): always `410 transfer_requests_retired`.
   * @deprecated Retired (ADR 0038): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote.
   */
  declineEmoteTransfer: EmoteTransferRequest;
  /**
   * Delete your account. `confirm` is one of your Twitch or Kick logins. Website only
   * (`website_only` with an app token); the owner's account can't be (`owner_protected`).
   * The account is hidden at once (profile, sets, emotes and channels read as gone, every
   * session ends, the cookie is cleared) and erased for good 30 days later: then your
   * identities, channels, sets, editor grants, cosmetics and notifications go, your approved
   * emotes that are in other people's sets stay, shown as by a deleted user, and your other
   * emotes are deleted. Signing in again before then offers to undo it (`pendingDeletion`,
   * `restoreAccount`); after it, signing in makes a new account. Audited.
   * `confirm_mismatch` when `confirm` isn't one of your logins; `restricted` while the account's
   * uploads or sign-in are blocked. 3 tries an hour (`rate_limited`).
   */
  deleteAccount: Scalars['Boolean']['output'];
  /**
   * Delete a badge: everyone loses it and stops showing it. False when there was none. Needs
   * `cosmetics.manage`. Audited.
   */
  deleteBadge: Scalars['Boolean']['output'];
  /**
   * Delete your emote, an emote of an account you edit with `EMOTE_MANAGE`, or anyone's with
   * `emotes.delete_any` (audited). It is removed from every set.
   */
  deleteEmote: Scalars['Boolean']['output'];
  /** Delete an emote note: its author may, and the owner. False if it was gone. Audited. */
  deleteEmoteNote: Scalars['Boolean']['output'];
  /**
   * Delete a paint: everyone loses it and stops showing it. False when there was none. Needs
   * `cosmetics.manage`. Audited.
   */
  deletePaint: Scalars['Boolean']['output'];
  /**
   * Delete a role below your own highest one that you don't hold; built-in roles stay.
   * Needs `roles.manage`. Audited.
   */
  deleteRole: Scalars['Boolean']['output'];
  /**
   * Delete a set. Its owner, or their account editors with `SETS`. A channel's active set
   * can't be deleted.
   */
  deleteSet: Scalars['Boolean']['output'];
  /**
   * Delete a shop item: never listed or sold again; its orders stay. False when there was
   * none. Needs `shop.manage`. Audited.
   */
  deleteShopItem: Scalars['Boolean']['output'];
  /** Delete a staff note: its author may, and the owner. False if it was gone. Audited. */
  deleteStaffNote: Scalars['Boolean']['output'];
  /** Delete a webhook and its delivery log. */
  deleteWebhook: Scalars['Boolean']['output'];
  /** Turn off every code of a batch. Needs `codes.manage`. Audited. */
  disableCodeBatch: CodeBatch;
  /**
   * Dismiss up to 100 open reports with one optional note, in one step, one audit row each.
   * Reports already resolved are listed in `failed`. Takedowns stay one at a time
   * (`resolveReport`). Needs `emotes.review`, `emotes.takedown` or `reports.manage`.
   */
  dismissReports: BatchOutcome;
  /** End an announcement now. Needs `site.announce`; audited. */
  endAnnouncement: Announcement;
  /**
   * A copy of everything about your account, as JSON text (`format`, `version`, `account`,
   * `identities`, `sessions`, `channels`, `emotes`, `sets`, `set_editors`, `account_editors`,
   * `editor_of`, `cosmetics`, `roles`, `reports_filed`, `notifications`, `orders`,
   * `hidden_emotes`, `hidden_creators`, `favourite_emotes`). Website
   * only (`website_only` with an app token). 5 an hour (`rate_limited`). Audited.
   */
  exportMyData: Scalars['String']['output'];
  /**
   * Add an emote to your favourites. True when newly added. `not_found` for an emote you can't
   * see, `limit_reached` at 1000 favourites.
   */
  favouriteEmote: Scalars['Boolean']['output'];
  /** Give an account a badge or paint, with a reason. Needs `cosmetics.grant`. Audited. */
  grantCosmetic: Entitlement;
  /**
   * Hide every emote by a creator from your own view. True when newly hidden. `not_found` for
   * an unknown account, `own_account` for yourself, `limit_reached` at 200 creators.
   */
  hideCreator: Scalars['Boolean']['output'];
  /**
   * Hide an emote from your own view. True when newly hidden, false when it was already.
   * `not_found` for an emote you can't see, `limit_reached` at 1000 hidden emotes.
   */
  hideEmote: Scalars['Boolean']['output'];
  /**
   * Lift a restriction early, with an optional reason. Needs the same permission as placing
   * it. Audited.
   */
  liftRestriction: AccountStatus;
  /**
   * Mark your notifications read: those in `ids` (up to 100; others' IDs are ignored), or all
   * of them without `ids`. Returns how many were unread.
   */
  markNotificationsRead: Scalars['Int']['output'];
  /**
   * Restrict an account with a reason, for good or `until` a time: no uploads
   * (`users.restrict_uploads`), no reports or tickets (`users.restrict_tickets`), or a full
   * block that ends its sessions (`users.block`); `users.restrict` does all three. A running
   * restriction of the same kind is replaced. The owner, and accounts whose highest role is at
   * or above yours, are protected. Audited.
   */
  placeRestriction: AccountStatus;
  /**
   * Redeem a code (case, spaces and dashes don't matter). Wrong codes lock you out for a
   * while after a few tries (`locked_out`). Each cosmetic it gives sends a `cosmetic.granted`
   * notification (`source` `code`).
   *
   * `idempotencyKey` (optional UUID, one per attempt, sent again on a retry): a repeat within
   * 24 hours answers what the first redeem gave instead of `already_redeemed`.
   * `idempotency_key_reused` when the key went with another code.
   */
  redeemCode: RedeemResult;
  /**
   * Refund a paid order: asks Paddle for a full refund when it is set up, and takes the
   * cosmetics back at once (only those no other paid order, code or staff grant covers).
   * `already_refunded` when Paddle says it was refunded already; `refund_in_progress` while
   * another refund of it runs (it may be tried again after 10 minutes); `refund_unavailable`
   * when the order has a Paddle transaction and Paddle isn't set up. Needs `shop.orders`.
   * Audited.
   */
  refundOrder: Order;
  /** Reject a pending emote with a reason. Needs `emotes.review`. Audited. */
  rejectEmote: Emote;
  /**
   * Make a badge or paint picture's files again from its stored original: a badge made
   * before the 128, 256 and 512 px sizes gets them. Its files stay in use until the new set
   * is made; read `cosmeticImage(id)` to see it. `original_missing` when the original is
   * gone (upload it again). Needs `cosmetics.manage`. Audited.
   */
  remakeCosmeticImage: CosmeticImage;
  /**
   * Remove an account editor: the owner, an editor with `EDITORS`, `editors.manage_any`, or the editor themselves
   * (with `ownerId` set to the account they leave). False when they weren't one. Audited.
   */
  removeAccountEditor: Scalars['Boolean']['output'];
  /** Remove an editor. Owner, or the editor themselves. */
  removeSetEditor: Scalars['Boolean']['output'];
  /** Remove an emote from a set. */
  removeSetEmote: Scalars['Boolean']['output'];
  /** Report an emote you can see, with a reason. At most 10 an hour, one open report per emote. */
  reportEmote: Report;
  /**
   * Send up to 100 pending emotes to the end of the review queue for every reviewer (an
   * approval taken back within its undo time comes back last, ADR 0024). Emotes that aren't
   * pending are listed in `failed`. Needs `emotes.review`. Audited (`emote.requeued`).
   */
  requeueEmotes: BatchOutcome;
  /**
   * Dismiss a report, or take its emote down (`emotes.takedown`). Needs `emotes.review` or
   * `emotes.takedown`. Audited.
   */
  resolveReport: Report;
  /**
   * Undo `deleteAccount` within 30 days, signed in with the cookie that signing in to the
   * deleted account gave (the website's restore page). Everything shows again and the session
   * works as before. `signed_out` without such a session; `nothing_to_restore` when the window
   * has passed. Audited.
   */
  restoreAccount: Scalars['Boolean']['output'];
  /**
   * Block an account's uploads, or the whole account, for good and without a reason (see
   * `placeRestriction` for timed ones). Needs `users.restrict`, or `users.restrict_uploads` /
   * `users.block` for the one you change; the owner is protected. Audited.
   */
  restrictAccount: AccountStatus;
  /**
   * Decide the public listing, personal use or both of up to 100 emotes (ADR 0038): the main
   * queue's Approve both, each part on its own, the personal queue and the edit dialog. The
   * listing needs `emotes.review`, personal use `emotes.review_personal`; a `DENY` needs a
   * `reason` (`bad_approvals`). With `onlyPending` (default true) a part that was already
   * decided is skipped and an emote with nothing pending left is listed in `failed`
   * (`not_pending`). All of it commits together, with one audit row (`emote.approved` for the
   * first listing verdict, else `emote.approvals_changed`) and one notice per emote.
   * `idempotencyKey` (optional UUID): a retry with it within 24 hours answers the first outcome.
   */
  reviewApprovals: BatchOutcome;
  /**
   * Approve (optionally as `visibility`) or reject (with `reason`) up to 100 pending emotes in
   * one step. All of it commits together, with one audit row and one notice per emote; emotes
   * that aren't pending are left alone and listed in `failed`. Needs `emotes.review`.
   * `idempotencyKey` (optional UUID): a retry with it within 24 hours answers the first
   * outcome instead of listing every emote as `not_pending`.
   */
  reviewEmotes: BatchOutcome;
  /** Revoke one of your API keys; it stops working at once. False when there was no such key. */
  revokeApiKey: Scalars['Boolean']['output'];
  /**
   * Take a badge or paint away; if they showed it, it stops. False when they didn't own it.
   * Needs `cosmetics.grant`. Audited.
   */
  revokeCosmetic: Scalars['Boolean']['output'];
  /**
   * Sign out everywhere but here. Returns how many sessions ended. Website only
   * (`website_only` with an app token).
   */
  revokeOtherSessions: Scalars['Int']['output'];
  /**
   * Sign out one of your sessions, the current one too (then the cookie is cleared). False
   * when you have none with that ID.
   */
  revokeSession: Scalars['Boolean']['output'];
  /** Replace a webhook's secret. The new one is in this answer only; the old one stops at once. */
  rotateWebhookSecret: CreatedWebhook;
  /**
   * Pick the badge and paint you show, from what you own (ADR 0015). Leave a field out to keep
   * it; send null or an empty string to show none. With `ownerId`, for an account you edit
   * with `PROFILE`.
   */
  selectCosmetics: OwnedCosmetics;
  /** Show (or hide) the badges and paints you own on your public profile. Off by default. */
  setCollectionPublic: AccountCollection;
  /**
   * Go by the name and picture of your identity on `platform` (when more than one is linked,
   * pick which login you show as). Every linked login still finds you in search.
   * `not_found` when that platform isn't linked. Unlinking it goes back to the default.
   */
  setDisplayPlatform: Me;
  /**
   * Change an emote's flags after review: visibility (`UNLISTED` takes it off the listings)
   * and zero-width. `nsfw: true` is `nsfw_not_allowed` (NSFW emotes aren't allowed); saving
   * clears an old NSFW flag. Needs `emotes.flags` or `emotes.edit_any`. Audited
   * (`emote.flags_changed`); the owner is told when the visibility changes (ADR 0024).
   */
  setEmoteFlags: Emote;
  /**
   * Turn notification kinds on or off (`notify`: off, the server doesn't create them at all)
   * and choose which pop up (`toast`). Kinds left out keep their setting. `unknown_kind` for a
   * kind that doesn't exist, `required_kind` for turning off one that must stay on.
   */
  setNotificationPrefs: Array<NotificationPref>;
  /**
   * Replace the review queue's ready reject reasons (0 to 12, each 1 to 200 characters, in
   * order). Needs `review.settings`. Audited.
   */
  setReviewReasons: Array<Scalars['String']['output']>;
  /**
   * Set a role's limits: the overrides listed (a null value or a limit left out keeps the
   * default), for everyone who holds it. Only on a role you may edit, not the User or top
   * role. Needs `roles.manage`. Audited.
   */
  setRoleLimits: Array<RoleLimit>;
  /**
   * Sign out everywhere, here too: every site and app session ends and the cookie is cleared.
   * Returns how many sessions ended. With an app token, only that app session ends.
   */
  signOutEverywhere: Scalars['Int']['output'];
  /**
   * Start buying an item for yourself, or as a gift with `recipient`. Answers the Paddle
   * transaction to open in Paddle's checkout; the order turns `PAID` once Paddle confirms.
   * `checkout_unavailable` while Paddle isn't set up or the item has no Paddle price. Asking
   * again for the same item and recipient within the hour hands out the same transaction
   * (one pending order each); `checkout_busy` while another request is starting it.
   *
   * `idempotencyKey` (optional, a UUID the client makes once per purchase and sends again on
   * a retry): a repeat within 24 hours answers what the first one did (as it was then; poll
   * `order(id)` for where it stands) instead of starting another order.
   * `idempotency_key_reused` when the key went with another item or recipient,
   * `idempotency_in_progress` while the first is still running. A request that failed gives
   * its key back.
   */
  startCheckout: Checkout;
  /**
   * Take an emote's image down everywhere: out of every set, its file blocked and deleted,
   * its owners told. Needs `emotes.takedown`. Audited.
   */
  takedownEmote: Emote;
  /** Send a `webhook.test` delivery now (once, no retries), even while the webhook is off. */
  testWebhook: WebhookDelivery;
  /**
   * Move an emote to another account at once (ADR 0038, `emote.transferred`). Needs
   * `emotes.transfer`: anyone else, the owner included, gets `403 transfer_staff_only`.
   * The receiver must be a live account (not deleted, blocked or restricted). Both the old and
   * the new owner are notified. 60 an hour.
   */
  transferEmote: Emote;
  /**
   * Take a role from an account. The owner and the last `roles.manage` holder are protected.
   * Needs `roles.manage` or `roles.assign`. Audited.
   */
  unassignRole: Scalars['Boolean']['output'];
  /** Take an emote off your favourites. False when it wasn't one. */
  unfavouriteEmote: Scalars['Boolean']['output'];
  /** Show a creator's emotes again. False when they weren't hidden. */
  unhideCreator: Scalars['Boolean']['output'];
  /** Show an emote you hid again. False when it wasn't hidden. */
  unhideEmote: Scalars['Boolean']['output'];
  /**
   * Unlink a platform from your account. The last identity can't be removed, and a
   * restricted or blocked account can't unlink (`restricted`).
   */
  unlinkIdentity: Me;
  /**
   * Change an account editor's permissions. `ownerId` is the account edited; empty for yours.
   * The owner, an editor with `EDITORS`, or `editors.manage_any`.
   * `setIds` changes which sets they work on: empty for every set, left out to keep them.
   * Audited.
   */
  updateAccountEditor: AccountEditor;
  /** Replace every field of an announcement that hasn't ended. Needs `site.announce`; audited. */
  updateAnnouncement: Announcement;
  /** Change a badge's name, tooltip or picture. Needs `cosmetics.manage`. Audited. */
  updateBadge: Badge;
  /**
   * Change a channel's active set and settings. The channel's owner, or their account editors
   * with `SETS`.
   */
  updateChannelSettings: Channel;
  /**
   * Change an emote's name, tags or flags. Owner, an account editor of the owner with
   * `EMOTE_MANAGE` on every set (audited), or `emotes.edit_any`.
   */
  updateEmote: Emote;
  /** Replace every field of a paint. Needs `cosmetics.manage`. Audited. */
  updatePaint: Paint;
  /**
   * Change a role below your own highest one that you don't hold. The top role can't be
   * changed; the User role keeps position 0. Needs `roles.manage`. Audited.
   */
  updateRole: Role;
  /**
   * Rename a set or change its capacity (owner, editors, account editors with `SETS`). Capacity
   * is 1 to the owner's `set_capacity_max` role limit (1.000 by default; `capacity_over_limit`
   * above it), up to 1.000.000 with `sets.edit_any`, and never below the emotes the set holds
   * (`capacity_too_low`); see `setCapacityRange`. Every set is public: `visibility` other than
   * what it is needs `sets.edit_any` (`bad_visibility`).
   */
  updateSet: EmoteSet;
  /** Rename an emote in a set, or change its zero-width flag there. */
  updateSetEmote: SetEmote;
  /**
   * Replace every field of a shop item. Orders made already keep their price. Needs
   * `shop.manage`. Audited.
   */
  updateShopItem: ShopItem;
  /** Change a webhook's URL or events, or turn it off or on again. */
  updateWebhook: Webhook;
};


export type MutationAcceptEmoteTransferArgs = {
  id: Scalars['String']['input'];
};


export type MutationAddAccountEditorArgs = {
  input: AddAccountEditorInput;
};


export type MutationAddEmoteNoteArgs = {
  body: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
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


export type MutationAddStaffNoteArgs = {
  accountId: Scalars['String']['input'];
  body: Scalars['String']['input'];
};


export type MutationApproveEmoteArgs = {
  id: Scalars['String']['input'];
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
  visibility?: InputMaybe<EmoteVisibility>;
};


export type MutationAssignRoleArgs = {
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
};


export type MutationCancelEmoteTransferArgs = {
  id: Scalars['String']['input'];
};


export type MutationCloneSetArgs = {
  id: Scalars['String']['input'];
  name?: InputMaybe<Scalars['String']['input']>;
  ownerId?: InputMaybe<Scalars['String']['input']>;
  visibility?: InputMaybe<SetVisibility>;
};


export type MutationCreateAnnouncementArgs = {
  input: AnnouncementInput;
};


export type MutationCreateApiKeyArgs = {
  name: Scalars['String']['input'];
};


export type MutationCreateBadgeArgs = {
  input: CreateBadgeInput;
};


export type MutationCreateCodeBatchArgs = {
  input: CodeBatchInput;
};


export type MutationCreatePaintArgs = {
  input: PaintInput;
};


export type MutationCreateRoleArgs = {
  input: CreateRoleInput;
};


export type MutationCreateSetArgs = {
  input: CreateSetInput;
};


export type MutationCreateShopItemArgs = {
  input: ShopItemInput;
};


export type MutationCreateWebhookArgs = {
  id: Scalars['String']['input'];
  input: CreateWebhookInput;
  platform: Platform;
};


export type MutationDeclineEmoteTransferArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteAccountArgs = {
  confirm: Scalars['String']['input'];
};


export type MutationDeleteBadgeArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteEmoteNoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeletePaintArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteRoleArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteSetArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteShopItemArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteStaffNoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteWebhookArgs = {
  id: Scalars['String']['input'];
};


export type MutationDisableCodeBatchArgs = {
  id: Scalars['String']['input'];
};


export type MutationDismissReportsArgs = {
  ids: Array<Scalars['String']['input']>;
  note?: InputMaybe<Scalars['String']['input']>;
};


export type MutationEndAnnouncementArgs = {
  id: Scalars['String']['input'];
};


export type MutationFavouriteEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationGrantCosmeticArgs = {
  input: GrantCosmeticInput;
};


export type MutationHideCreatorArgs = {
  accountId: Scalars['String']['input'];
};


export type MutationHideEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationLiftRestrictionArgs = {
  accountId: Scalars['String']['input'];
  kind: RestrictionKind;
  reason?: InputMaybe<Scalars['String']['input']>;
};


export type MutationMarkNotificationsReadArgs = {
  ids?: InputMaybe<Array<Scalars['String']['input']>>;
};


export type MutationPlaceRestrictionArgs = {
  accountId: Scalars['String']['input'];
  input: RestrictionInput;
};


export type MutationRedeemCodeArgs = {
  code: Scalars['String']['input'];
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
};


export type MutationRefundOrderArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationRejectEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationRemakeCosmeticImageArgs = {
  id: Scalars['String']['input'];
};


export type MutationRemoveAccountEditorArgs = {
  accountId: Scalars['String']['input'];
  ownerId?: InputMaybe<Scalars['String']['input']>;
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


export type MutationRequeueEmotesArgs = {
  ids: Array<Scalars['String']['input']>;
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


export type MutationReviewApprovalsArgs = {
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
  ids: Array<Scalars['String']['input']>;
  input: ApprovalsInput;
};


export type MutationReviewEmotesArgs = {
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
  ids: Array<Scalars['String']['input']>;
  reason?: InputMaybe<Scalars['String']['input']>;
  verdict: ReviewVerdict;
  visibility?: InputMaybe<EmoteVisibility>;
};


export type MutationRevokeApiKeyArgs = {
  id: Scalars['String']['input'];
};


export type MutationRevokeCosmeticArgs = {
  accountId: Scalars['String']['input'];
  cosmeticId: Scalars['String']['input'];
  kind: CosmeticKind;
  reason?: InputMaybe<Scalars['String']['input']>;
};


export type MutationRevokeSessionArgs = {
  id: Scalars['String']['input'];
};


export type MutationRotateWebhookSecretArgs = {
  id: Scalars['String']['input'];
};


export type MutationSelectCosmeticsArgs = {
  badgeId?: InputMaybe<Scalars['String']['input']>;
  ownerId?: InputMaybe<Scalars['String']['input']>;
  paintId?: InputMaybe<Scalars['String']['input']>;
};


export type MutationSetCollectionPublicArgs = {
  public: Scalars['Boolean']['input'];
};


export type MutationSetDisplayPlatformArgs = {
  platform: Platform;
};


export type MutationSetEmoteFlagsArgs = {
  id: Scalars['String']['input'];
  input: EmoteFlagsInput;
};


export type MutationSetNotificationPrefsArgs = {
  prefs: Array<NotificationPrefInput>;
};


export type MutationSetReviewReasonsArgs = {
  reasons: Array<Scalars['String']['input']>;
};


export type MutationSetRoleLimitsArgs = {
  id: Scalars['String']['input'];
  limits: Array<RoleLimitInput>;
};


export type MutationStartCheckoutArgs = {
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
  itemId: Scalars['String']['input'];
  recipient?: InputMaybe<RecipientInput>;
};


export type MutationTakedownEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationTestWebhookArgs = {
  id: Scalars['String']['input'];
};


export type MutationTransferEmoteArgs = {
  id: Scalars['String']['input'];
  toAccountId: Scalars['String']['input'];
};


export type MutationUnassignRoleArgs = {
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
};


export type MutationUnfavouriteEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationUnhideCreatorArgs = {
  accountId: Scalars['String']['input'];
};


export type MutationUnhideEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationUnlinkIdentityArgs = {
  platform: Platform;
};


export type MutationUpdateAccountEditorArgs = {
  accountId: Scalars['String']['input'];
  ownerId?: InputMaybe<Scalars['String']['input']>;
  permissions: Array<EditorPermission>;
  setIds?: InputMaybe<Array<Scalars['String']['input']>>;
};


export type MutationUpdateAnnouncementArgs = {
  id: Scalars['String']['input'];
  input: AnnouncementInput;
};


export type MutationUpdateBadgeArgs = {
  id: Scalars['String']['input'];
  input: UpdateBadgeInput;
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


export type MutationUpdatePaintArgs = {
  id: Scalars['String']['input'];
  input: PaintInput;
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


export type MutationUpdateShopItemArgs = {
  id: Scalars['String']['input'];
  input: ShopItemInput;
};


export type MutationUpdateWebhookArgs = {
  id: Scalars['String']['input'];
  input: UpdateWebhookInput;
};

/** The lengths of the names and notes clients type. */
export type NameLimits = {
  __typename?: 'NameLimits';
  badge: TextLimit;
  /** An emote's name in a set (no spaces). */
  emote: TextLimit;
  paint: TextLimit;
  set: TextLimit;
  /** A badge's tooltip. */
  tooltip: TextLimit;
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
  /**
   * For `emote.transfer_requested`: where the request stands now (it may have been answered,
   * withdrawn or run out since). Null for other kinds.
   */
  transferState?: Maybe<TransferRequestState>;
};

/** One kind of notification and what the account chose for it. */
export type NotificationPref = {
  __typename?: 'NotificationPref';
  /** For example `emote.approved` (the same as `Notification.kind`). */
  kind: Scalars['String']['output'];
  /** Created at all. Off: nothing is stored for this kind. */
  notify: Scalars['Boolean']['output'];
  /** Can't be turned off (a hand-over waiting for your answer, a takedown). */
  required: Scalars['Boolean']['output'];
  /** Pops up on the website when it arrives. */
  toast: Scalars['Boolean']['output'];
};

export type NotificationPrefInput = {
  kind: Scalars['String']['input'];
  notify: Scalars['Boolean']['input'];
  toast: Scalars['Boolean']['input'];
};

export type Order = {
  __typename?: 'Order';
  /**
   * The item's price when the checkout started, in cents. 0 when a gift's recipient (not its
   * buyer) looks at it.
   */
  amountCents: Scalars['Int']['output'];
  /** Who paid. None once their account is gone. */
  buyer?: Maybe<AccountSummary>;
  createdAt: Scalars['String']['output'];
  currency: Scalars['String']['output'];
  failedAt?: Maybe<Scalars['String']['output']>;
  /** Bought for another account (decided at checkout; stays when either account is deleted). */
  gift: Scalars['Boolean']['output'];
  id: Scalars['String']['output'];
  itemId: Scalars['String']['output'];
  /** The item's name when it was bought. */
  itemName: Scalars['String']['output'];
  paidAt?: Maybe<Scalars['String']['output']>;
  paidCurrency?: Maybe<Scalars['String']['output']>;
  /**
   * What Paddle says was paid, tax included, in the lowest unit. Set once paid; null for a
   * gift's recipient, like `paidCurrency` and `providerTransactionId`.
   */
  paidTotalCents?: Maybe<Scalars['Int']['output']>;
  /** `paddle`. */
  provider: Scalars['String']['output'];
  /** Paddle's transaction ID (`txn_…`). */
  providerTransactionId?: Maybe<Scalars['String']['output']>;
  /** Who got the cosmetics. */
  recipient?: Maybe<AccountSummary>;
  refundReason?: Maybe<Scalars['String']['output']>;
  /**
   * When Paddle rejected this order's refund after the cosmetics were taken back (RFC 3339):
   * look at it by hand. Only for holders of `shop.orders`.
   */
  refundRejectedAt?: Maybe<Scalars['String']['output']>;
  refundedAt?: Maybe<Scalars['String']['output']>;
  status: OrderStatus;
};

export type OrderPage = {
  __typename?: 'OrderPage';
  items: Array<Order>;
  total: Scalars['Int']['output'];
};

export type OrderStatus =
  /**
   * Cancelled at Paddle, not started there, or not paid within a day (a late payment is still
   * taken).
   */
  | 'FAILED'
  /** Paid; the recipient owns the cosmetics. Also while a staff refund is being sent to Paddle. */
  | 'PAID'
  /** Checkout started, not paid yet. */
  | 'PENDING'
  /** Refunded or charged back; the cosmetics were taken back. */
  | 'REFUNDED';

/** One of your own roles, as `me.roles` lists it. */
export type OwnRole = {
  __typename?: 'OwnRole';
  /** The role's colour, `#rrggbb`, if it has one. */
  color?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  /** Never shown publicly, whatever `public` says (only you and staff see it). */
  invisible: Scalars['Boolean']['output'];
  name: Scalars['String']['output'];
  /** Shown on your public profile. When false, only you see it. */
  public: Scalars['Boolean']['output'];
};

/** An account's cosmetics: what it owns and what it shows. */
export type OwnedCosmetics = {
  __typename?: 'OwnedCosmetics';
  account: AccountSummary;
  /** The badge shown, one of `badges`. */
  badgeId?: Maybe<Scalars['String']['output']>;
  badges: Array<Badge>;
  /** The paint shown, one of `paints`. */
  paintId?: Maybe<Scalars['String']['output']>;
  paints: Array<Paint>;
};

/** A name paint: a gradient or a picture drawn into the chatter's name. */
export type Paint = {
  __typename?: 'Paint';
  /** Degrees, 0 to 360, CSS convention (0 points up, 90 to the right). Linear only. */
  angle: Scalars['Int']['output'];
  /** Shown where a client can't draw the paint. */
  color?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** Image paints only. 64 px tall at 1x. */
  image?: Maybe<CosmeticImage>;
  kind: PaintKind;
  name: Scalars['String']['output'];
  /** `repeating-*-gradient`, or a tiled image. */
  repeat: Scalars['Boolean']['output'];
  /** At most 4. */
  shadows: Array<PaintShadow>;
  /** Radial only. */
  shape: PaintShape;
  /** 2 to 10 for gradients; none for image paints. */
  stops: Array<PaintStop>;
  updatedAt: Scalars['String']['output'];
};

/** A whole paint: `createPaint` makes one, `updatePaint` replaces every field. */
export type PaintInput = {
  /** 0 to 360, default 90. */
  angle?: InputMaybe<Scalars['Int']['input']>;
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color?: InputMaybe<Scalars['String']['input']>;
  /** A ready paint image; `IMAGE` paints only. */
  imageId?: InputMaybe<Scalars['String']['input']>;
  kind: PaintKind;
  /** 1 to 32 characters, unique. */
  name: Scalars['String']['input'];
  /** Default false. */
  repeat?: InputMaybe<Scalars['Boolean']['input']>;
  /** At most 4. */
  shadows?: Array<PaintShadowInput>;
  /** Default `CIRCLE`. */
  shape?: InputMaybe<PaintShape>;
  /** 2 to 10 for gradients, none for `IMAGE`. */
  stops?: Array<PaintStopInput>;
};

/** How a paint is drawn. */
export type PaintKind =
  /** The paint's image, drawn into the name's text. */
  | 'IMAGE'
  /** `linear-gradient(<angle>deg, <stops>)`. */
  | 'LINEAR_GRADIENT'
  /** `radial-gradient(<shape>, <stops>)`. */
  | 'RADIAL_GRADIENT';

/** A drop shadow under a painted name (CSS `drop-shadow(x y radius color)`, in px). */
export type PaintShadow = {
  __typename?: 'PaintShadow';
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['output'];
  /** Blur radius, 0 to 16. */
  radius: Scalars['Float']['output'];
  /** −16 to 16. */
  x: Scalars['Float']['output'];
  /** −16 to 16. */
  y: Scalars['Float']['output'];
};

/** A drop shadow under a painted name (CSS `drop-shadow(x y radius color)`, in px). */
export type PaintShadowInput = {
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['input'];
  /** Blur radius, 0 to 16. */
  radius: Scalars['Float']['input'];
  /** −16 to 16. */
  x: Scalars['Float']['input'];
  /** −16 to 16. */
  y: Scalars['Float']['input'];
};

/** The shape of a radial paint. */
export type PaintShape =
  | 'CIRCLE'
  | 'ELLIPSE';

/** One colour stop of a gradient paint. */
export type PaintStop = {
  __typename?: 'PaintStop';
  /** Where along the gradient, 0 to 1. Each stop is at or after the one before. */
  at: Scalars['Float']['output'];
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['output'];
};

/** One colour stop of a gradient paint. */
export type PaintStopInput = {
  /** Where along the gradient, 0 to 1. Each stop is at or after the one before. */
  at: Scalars['Float']['input'];
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['input'];
};

/** A deleted account that can still be restored (ADR 0019), for the page signing in to it opens. */
export type PendingDeletion = {
  __typename?: 'PendingDeletion';
  avatar?: Maybe<Scalars['String']['output']>;
  /** The name and avatar it showed, Twitch first. */
  displayName?: Maybe<Scalars['String']['output']>;
  /** When it will be erased for good, unless restored first. */
  eraseAfter: Scalars['String']['output'];
  /** When `deleteAccount` was called. */
  requestedAt: Scalars['String']['output'];
};

/**
 * Whether an account holds one permission, and which role decided it (`accountPermissions`).
 * Its most senior role that allows or denies the permission decides; the User role comes last.
 */
export type PermissionDecision = {
  __typename?: 'PermissionDecision';
  /** What the deciding role says; null when no role names it (not granted). */
  effect?: Maybe<PermissionEffect>;
  granted: Scalars['Boolean']['output'];
  /** Granted because the account is an owner, who holds every permission and can't be denied. */
  owner: Scalars['Boolean']['output'];
  permission: Scalars['String']['output'];
  /** The deciding role; null when none decided, and for an owner unless you are one. */
  roleId?: Maybe<Scalars['String']['output']>;
  roleName?: Maybe<Scalars['String']['output']>;
};

/** What a role says about a permission (`PermissionDecision.effect`). */
export type PermissionEffect =
  | 'ALLOW'
  | 'DENY';

/**
 * One person in a "most important" list (`activityTopActors`, `auditTopPeople`,
 * `orderTopBuyers`, `reviewTopUploaders`, `entitlementTopHolders`): their public summary and
 * logins, and how many entries they have in that list.
 */
export type PersonCount = {
  __typename?: 'PersonCount';
  account: AccountSummary;
  /** Their entries in the list (the newest 2000 rows of a log or feed are counted). */
  count: Scalars['Int']['output'];
  /** Twitch first. */
  identities: Array<PublicIdentity>;
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

/** A linked Twitch or Kick account as anyone may see it: no IDs. */
export type PublicIdentity = {
  __typename?: 'PublicIdentity';
  displayName: Scalars['String']['output'];
  login: Scalars['String']['output'];
  platform: Platform;
};

/** Every limit, as `limits` answers. */
export type PublicLimits = {
  __typename?: 'PublicLimits';
  /**
   * The caller's per-account limits with their roles' overrides applied. Empty when signed out
   * and on `GET /v1/limits`.
   */
  account: Array<EffectiveLimit>;
  badge: MediaLimits;
  emote: MediaLimits;
  names: NameLimits;
  paint: MediaLimits;
  /**
   * The request rates, by the name of their `CYEX_TV_LIMIT_*` variable: per IP for the route
   * classes (`auth`, `gql`, `reads`, `upload`, `socket`), per account (or per IP signed out)
   * for the account limits, and per developer API key for `api_key` (ADR 0033).
   */
  rates: Array<RateInfo>;
  tags: TagLimits;
};

/** A role as anyone may see it on a profile. */
export type PublicRole = {
  __typename?: 'PublicRole';
  /** The role's colour, `#rrggbb`, if it has one. */
  color?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

export type Query = {
  __typename?: 'Query';
  /** An account's public summary. Null when the account is unknown, deleted or blocked. */
  account?: Maybe<AccountSummary>;
  /**
   * An account's public activity, newest first, like 7TV's "Activity" tab: sets and emotes the
   * owner or their editors changed, the active set, badge and paint picks, approved uploads.
   * Page back with `before` (an entry ID); `limit` is 1 to 100, default 30. Empty when the
   * account is unknown, deleted or blocked. Nothing private or from moderation shows. A page
   * can be shorter than `limit` and still not the last: use `accountActivityPage` to know.
   * Anyone but the owner and staff pages back through the newest 100 lines only.
   * A heavy read: 120 a minute per account, or per IP signed out (`rate_limited`).
   */
  accountActivity: Array<ActivityEntry>;
  /**
   * `accountActivity` with a cursor: `next` is the `before` for the following page, null when
   * there is none. A page can hold fewer than `limit` entries and still have a `next`, because
   * the feed skips what isn't public.
   *
   * `filter` narrows it: `groups` (emotes, sets, cosmetics, editors; editor lines only show
   * when asked for), `actorId` (only lines by the owner or that editor), and `since` /
   * `until` (RFC 3339; `bad_time` otherwise).
   *
   * Only the owner and staff (`users.view`) see the whole feed and may narrow it by time. For
   * everyone else it is the newest 100 lines, all time: `since` and `until` are ignored and
   * paging ends at the 100th line (`next` is null there; a cursor past it gets no entries).
   */
  accountActivityPage: ActivityPage;
  /**
   * The badges and paints an account owns, with since when and how each came (staff, the
   * shop, a code, an event); never who gave it, why or what it cost. Null unless the account
   * shows its collection (`AccountProfile.collectionPublic`); the account itself always sees
   * its own.
   */
  accountCollection?: Maybe<AccountCollection>;
  /**
   * The badge and paint up to 100 accounts show, with the badges and paints themselves, by
   * cyex.tv account ID (for names on profiles, tiles and lists). Only accounts showing
   * something are listed; blocked and deleted ones never. `too_many` over 100.
   */
  accountCosmetics: Array<AccountCosmetics>;
  /**
   * An account's editors (ADR 0014): yours, or, with `accountId`, those of an account you edit
   * with the `EDITORS` permission, or anyone's with `editors.manage_any`.
   */
  accountEditors: Array<AccountEditor>;
  /**
   * An account's emotes in every state, newest first. Needs `users.view`, `emotes.review`,
   * `emotes.takedown`, `emotes.edit_any`, `emotes.delete_any` or `emotes.flags`, or to be the account or its
   * editor with `EMOTE_MANAGE` on every set. `perPage` is 1 to 100, default 50.
   * Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
   * ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
   */
  accountEmotes: EmotePage;
  /**
   * Every permission an account holds or doesn't, and the role that decided each: its most
   * senior role that allows or denies it (Deny wins at equal positions), the User role last.
   * Yours, or anyone's with `roles.manage`, `roles.assign` or `users.view`.
   */
  accountPermissions: Array<PermissionDecision>;
  /**
   * An account's public profile: linked Twitch and Kick logins (no IDs), roles marked public,
   * and each channel's active set ("Active emotes"). Listed sets: `accountSets`. Null when the
   * account is unknown, deleted or blocked.
   */
  accountProfile?: Maybe<AccountProfile>;
  /**
   * Reports about an account's emotes and the ones it filed, newest first, at most 50 each.
   * Needs `emotes.review` or `emotes.takedown`.
   */
  accountReports: AccountReports;
  /**
   * Every restriction placed on an account, newest first (the latest 100), running or over,
   * with its reason and end. Needs `users.view`, `users.restrict` or a narrower restrict
   * permission.
   */
  accountRestrictions: Array<Restriction>;
  /** An account's roles. Yours, or anyone's with `roles.manage` or `users.view`. */
  accountRoles: Array<Role>;
  /**
   * Where an account is signed in, the most recently used first. Never tokens. Needs
   * `users.sessions`.
   */
  accountSessions: Array<AccountSession>;
  /**
   * An account's `LISTED` sets as profile cards, oldest first, at most 50. `preview` is how
   * many of each set's newest emotes to include, 0 to 50, default 12. Empty when the account
   * is unknown, deleted or blocked. Counts as a heavy read: 120 a minute per account, or per IP
   * signed out (`rate_limited`), and a high query cost.
   */
  accountSets: Array<SetSummary>;
  /**
   * What an account may do now: uploads, reports and sign-in, each with its end when timed.
   * Needs `users.view`, `users.restrict` or a narrower restrict permission.
   */
  accountStatus: AccountStatus;
  /**
   * An account's subscriptions: past, current and to come. There are no subscriptions yet,
   * so this is empty until there are. Needs `billing.view`, `subscriptions.manage` or
   * `shop.orders`.
   */
  accountSubscriptions: Array<AccountSubscription>;
  /**
   * Accounts for up to 100 pasted logins (`name`, `@name`, `twitch:name` or `kick:name`), in
   * order, each with the account or why there is none (`not_found`, `ambiguous`,
   * `bad_login`). For grants to many at once: needs `cosmetics.grant`, `roles.assign`,
   * `roles.manage` or `users.view`.
   */
  accountsByLogin: Array<LoginMatch>;
  /**
   * The site announcement shown under the header right now, or null. Public; shared for a few
   * seconds, so a change shows within that.
   */
  activeAnnouncement?: Maybe<Announcement>;
  /**
   * Who an account's (`accountId`) or an emote's (`emoteId`; exactly one, `bad_scope`
   * otherwise) public activity shows most often as by, most lines first (the newest 2000
   * lines are counted): the people a "By" filter offers before anything is typed. For an
   * account, `filter`'s groups and time range apply (its `actorId` doesn't). Empty when the
   * feed is. `limit` 1 to 10, default 5. A heavy read, as the feed.
   */
  activityTopActors: Array<PersonCount>;
  /**
   * An account as staff see it: identities, created time and restrictions. Needs
   * `users.view` or `users.restrict`.
   */
  adminAccount: AdminAccount;
  /**
   * Every site announcement, newest first (the latest 50), ended ones too. Needs
   * `site.announce`.
   */
  announcements: Array<Announcement>;
  /**
   * Audited changes, newest first. Needs `audit.view`. Narrow by target, `actorId`, an exact
   * `action`, and `since` (inclusive) / `until` (exclusive) as RFC 3339 times. Page back with
   * `before` (an entry ID); `limit` is 1 to 100, default 50.
   */
  auditLog: Array<AuditEntry>;
  /**
   * The audit log's busiest people for the filters `auditLog` takes: its actors (`by: ACTOR`,
   * the default) or the accounts its entries are about (`by: TARGET`), most entries first
   * among the newest 2000. Needs `audit.view`. `limit` 1 to 10, default 5.
   */
  auditTopPeople: Array<PersonCount>;
  /** A channel by its platform ID. */
  channel?: Maybe<Channel>;
  /** A channel by its login or slug. Logins change; store the ID, not the login. */
  channelByLogin?: Maybe<Channel>;
  /**
   * A channel, by its platform ID (`id`) or its login (`login`; one of the two), with the sets
   * of its account you may add, rename and remove emotes in, your role on it and whether its
   * active set is among them. The same rights check as `addSetEmote`, `updateSetEmote` and
   * `removeSetEmote` (owner, account editors with `EMOTES` covering the set, set editors,
   * `sets.edit_any`). Null when the channel is unknown. Signed-in only.
   */
  channelEditableSets?: Maybe<ChannelEditableSets>;
  /**
   * A channel's webhooks, for its owner and its account editors with the channel settings
   * permission (`forbidden` for anyone else).
   */
  channelWebhooks: Array<Webhook>;
  /**
   * Several channels of one platform at once, by their platform IDs, in the order asked,
   * each once (ADR 0035). Channels not on cyex.tv are left out. At most 100 IDs
   * (`too_many`); every 10 IDs cost what one `channel` lookup would.
   */
  channels: Array<Channel>;
  /**
   * Redeem code batches, newest first. Needs `codes.manage`. The codes themselves are shown
   * only once, by `createCodeBatch`. `perPage` is 1 to 100, default 50.
   */
  codeBatches: CodeBatchPage;
  /**
   * Who owns which badges or paints, newest first; narrow it with `accountId` and/or
   * `cosmeticId`. `perPage` is 1 to 100, default 50. Needs `cosmetics.grant`.
   */
  cosmeticEntitlements: EntitlementPage;
  /**
   * A badge or paint picture, to see whether its files are made (`ready`). Needs
   * `cosmetics.manage`.
   */
  cosmeticImage?: Maybe<CosmeticImage>;
  /**
   * The cyex.tv chat badges and name paints that are for sale in the shop or held by at least
   * one account (ADR 0015). Public; unreleased ones are left out, except for staff with a
   * `cosmetics.*` permission. Clients cache it (`GET /v1/cosmetics` has an ETag) and follow
   * the `cosmetics` topic.
   */
  cosmetics: CosmeticCatalog;
  /** The channels of the accounts you edit. */
  editorChannels: Array<Channel>;
  /** The accounts you edit, with what you may do on each. */
  editorOf: Array<EditedAccount>;
  /** The sets of the accounts you edit. Each set names its owner. */
  editorSets: Array<EmoteSet>;
  /** An emote. Pending ones are visible to their owner and to moderators only. */
  emote?: Maybe<Emote>;
  /**
   * An emote's public activity, newest first: `EMOTE_UPLOADED` (when it was approved, shown
   * as by its owner) and `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED` in listed
   * sets, by their owners and account editors. Page back with `before` (an entry ID); `limit`
   * is 1 to 100, default 30. Empty when the emote is unknown or not public. Nothing private or
   * from moderation shows. A page can be shorter than `limit` and still not the last: use
   * `emoteActivityPage` to know. A heavy read: 120 a minute per account, or per IP signed out
   * (`rate_limited`).
   */
  emoteActivity: Array<ActivityEntry>;
  /**
   * `emoteActivity` with a cursor: `next` is the `before` for the following page, null when
   * there is none. A page can hold fewer than `limit` entries and still have a `next`.
   * `actorId` keeps only the lines shown as by that account (an upload is its owner's).
   */
  emoteActivityPage: ActivityPage;
  /**
   * Channels whose active set holds the emote, most recently added first. `limit` is 1 to 50,
   * default 8; `offset` skips that many (0 to 10000, default 0) for paging. Empty when the
   * emote is unknown or not public.
   */
  emoteChannels: Array<Channel>;
  /**
   * Emotes that may duplicate one in the review queue: the same file (sha256) first, then
   * similar names, closest first; at most 12, any state but deleted. Needs `emotes.review`.
   */
  emoteDuplicates: Array<DuplicateCandidate>;
  /**
   * What happened to one emote, newest first: the latest 100 audit entries about it
   * (upload, edits, reviews, notes, takedowns). Needs `emotes.review`.
   */
  emoteHistory: Array<AuditEntry>;
  /** Reviewers' notes on an emote, newest first (the latest 200). Needs `emotes.review`. */
  emoteNotes: Array<EmoteNote>;
  /**
   * Retired with transfer requests (ADR 0038): always empty. Past requests and transfers are in
   * the audit log.
   * @deprecated Retired (ADR 0038): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote.
   */
  emoteTransferRequests: Array<EmoteTransferRequest>;
  /**
   * Where an emote stands among the 100 most added in `window` (`DAY`, `WEEK` by default or
   * `MONTH`): its rank and the channels that added it, net of removals. Null when `id` isn't
   * an emote ID, the emote isn't approved and listed, or it is not in the top 100.
   */
  emoteTrending?: Maybe<EmoteTrending>;
  /**
   * Several emotes at once, in the order asked, each once (ADR 0035): who may see what is the
   * rule of `emote`, and unknown or hidden emotes are left out. At most 100 IDs (`too_many`);
   * every 10 IDs cost what one `emote` lookup would.
   */
  emotes: Array<Emote>;
  /**
   * Public emotes already made from the file with this sha256 (64 hex digits), oldest first,
   * at most 3: the upload form asks before sending, so it can say "looks like" (it never
   * blocks). Signed in.
   */
  emotesWithSameFile: Array<Emote>;
  /**
   * The entitlement graph of an account: every badge, paint and role it holds or held, where
   * each came from (staff, an order, a redeem code, an event, the User role, the owner list),
   * who gave it and when, and when and by whom it was taken away. Held ones first. Needs
   * `entitlements.view` or `cosmetics.grant`.
   */
  entitlementGraph: Array<EntitlementGrant>;
  /**
   * The reverse: who holds or held one badge, paint or role, and how each got it (the latest
   * 200 of each kind of record). Not for the User and top roles (`implicit_role`). Needs
   * `entitlements.view` or `cosmetics.grant`.
   */
  entitlementHolders: Array<EntitlementGrant>;
  /**
   * The accounts holding the most badges, paints and roles. Needs `entitlements.view` or
   * `cosmetics.grant`. `limit` 1 to 10, default 5.
   */
  entitlementTopHolders: Array<PersonCount>;
  /** The IDs of all your favourite emotes (up to 1000), the latest first: for marking them. */
  favouriteEmoteIds: Array<Scalars['String']['output']>;
  /** The cyex.tv global set, shown in every channel. */
  globalSet: EmoteSet;
  /**
   * Creators whose emotes you hid from your own view, the latest first: all of them, up to 200.
   * Only you can read it.
   */
  hiddenCreators: Array<HiddenCreator>;
  /**
   * Emotes you hid from your own view ("Hide for me"), the latest first: all of them, up to
   * 1000. Only you can read it; the public reads stay the same for everyone, so filter with
   * this list (and `hiddenCreators`) on your side.
   */
  hiddenEmotes: Array<HiddenEmote>;
  /**
   * Every per-account limit a role can change, with what everyone gets without one. Needs
   * `roles.manage`.
   */
  limitDefaults: Array<RoleLimit>;
  /**
   * Every upload and field limit in one public list: per kind (emote, badge, paint) the file
   * size, side, frames, pixels and duration; the name and tag lengths; the per-IP rates. Signed
   * in, `account` also lists your own per-account limits with your roles' overrides applied.
   */
  limits: PublicLimits;
  /** The signed-in account. */
  me: Me;
  /**
   * The API's version, build, where its limits are published (the `limits` field) and what
   * is deprecated, with the sunset date of each. Public. Same as `GET /v1/meta`.
   */
  meta: ApiMeta;
  /**
   * Your developer API keys, oldest first, each with its requests per day for the last 30
   * days. Never the keys themselves.
   */
  myApiKeys: Array<ApiKey>;
  /** Your channels: one per linked platform you have signed in with. */
  myChannels: Array<Channel>;
  /**
   * Your live personal API token (a console token, ADR 0028), as `mySessions` lists it: its
   * `id` (pass to `revokeSession`), when it was made and when it expires (3 days after). Null
   * when you have none: it rotates whenever you sign a session out, one or all (ADR 0029), and
   * `createConsoleToken` makes the next one. Never the token itself.
   */
  myConsoleToken?: Maybe<AccountSession>;
  /**
   * The cosmetics you own and show; with `accountId`, those of an account you edit with
   * `PROFILE`.
   */
  myCosmetics: OwnedCosmetics;
  /**
   * Channels you can manage: yours (`OWNER`), those of accounts you are an account editor of
   * (`ACCOUNT_EDITOR`), and those of accounts where you edit at least one set (`SET_EDITOR`).
   * Each channel once, under your highest role; sorted by role (`OWNER`, `ACCOUNT_EDITOR`,
   * `SET_EDITOR`), then login (ignoring case), then platform. Signed-in only.
   */
  myEditableChannels: Array<EditableChannel>;
  /**
   * Your own emotes in every state, newest first. `perPage` is 1 to 100, default 50.
   * Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
   * ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
   */
  myEmotes: EmotePage;
  /**
   * Your favourite emotes, the latest added first. `query` keeps the ones whose name holds it
   * (any case). `perPage` is 1 to 100, default 50. Only you can read it.
   */
  myFavourites: EmotePage;
  /**
   * Every notification kind and what you chose for it: created at all (`notify`) and popping
   * up on the website (`toast`). `required` kinds always notify.
   */
  myNotificationPrefs: Array<NotificationPref>;
  /**
   * Your notifications, newest first (the latest 100). `kind` is one of `emote.approved`,
   * `emote.rejected`, `emote.taken_down`, `emote.visibility_changed` (staff changed its
   * visibility, ADR 0024: `visibility` and `was`), `editor.added`, `editor.changed`, `editor.removed`,
   * `cosmetic.granted` and `cosmetic.revoked`; `payload` is JSON with the IDs and names it is
   * about (see the API reference). Cosmetics from the shop and from codes carry `source`
   * (`purchase` or `code`), and from orders `order_id`.
   */
  myNotifications: Array<Notification>;
  /**
   * Orders you bought, and gifts you got once paid (without what the buyer paid), newest
   * first. `perPage` is 1 to 100, default 50.
   */
  myOrders: OrderPage;
  /**
   * Where you are signed in: your live sessions, the most recently used first, with the one
   * this request came with marked `current`. Never shows tokens.
   */
  mySessions: Array<AccountSession>;
  /** The sets you own, then the ones you edit. Pending emotes included. */
  mySets: Array<EmoteSet>;
  /**
   * One order, for its buyer, its recipient (once paid, without what the buyer paid), or
   * `shop.orders`. Poll it after `startCheckout` until it is `PAID`.
   */
  order?: Maybe<Order>;
  /**
   * The buyers with the most orders in a state and of an item. Needs `shop.orders`.
   * `limit` 1 to 10, default 5.
   */
  orderTopBuyers: Array<PersonCount>;
  /**
   * Every order, newest first; narrow by status, account (bought or received) and item.
   * Needs `shop.orders`, or `billing.view` narrowed to one account. `perPage` is 1 to 100, default
   * 50.
   */
  orders: OrderPage;
  /**
   * The deletion you can still undo: set only for the cookie session that signing in to a
   * deleted account within its 30 days gave (`restoreAccount` undoes it). Null otherwise,
   * including for every normal session.
   */
  pendingDeletion?: Maybe<PendingDeletion>;
  /** Every permission name a role can hold. */
  permissions: Array<Scalars['String']['output']>;
  /** One user's personal set. */
  personalSet?: Maybe<EmoteSet>;
  /** Personal sets for up to 100 chatters at once. */
  personalSets: Array<PersonalSetEntry>;
  /**
   * The header search: the top 5 emotes for `query` by the name matcher (exact name, same
   * ignoring case, name prefix, word or initials prefix such as `point` or `pp` for
   * `PepePoint`, substring, then similar names for 3+ characters; most used first within a
   * tier), and the top 3 accounts (`searchAccounts` ranking, a leading `@` ignored, under 2
   * characters finds none). Approved, listed, non-NSFW emotes only. 240 searches a minute per
   * account, or per IP signed out (`rate_limited`).
   */
  quickSearch: QuickSearch;
  /**
   * Open reports, oldest first. Needs `emotes.review` or `emotes.takedown`. `perPage` is 1 to 100,
   * default 50.
   */
  reportQueue: ReportQueue;
  /**
   * Pending emotes, oldest first. Needs `emotes.review` (the `PERSONAL` tab also takes
   * `emotes.review_personal`). `perPage` is 1 to 100, default 50. With `filter`: one tab (not
   * private, private, resolved, or the personal queue, ADR 0038), a search by name or
   * uploader, a personal-use state, and an order. A cursor only works for the same filter (`bad_cursor`).
   * Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
   * ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
   */
  reviewQueue: ReviewQueue;
  /**
   * The ready reject reasons the review queue offers, in order. Needs `emotes.review` or
   * `review.settings`.
   */
  reviewReasons: Array<Scalars['String']['output']>;
  /**
   * The uploaders with the most emotes in one tab of the review queue (`filter`'s tab and
   * search; its uploader and order don't matter). Needs `emotes.review`. `limit` 1 to 10,
   * default 5.
   */
  reviewTopUploaders: Array<PersonCount>;
  /**
   * A role's limits: each per-account limit with its default and the role's override. An
   * account counts under the most generous override among its roles. Needs `roles.manage`.
   */
  roleLimits: Array<RoleLimit>;
  /**
   * All roles, most senior first, each with whether you may edit or assign it. Needs
   * `roles.manage` or `roles.assign`. The top role is listed to the owners only.
   */
  roles: Array<Role>;
  /**
   * Accounts by Twitch or Kick login or display name (a leading `@` is ignored): exact login
   * first, then login and name prefixes, then similar names. Public fields only; deleted and
   * blocked accounts are never listed. `perPage` is 1 to 50, default 20. 60 searches a minute
   * per account, or per IP signed out (`rate_limited`). A `query` under 2 characters (after the
   * `@`) finds nobody. Matches are counted up to 1000: `totalCapped` says there are more, and
   * the pages end there. Pass a page's `next` as `after` for the page after it (cursor paging,
   * ADR 0030; `page` is ignored then); a cursor only works for the same `query` (`bad_cursor`).
   */
  searchAccounts: AccountResults;
  /**
   * Approved, listed emotes by name and tag, best match first, then the most used.
   * `perPage` is 1 to 100, default 50. `nsfw` is retired and changes nothing: NSFW emotes
   * aren't allowed, so none is ever listed. `sort` is
   * `RELEVANCE` (default: best match, then most used), `TOP` (most used, then name), `NEW`
   * (newest first) or `TRENDING` (most channels added it in `window`, net of removals, then
   * most used; only emotes with a net gain are listed). An empty `query` lists every approved,
   * listed emote.
   * `flags` keeps the emotes that have every flag listed (`ZERO_WIDTH`, `WIDE`; at most 3
   * different ones). `NSFW` is retired and finds nothing. `tags` keeps
   * the emotes that have every tag listed (exact; a leading `#` and case are ignored, at most
   * 10). `exact: true` makes a non-empty `query` match the whole name (any case) instead of
   * roughly. `window` is `DAY`, `WEEK` (default) or `MONTH`, and only matters for `TRENDING`.
   *
   * Cursor paging (ADR 0030): pass a page's `next` as `after` to read the page after it, in
   * the same order, with nothing shown twice or skipped when emotes are added meanwhile; `page`
   * is ignored then and there is no depth limit. `next` is null on the last page. A cursor only
   * works with the same `query`, filters and `sort` (`bad_cursor` otherwise).
   */
  searchEmotes: SearchResults;
  /**
   * A set with its emotes. Null when there is none, or when it is `PRIVATE` and you are not
   * its owner, an editor or staff (a channel's active set is always public).
   */
  set?: Maybe<EmoteSet>;
  /**
   * What you may set a set's capacity to (`updateSet`): from the emotes it holds (at least 1)
   * to the owner's `set_capacity_max` role limit (1.000 by default), or 1.000.000 with
   * `sets.edit_any`. Null when you can't change its capacity. Signed in.
   */
  setCapacityRange?: Maybe<SetCapacityRange>;
  /**
   * What changed in a set after `since`, a `version` you hold (delta sync, ADR 0035). Apply
   * `added`, `renamed` and `updated` as upserts by emote ID and `removedIds` as deletes, then
   * keep `version`. `full: true` (with the whole set in `set`) when the change log no longer
   * reaches back to `since` (7 days or 1.000 versions) or `since` isn't a version of this
   * set. Null when `set` would be.
   */
  setChanges?: Maybe<SetChanges>;
  /** A set's editors. For its owner, its editors and `sets.edit_any`. */
  setEditors: Array<AccountSummary>;
  /**
   * A page of a set's emotes, oldest first, for sets too big to read whole. Who may see what is
   * the rule of `set`; an unknown or hidden set is an empty page. `query` keeps the emotes whose
   * name in the set or own name contains it (case-insensitive). `perPage` is 1 to 200, default
   * 120; `total` counts every match.
   * Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
   * ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
   */
  setEmotes: SetEmotePage;
  /**
   * Several sets at once, in the order asked, each once (ADR 0035): who may see what is the
   * rule of `set`, and unknown or hidden sets are left out. At most 25 IDs (`too_many`); one
   * call costs what that many `set` lookups would.
   */
  sets: Array<EmoteSet>;
  /** One shop item: on sale for everyone; any, deleted ones too, with `shop.manage`. */
  shopItem?: Maybe<ShopItem>;
  /**
   * Shop items on sale now, by sort order (ADR 0016). Public; signed in, `owned` says whether
   * you own everything in each. With `all: true`, every item that isn't deleted, hidden and
   * scheduled ones too (needs `shop.manage`).
   */
  shopItems: Array<ShopItem>;
  /** Staff notes on an account, newest first (the latest 200). Needs `users.notes`. */
  staffNotes: Array<StaffNote>;
  /** How many of your notifications are unread. */
  unreadNotificationCount: Scalars['Int']['output'];
  /**
   * Where uploads stand: how many emotes wait for review and how many may, and the uploads
   * this hour. Yours, or with `ownerId` an account you may upload for.
   */
  uploadQuota: UploadQuota;
  /**
   * The badge and paint up to 100 chatters show, as catalog IDs. Only chatters showing
   * something are listed.
   */
  userCosmetics: Array<UserCosmetics>;
  /** A webhook's latest deliveries, newest first (the last 50 are kept). */
  webhookDeliveries: Array<WebhookDelivery>;
};


export type QueryAccountArgs = {
  id: Scalars['String']['input'];
};


export type QueryAccountActivityArgs = {
  accountId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountActivityPageArgs = {
  accountId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  filter?: InputMaybe<ActivityFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountCollectionArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountCosmeticsArgs = {
  accountIds: Array<Scalars['String']['input']>;
};


export type QueryAccountEditorsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryAccountEmotesArgs = {
  accountId: Scalars['String']['input'];
  after?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountPermissionsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountProfileArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountReportsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountRestrictionsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountRolesArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountSessionsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountSetsArgs = {
  accountId: Scalars['String']['input'];
  preview?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountStatusArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountSubscriptionsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountsByLoginArgs = {
  logins: Array<Scalars['String']['input']>;
};


export type QueryActivityTopActorsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
  emoteId?: InputMaybe<Scalars['String']['input']>;
  filter?: InputMaybe<ActivityFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAdminAccountArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAuditLogArgs = {
  action?: InputMaybe<Scalars['String']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  since?: InputMaybe<Scalars['String']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  targetKind?: InputMaybe<Scalars['String']['input']>;
  until?: InputMaybe<Scalars['String']['input']>;
};


export type QueryAuditTopPeopleArgs = {
  action?: InputMaybe<Scalars['String']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
  by?: InputMaybe<AuditPersonRole>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  since?: InputMaybe<Scalars['String']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  targetKind?: InputMaybe<Scalars['String']['input']>;
  until?: InputMaybe<Scalars['String']['input']>;
};


export type QueryChannelArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryChannelByLoginArgs = {
  login: Scalars['String']['input'];
  platform: Platform;
};


export type QueryChannelEditableSetsArgs = {
  id?: InputMaybe<Scalars['String']['input']>;
  login?: InputMaybe<Scalars['String']['input']>;
  platform: Platform;
};


export type QueryChannelWebhooksArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryChannelsArgs = {
  ids: Array<Scalars['String']['input']>;
  platform: Platform;
};


export type QueryCodeBatchesArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryCosmeticEntitlementsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
  cosmeticId?: InputMaybe<Scalars['String']['input']>;
  kind: CosmeticKind;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryCosmeticImageArgs = {
  id: Scalars['String']['input'];
};


export type QueryEmoteArgs = {
  id: Scalars['String']['input'];
};


export type QueryEmoteActivityArgs = {
  before?: InputMaybe<Scalars['String']['input']>;
  emoteId: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryEmoteActivityPageArgs = {
  actorId?: InputMaybe<Scalars['String']['input']>;
  before?: InputMaybe<Scalars['String']['input']>;
  emoteId: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryEmoteChannelsArgs = {
  id: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryEmoteDuplicatesArgs = {
  emoteId: Scalars['String']['input'];
};


export type QueryEmoteHistoryArgs = {
  emoteId: Scalars['String']['input'];
};


export type QueryEmoteNotesArgs = {
  emoteId: Scalars['String']['input'];
};


export type QueryEmoteTransferRequestsArgs = {
  emoteId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryEmoteTrendingArgs = {
  id: Scalars['String']['input'];
  window?: InputMaybe<TrendingWindow>;
};


export type QueryEmotesArgs = {
  ids: Array<Scalars['ID']['input']>;
};


export type QueryEmotesWithSameFileArgs = {
  sha256: Scalars['String']['input'];
};


export type QueryEntitlementGraphArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryEntitlementHoldersArgs = {
  id: Scalars['String']['input'];
  kind: GrantKind;
};


export type QueryEntitlementTopHoldersArgs = {
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryMyCosmeticsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryMyEmotesArgs = {
  after?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryMyFavouritesArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query?: InputMaybe<Scalars['String']['input']>;
};


export type QueryMyNotificationsArgs = {
  unreadOnly?: InputMaybe<Scalars['Boolean']['input']>;
};


export type QueryMyOrdersArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryOrderArgs = {
  id: Scalars['String']['input'];
};


export type QueryOrderTopBuyersArgs = {
  itemId?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<OrderStatus>;
};


export type QueryOrdersArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
  itemId?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<OrderStatus>;
};


export type QueryPersonalSetArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryPersonalSetsArgs = {
  platform: Platform;
  userIds: Array<Scalars['String']['input']>;
};


export type QueryQuickSearchArgs = {
  query: Scalars['String']['input'];
};


export type QueryReportQueueArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryReviewQueueArgs = {
  after?: InputMaybe<Scalars['String']['input']>;
  filter?: InputMaybe<ReviewQueueFilter>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryReviewTopUploadersArgs = {
  filter?: InputMaybe<ReviewQueueFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryRoleLimitsArgs = {
  id: Scalars['String']['input'];
};


export type QuerySearchAccountsArgs = {
  after?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query: Scalars['String']['input'];
};


export type QuerySearchEmotesArgs = {
  after?: InputMaybe<Scalars['String']['input']>;
  animated?: InputMaybe<Scalars['Boolean']['input']>;
  exact?: InputMaybe<Scalars['Boolean']['input']>;
  flags?: InputMaybe<Array<EmoteFlag>>;
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  owner?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query: Scalars['String']['input'];
  sort?: InputMaybe<EmoteSort>;
  tags?: InputMaybe<Array<Scalars['String']['input']>>;
  window?: InputMaybe<TrendingWindow>;
};


export type QuerySetArgs = {
  id: Scalars['String']['input'];
};


export type QuerySetCapacityRangeArgs = {
  id: Scalars['String']['input'];
};


export type QuerySetChangesArgs = {
  id: Scalars['String']['input'];
  since: Scalars['Int']['input'];
};


export type QuerySetEditorsArgs = {
  setId: Scalars['String']['input'];
};


export type QuerySetEmotesArgs = {
  after?: InputMaybe<Scalars['String']['input']>;
  id: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query?: InputMaybe<Scalars['String']['input']>;
};


export type QuerySetsArgs = {
  ids: Array<Scalars['ID']['input']>;
};


export type QueryShopItemArgs = {
  id: Scalars['String']['input'];
};


export type QueryShopItemsArgs = {
  all?: InputMaybe<Scalars['Boolean']['input']>;
};


export type QueryStaffNotesArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryUploadQuotaArgs = {
  ownerId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryUserCosmeticsArgs = {
  platform: Platform;
  userIds: Array<Scalars['String']['input']>;
};


export type QueryWebhookDeliveriesArgs = {
  id: Scalars['String']['input'];
};

/** The header search: the best emotes and accounts for one query. */
export type QuickSearch = {
  __typename?: 'QuickSearch';
  emotes: Array<Emote>;
  users: Array<AccountHit>;
};

/** A count of requests or writes per window. */
export type RateInfo = {
  __typename?: 'RateInfo';
  /** Requests allowed per window (the burst). */
  count: Scalars['Int']['output'];
  /** For example `gql`, `reads`, `upload`: the name of `CYEX_TV_LIMIT_<NAME>`. */
  key: Scalars['String']['output'];
  windowSeconds: Scalars['Int']['output'];
};

/** Who gets a gift: their Twitch or Kick login. */
export type RecipientInput = {
  login: Scalars['String']['input'];
  platform: Platform;
};

/** What a redeemed code gave. */
export type RedeemResult = {
  __typename?: 'RedeemResult';
  /** Newly owned. */
  badges: Array<Badge>;
  /** The batch's name. */
  name: Scalars['String']['output'];
  paints: Array<Paint>;
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

/** One restriction placed on an account, running or over. */
export type Restriction = {
  __typename?: 'Restriction';
  /** Still holding now. */
  active: Scalars['Boolean']['output'];
  createdAt: Scalars['String']['output'];
  /** The staff account that placed it. */
  createdBy?: Maybe<Scalars['String']['output']>;
  /** When it ends; empty for good. */
  endsAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  kind: RestrictionKind;
  /** When staff lifted it early, or a newer restriction of the same kind replaced it. */
  liftedAt?: Maybe<Scalars['String']['output']>;
  liftedBy?: Maybe<Scalars['String']['output']>;
  /** Why, as staff wrote it. Empty on restrictions made without one (`restrictAccount`). */
  reason?: Maybe<Scalars['String']['output']>;
};

/** A restriction to place. */
export type RestrictionInput = {
  kind: RestrictionKind;
  /** Why: 1 to 500 characters. Staff see it; the account doesn't. */
  reason: Scalars['String']['input'];
  /** When it ends (RFC 3339, in the future, at most ten years out); empty for good. */
  until?: InputMaybe<Scalars['String']['input']>;
};

/** What a restriction stops. */
export type RestrictionKind =
  /** The whole account: no sign-in, sessions ended (`users.block`). */
  | 'ACCOUNT'
  /** Filing reports and tickets (`users.restrict_tickets`). */
  | 'TICKETS'
  /** Uploading emotes (`users.restrict_uploads`). */
  | 'UPLOADS';

/** How many emotes wait for review, by tab. */
export type ReviewCounts = {
  __typename?: 'ReviewCounts';
  /** Pending and private (every upload is unlisted until its listing is approved, ADR 0038). */
  limited: Scalars['Int']['output'];
  /** Personal use pending, emote usable: the personal queue (ADR 0038). */
  personal: Scalars['Int']['output'];
  /** Pending and not private: listed once its listing is approved. */
  public: Scalars['Int']['output'];
};

export type ReviewQueue = {
  __typename?: 'ReviewQueue';
  /** How many wait in each pending tab, whatever the read asked for. */
  counts: ReviewCounts;
  items: Array<Emote>;
  /**
   * Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
   * page. Opaque: a list's cursor only works for the same list, filters and order.
   */
  next?: Maybe<Scalars['String']['output']>;
  /** How many match the read in all (its tab and search). */
  total: Scalars['Int']['output'];
};

/** What `reviewQueue` lists. Every field is optional: none is every pending emote, oldest first. */
export type ReviewQueueFilter = {
  /** Newest (or Z) first. Default: oldest first, except `RESOLVED`, latest review first. */
  descending?: InputMaybe<Scalars['Boolean']['input']>;
  /** Only emotes whose personal use is in this state (ADR 0038), for example `APPROVED`. */
  personalUse?: InputMaybe<ApprovalState>;
  /** Part of the emote's name or of its uploader's login or display name, any case. */
  search?: InputMaybe<Scalars['String']['input']>;
  sort?: InputMaybe<ReviewSort>;
  tab?: InputMaybe<ReviewTab>;
  /** Only this uploader's emotes (an account ID). */
  uploaderId?: InputMaybe<Scalars['String']['input']>;
};

/** How the review queue is ordered. Ties go by upload time. */
export type ReviewSort =
  /** The emote's name. */
  | 'NAME'
  /** Review time. The default for `RESOLVED`. */
  | 'REVIEWED'
  /** Upload time. The default for the pending tabs. */
  | 'UPLOADED'
  /** The uploader's display name. */
  | 'UPLOADER';

/** A part of the review queue. */
export type ReviewTab =
  /** Pending and private. */
  | 'LIMITED'
  /** Every pending emote. The default. */
  | 'PENDING'
  /**
   * The personal queue (ADR 0038): personal use pending, and the emote may be used (pending
   * or approved). Needs `emotes.review_personal` (or `emotes.review`).
   */
  | 'PERSONAL'
  /** Pending and not private: search shows it once its listing is approved. */
  | 'PUBLIC'
  /** Approved or rejected by a reviewer. */
  | 'RESOLVED';

/** What a batch review does to every emote in it. */
export type ReviewVerdict =
  | 'APPROVE'
  | 'REJECT';

export type Role = {
  __typename?: 'Role';
  /**
   * Whether you may give it to someone (and take it away): below your highest role, you hold
   * what it grants, and not the User or top role.
   */
  assignable: Scalars['Boolean']['output'];
  /**
   * Set on the built-in roles: `USER` (every account holds it), `ADMIN` and `ROOT` (the
   * owners' top role, shown only to its holders).
   */
  builtIn?: Maybe<BuiltInRole>;
  /**
   * Optional colour, `#rrggbb` in lower case. Drawn as a dot on its chips and, for the most
   * senior coloured public role, as the ring round its holders' profile pictures
   * (`AccountProfile.roleColor`). Readable on both themes: the editor shows the contrast.
   */
  color?: Maybe<Scalars['String']['output']>;
  /**
   * What it denies, sorted: a Deny here beats an Allow on any lower role (the User role
   * included). Never also in `permissions`; a permission in neither is left to the roles below.
   */
  denies: Array<Scalars['String']['output']>;
  description?: Maybe<Scalars['String']['output']>;
  /** Whether you may change it: below your highest role, not one you hold, not the top role. */
  editable: Scalars['Boolean']['output'];
  id: Scalars['String']['output'];
  /**
   * Never shown publicly (profiles, user cards), even when `public`, and ignored wherever roles
   * rank people. Staff and its holders still see it.
   */
  invisible: Scalars['Boolean']['output'];
  name: Scalars['String']['output'];
  /** What it allows, sorted. */
  permissions: Array<Scalars['String']['output']>;
  /** Higher means more senior. You can only manage roles below your own highest one. */
  position: Scalars['Int']['output'];
  /** Shown on its holders' public profiles (`accountProfile`). Off by default. */
  public: Scalars['Boolean']['output'];
};

/**
 * One per-account limit and a role's override of it (`roleLimits`, `limitDefaults`). An
 * account's limit is the most generous override among its roles, else `default`.
 */
export type RoleLimit = {
  __typename?: 'RoleLimit';
  /** What everyone gets without an override (set when the API starts). */
  default: Scalars['Int']['output'];
  group: LimitGroup;
  /**
   * `gql_account`, `uploads_per_hour`, `pending_emotes`, `editors`, `cosmetics`,
   * `set_clones`, `account_search`, `quick_search`, `heavy_reads`, `personal_lists`,
   * `emote_transfers`, `prefs`, `checkout`, `redeem`, `data_export`, `set_capacity_max`, or a
   * media limit: `badge_max_bytes`,
   * `badge_max_side`, `badge_max_frames`, `paint_max_bytes` or `paint_max_side` (bytes, px
   * and frames of the largest file an account may upload).
   */
  key: Scalars['String']['output'];
  /**
   * The most this limit may be set to: 1,000,000 for a count, and for a media limit the
   * ceiling the API runs with (an emote's side never above 1000 px).
   */
  max: Scalars['Int']['output'];
  /** The role's own count; null when it keeps the default. */
  value?: Maybe<Scalars['Int']['output']>;
  /**
   * The window the count is per, in seconds; null for `pending_emotes`, a cap on how many
   * wait at once.
   */
  windowSeconds?: Maybe<Scalars['Int']['output']>;
};

/** One override for `setRoleLimits`. */
export type RoleLimitInput = {
  key: Scalars['String']['input'];
  /** 1 up to the limit's `max` (1,000,000 for a count); null keeps the default. */
  value?: InputMaybe<Scalars['Int']['input']>;
};

export type SearchResults = {
  __typename?: 'SearchResults';
  items: Array<Emote>;
  /**
   * Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
   * page. Opaque: a list's cursor only works for the same list, filters and order.
   */
  next?: Maybe<Scalars['String']['output']>;
  page: Scalars['Int']['output'];
  perPage: Scalars['Int']['output'];
  total: Scalars['Int']['output'];
};

/** How a session signed in. */
export type SessionKind =
  /** A Bearer token for an app (cyex deck, the extension). */
  | 'APP'
  /** A personal API token you made on cyex.tv (`createConsoleToken`, ADR 0028). */
  | 'CONSOLE'
  /** The cyex.tv website's cookie. */
  | 'SITE';

/** How you may change a set's emotes (`channelEditableSets`). */
export type SetAccess =
  /** You are an account editor of its owner with `EMOTES`, and your grant covers this set. */
  | 'ACCOUNT_EDITOR'
  /** The set is yours. */
  | 'OWNER'
  /** You are an editor of this set. */
  | 'SET_EDITOR'
  /** Through a staff permission (`sets.edit_any`); your changes are audited. */
  | 'STAFF';

/** What you may set a set's capacity to (`setCapacityRange`; ADR 0017, amendment 2026/11/22). */
export type SetCapacityRange = {
  __typename?: 'SetCapacityRange';
  /** The most slots you may pick. */
  max: Scalars['Int']['output'];
  /** The fewest slots: 1, or the emotes the set holds now if that is more. */
  min: Scalars['Int']['output'];
  /**
   * The most without a staff permission: the owner's `set_capacity_max` role limit (1.000 by
   * default) for a channel set, 5 for a personal set, or the set's capacity if it is higher.
   */
  ownerMax: Scalars['Int']['output'];
  /**
   * `max` comes from a staff permission (`sets.edit_any`, or `global_set.edit` for the global
   * set): up to 1.000.000.
   */
  staff: Scalars['Boolean']['output'];
  /** Emotes in the set now, pending ones included. */
  used: Scalars['Int']['output'];
};

/**
 * What changed in a set after a version (`setChanges`, `GET /v1/sets/{id}?since=`; ADR 0035).
 * Apply `added`, `renamed` and `updated` as upserts by `emote.id`, and `removedIds` as deletes;
 * then keep `version`. With `full: true` the change log doesn't reach back to `since` (or
 * `since` is not a version this set had): replace your copy with `set` instead.
 */
export type SetChanges = {
  __typename?: 'SetChanges';
  /** Members added since (or newly visible to you, for example an emote approved). */
  added: Array<SetEmote>;
  /** True when this is a full resync: `set` holds the whole set and the lists are empty. */
  full: Scalars['Boolean']['output'];
  id: Scalars['String']['output'];
  /** The set's name now. */
  name: Scalars['String']['output'];
  /** Emotes no longer in the set as you see it (removed, taken down, deleted, hidden). */
  removedIds: Array<Scalars['String']['output']>;
  /** Members whose name or zero-width flag in the set changed. */
  renamed: Array<SetEmote>;
  /** The whole set, only when `full` is true. */
  set?: Maybe<EmoteSet>;
  /** The `since` asked for. */
  since: Scalars['Int']['output'];
  /** Members whose emote changed (its files, flags, state or own name). */
  updated: Array<SetEmote>;
  /** The version this answer brings you to. */
  version: Scalars['Int']['output'];
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

/** A page of a set's members and how many match, in all (`setEmotes`). */
export type SetEmotePage = {
  __typename?: 'SetEmotePage';
  items: Array<SetEmote>;
  /**
   * Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
   * page. Opaque: a list's cursor only works for the same list, filters and order.
   */
  next?: Maybe<Scalars['String']['output']>;
  /** Members the viewer may see that match `query`, over every page. */
  total: Scalars['Int']['output'];
};

export type SetKind =
  | 'CHANNEL'
  | 'GLOBAL'
  | 'PERSONAL';

/** A set as a card on a profile: its facts and the first few emotes (`accountSets`). */
export type SetSummary = {
  __typename?: 'SetSummary';
  /** A channel shows this set now. */
  active: Scalars['Boolean']['output'];
  capacity: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  /** The set's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input. */
  id: Scalars['String']['output'];
  kind: SetKind;
  listed: Scalars['Boolean']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  /** The newest emotes you may see, at most the `preview` asked for. */
  preview: Array<SetEmote>;
  /** How many slots the set uses, as counted for you; see `EmoteSet.slotsUsed`. */
  slotsUsed: Scalars['Int']['output'];
  visibility: SetVisibility;
};

/** Who can find and open a set. */
export type SetVisibility =
  /** Anyone can open it, and it is listed on its owner's profile. New channel sets start here. */
  | 'LISTED'
  /**
   * Only its owner, its editors and staff can open it, except while a channel shows it:
   * a channel's active set is public. Channel sets only.
   */
  | 'PRIVATE'
  /** Anyone with its link (its ID) can open it; it isn't listed. Personal sets start here. */
  | 'UNLISTED';

/** Something staff sell: one or more badges and paints as a bundle. */
export type ShopItem = {
  __typename?: 'ShopItem';
  /** On sale from (RFC 3339); none: already. */
  activeFrom?: Maybe<Scalars['String']['output']>;
  /** On sale until (RFC 3339); none: no end. */
  activeTo?: Maybe<Scalars['String']['output']>;
  /** Can be bought now: visible, on sale, in stock, has cosmetics. */
  available: Scalars['Boolean']['output'];
  badges: Array<Badge>;
  createdAt: Scalars['String']['output'];
  /** ISO 4217, for example `EUR`. */
  currency: Scalars['String']['output'];
  /** Deleted: only staff see it, in lists with `all: true`. */
  deleted: Scalars['Boolean']['output'];
  description: Scalars['String']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  /** The caller owns every cosmetic in it (always false signed out). They can still gift it. */
  owned: Scalars['Boolean']['output'];
  /** The Paddle price (`pri_…`). Only for holders of `shop.manage`; null for everyone else. */
  paddlePriceId?: Maybe<Scalars['String']['output']>;
  paints: Array<Paint>;
  /** The price shown, in cents. Paddle's price (`paddlePriceId`) is what is charged. */
  priceCents: Scalars['Int']['output'];
  /** Lower first. */
  sortOrder: Scalars['Int']['output'];
  /** Copies left (pending checkouts of the last hour hold one each); none: no limit. */
  stockLeft?: Maybe<Scalars['Int']['output']>;
  /** Most copies sold; none: no limit. */
  stockLimit?: Maybe<Scalars['Int']['output']>;
  updatedAt: Scalars['String']['output'];
  /** Listed in the public shop (still only while on sale). */
  visible: Scalars['Boolean']['output'];
};

/** A whole item: `createShopItem` makes one, `updateShopItem` replaces every field. */
export type ShopItemInput = {
  /** RFC 3339. */
  activeFrom?: InputMaybe<Scalars['String']['input']>;
  /** RFC 3339, after `activeFrom`. */
  activeTo?: InputMaybe<Scalars['String']['input']>;
  /** 1 to 10 cosmetics in all. */
  badgeIds?: Array<Scalars['String']['input']>;
  /** ISO 4217, default `EUR`. */
  currency?: InputMaybe<Scalars['String']['input']>;
  /** Up to 500 characters; new lines allowed. */
  description?: InputMaybe<Scalars['String']['input']>;
  /** 1 to 64 characters. */
  name: Scalars['String']['input'];
  /** The Paddle price (`pri_…`). Without one, checkout says it isn't available. */
  paddlePriceId?: InputMaybe<Scalars['String']['input']>;
  paintIds?: Array<Scalars['String']['input']>;
  /** 0 to 100000 (1000.00). */
  priceCents: Scalars['Int']['input'];
  /** Default 0. */
  sortOrder?: InputMaybe<Scalars['Int']['input']>;
  /** 1 or more; none: no limit. */
  stockLimit?: InputMaybe<Scalars['Int']['input']>;
  /** Default false. */
  visible?: InputMaybe<Scalars['Boolean']['input']>;
};

/** A note staff keep on an account (`users.notes`). */
export type StaffNote = {
  __typename?: 'StaffNote';
  accountId: Scalars['String']['output'];
  /** Empty when the author's account is gone. */
  authorId?: Maybe<Scalars['String']['output']>;
  body: Scalars['String']['output'];
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
};

export type Subscription = {
  __typename?: 'Subscription';
  /**
   * Live updates for up to 500 topics: `channel:<platform>:<id>`, `set:<id>`,
   * `user:<platform>:<id>` (ADR 0005), and `cosmetics` (ADR 0015). The same events as the event WebSocket.
   */
  events: Event;
};


export type SubscriptionEventsArgs = {
  topics: Array<Scalars['String']['input']>;
};

/** Where a subscription is in time. */
export type SubscriptionState =
  | 'CURRENT'
  | 'FUTURE'
  | 'PAST';

/** Tags on an emote. */
export type TagLimits = {
  __typename?: 'TagLimits';
  /** The most tags one emote has. */
  max: Scalars['Int']['output'];
  /** The longest tag, in characters. */
  maxLen: Scalars['Int']['output'];
};

/** A text field's length in characters. */
export type TextLimit = {
  __typename?: 'TextLimit';
  max: Scalars['Int']['output'];
  min: Scalars['Int']['output'];
};

/** Where an emote transfer request stands (ADR 0014). */
export type TransferRequestState =
  | 'ACCEPTED'
  /** Withdrawn by the editor, or closed because the emote moved or went away. */
  | 'CANCELLED'
  | 'DECLINED'
  /** Nobody answered within 7 days. */
  | 'EXPIRED'
  /** Waiting for the emote's owner. */
  | 'PENDING';

/** How far back `TRENDING` counts channel adds. */
export type TrendingWindow =
  /** The last day. */
  | 'DAY'
  /** The last 30 days. */
  | 'MONTH'
  /** The last 7 days. The default. */
  | 'WEEK';

export type UpdateBadgeInput = {
  /** Another ready badge image. */
  imageId?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tooltip?: InputMaybe<Scalars['String']['input']>;
};

export type UpdateChannelSettingsInput = {
  /** One of the owner's channel sets. An empty string clears it. */
  activeSetId?: InputMaybe<Scalars['String']['input']>;
  allowPersonalEmotes?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateEmoteInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  /**
   * Retired: NSFW emotes aren't allowed. `false` is ignored; `true` is `nsfw_not_allowed`.
   * @deprecated NSFW emotes aren't allowed on cyex.tv. false is ignored; true is refused (nsfw_not_allowed).
   */
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  /**
   * `true` makes it private; `false` makes a private emote public (an unlisted one stays
   * unlisted). Prefer `visibility`.
   */
  private?: InputMaybe<Scalars['Boolean']['input']>;
  tags?: InputMaybe<Array<Scalars['String']['input']>>;
  /**
   * `PUBLIC` or `PRIVATE`. With `private` too, the two must agree (`bad_visibility`).
   * `UNLISTED` (and changing an unlisted emote) is for staff with `emotes.flags` or
   * `emotes.edit_any`: a moderator takes an emote off the listings, not its owner.
   */
  visibility?: InputMaybe<EmoteVisibility>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateRoleInput = {
  /** Its colour, `#rrggbb`. An empty string clears it. */
  color?: InputMaybe<Scalars['String']['input']>;
  /**
   * Replaces what it denies. A name in both lists is `bad_permissions`; one given in only one
   * of them leaves the other list (moving a name there from the other is fine).
   */
  denies?: InputMaybe<Array<Scalars['String']['input']>>;
  /** An empty string clears it. */
  description?: InputMaybe<Scalars['String']['input']>;
  /** Never show it publicly, and leave it out wherever roles rank people. */
  invisible?: InputMaybe<Scalars['Boolean']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  /** Replaces what it allows. */
  permissions?: InputMaybe<Array<Scalars['String']['input']>>;
  position?: InputMaybe<Scalars['Int']['input']>;
  /** Show it on its holders' public profiles. */
  public?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateSetEmoteInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateSetInput = {
  /**
   * 1 to the owner's `set_capacity_max` role limit (1.000 by default; 5 for a personal set),
   * never below the emotes the set holds; up to 1.000.000 with `sets.edit_any`.
   * `setCapacityRange` says what you may pick.
   */
  capacity?: InputMaybe<Scalars['Int']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  /**
   * Every set is public: changing it needs `sets.edit_any` (staff hiding a set). Personal sets
   * can't be `PRIVATE`; the global set is always listed.
   */
  visibility?: InputMaybe<SetVisibility>;
};

/** What to change on a webhook; what is left out stays. */
export type UpdateWebhookInput = {
  /** Turning it on again starts its failure count over. */
  enabled?: InputMaybe<Scalars['Boolean']['input']>;
  events?: InputMaybe<Array<EventType>>;
  url?: InputMaybe<Scalars['String']['input']>;
};

/**
 * Where an account stands with uploads (`uploadQuota`): how many of its emotes wait for review
 * and how many may, and its uploads this hour.
 */
export type UploadQuota = {
  __typename?: 'UploadQuota';
  /** The account uploads go to. */
  accountId: Scalars['String']['output'];
  /** How many times as wide as tall an emote may be (4, so 4:1). Fixed; a tall emote has no rule. */
  maxAspect: Scalars['Int']['output'];
  /** The most bytes one emote file may be (8 MB). Fixed, the same for everyone. */
  maxBytes: Scalars['Int']['output'];
  /**
   * The most frames a second (50). A frame shown for under 20 ms (a 0 or 10 ms GIF delay) is
   * shown for 100 ms, as browsers do, so an animation never runs faster. Fixed.
   */
  maxFps: Scalars['Int']['output'];
  /** The most frames an animated emote may have (1000). Fixed. */
  maxFrames: Scalars['Int']['output'];
  /** The longest side of an emote, in px (1000). Fixed. */
  maxSide: Scalars['Int']['output'];
  /** Its emotes waiting for review, ones still being processed included. */
  pending: Scalars['Int']['output'];
  /** How many may wait at once; an upload at this many is refused (`too_many_pending`). Null when the caller isn't capped (the owners). */
  pendingMax?: Maybe<Scalars['Int']['output']>;
  uploadsPerHour: Scalars['Int']['output'];
  /** Emotes it got in the last hour. */
  uploadsThisHour: Scalars['Int']['output'];
};

/** What one chatter shows, as IDs from the catalog. Only chatters showing something are listed. */
export type UserCosmetics = {
  __typename?: 'UserCosmetics';
  badgeId?: Maybe<Scalars['String']['output']>;
  paintId?: Maybe<Scalars['String']['output']>;
  platformUserId: Scalars['String']['output'];
};

/** A channel's webhook: where its events are sent. */
export type Webhook = {
  __typename?: 'Webhook';
  createdAt: Scalars['String']['output'];
  /** Why it is off: `manual` or `failures`. `None` while it is on. */
  disabledReason?: Maybe<Scalars['String']['output']>;
  /** Off: nothing is sent. It is turned off by hand, or after 5 deliveries in a row failed. */
  enabled: Scalars['Boolean']['output'];
  /** The event types it is sent. */
  events: Array<EventType>;
  /** Deliveries in a row that failed after every retry. */
  failedDeliveries: Scalars['Int']['output'];
  id: Scalars['String']['output'];
  url: Scalars['String']['output'];
};

/** One delivery of a webhook: what was sent and how it went. */
export type WebhookDelivery = {
  __typename?: 'WebhookDelivery';
  attempts: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  /** How long the last attempt took, in milliseconds. */
  durationMs?: Maybe<Scalars['Int']['output']>;
  /** What went wrong on the last attempt. */
  error?: Maybe<Scalars['String']['output']>;
  /** The outbox event's ID, as the event sockets send it. `None` for a test. */
  eventId?: Maybe<Scalars['String']['output']>;
  /** The event's type, or `webhook.test` for Send test. */
  eventType: Scalars['String']['output'];
  finishedAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  /** The receiver's HTTP status on the last attempt, if it answered. */
  responseStatus?: Maybe<Scalars['Int']['output']>;
  /** `pending`, `delivered` or `failed`. */
  state: Scalars['String']['output'];
};

export type StudioPaintFieldsFragment = { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null };

export type StudioPaintsQueryVariables = Exact<{ [key: string]: never; }>;


export type StudioPaintsQuery = { __typename?: 'Query', cosmetics: { __typename?: 'CosmeticCatalog', paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null }> } };

export type StudioCreatePaintMutationVariables = Exact<{
  input: PaintInput;
}>;


export type StudioCreatePaintMutation = { __typename?: 'Mutation', createPaint: { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null } };

export type StudioUpdatePaintMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: PaintInput;
}>;


export type StudioUpdatePaintMutation = { __typename?: 'Mutation', updatePaint: { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null } };

export type StudioCosmeticImageQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type StudioCosmeticImageQuery = { __typename?: 'Query', cosmeticImage?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null };

export type SetDisplayPlatformMutationVariables = Exact<{
  platform: Platform;
}>;


export type SetDisplayPlatformMutation = { __typename?: 'Mutation', setDisplayPlatform: { __typename?: 'Me', id: string, displayPlatform?: Platform | null } };

export type SignOutEverywhereMutationVariables = Exact<{ [key: string]: never; }>;


export type SignOutEverywhereMutation = { __typename?: 'Mutation', signOutEverywhere: number };

export type UnlinkMutationVariables = Exact<{
  platform: Platform;
}>;


export type UnlinkMutation = { __typename?: 'Mutation', unlinkIdentity: { __typename?: 'Me', id: string } };

export type MyNotificationsQueryVariables = Exact<{ [key: string]: never; }>;


export type MyNotificationsQuery = { __typename?: 'Query', myNotifications: Array<{ __typename?: 'Notification', id: string, kind: string, payload: string, createdAt: string, read: boolean }> };

export type MarkReadMutationVariables = Exact<{
  ids?: InputMaybe<Array<Scalars['String']['input']> | Scalars['String']['input']>;
}>;


export type MarkReadMutation = { __typename?: 'Mutation', markNotificationsRead: number };

export type UnreadCountQueryVariables = Exact<{ [key: string]: never; }>;


export type UnreadCountQuery = { __typename?: 'Query', unreadNotificationCount: number };

export type MySessionsQueryVariables = Exact<{ [key: string]: never; }>;


export type MySessionsQuery = { __typename?: 'Query', mySessions: Array<{ __typename?: 'AccountSession', id: string, kind: SessionKind, device: string, createdAt: string, lastSeenAt: string, expiresAt: string, current: boolean }> };

export type MyConsoleTokenQueryVariables = Exact<{ [key: string]: never; }>;


export type MyConsoleTokenQuery = { __typename?: 'Query', myConsoleToken?: { __typename?: 'AccountSession', id: string, createdAt: string, expiresAt: string } | null };

export type RevokeSessionMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type RevokeSessionMutation = { __typename?: 'Mutation', revokeSession: boolean };

export type RevokeOtherSessionsMutationVariables = Exact<{ [key: string]: never; }>;


export type RevokeOtherSessionsMutation = { __typename?: 'Mutation', revokeOtherSessions: number };

export type ExportMyDataMutationVariables = Exact<{ [key: string]: never; }>;


export type ExportMyDataMutation = { __typename?: 'Mutation', exportMyData: string };

export type DeleteAccountMutationVariables = Exact<{
  confirm: Scalars['String']['input'];
}>;


export type DeleteAccountMutation = { __typename?: 'Mutation', deleteAccount: boolean };

export type PendingDeletionQueryVariables = Exact<{ [key: string]: never; }>;


export type PendingDeletionQuery = { __typename?: 'Query', pendingDeletion?: { __typename?: 'PendingDeletion', requestedAt: string, eraseAfter: string, displayName?: string | null, avatar?: string | null } | null };

export type RestoreAccountMutationVariables = Exact<{ [key: string]: never; }>;


export type RestoreAccountMutation = { __typename?: 'Mutation', restoreAccount: boolean };

export type MyRolesQueryVariables = Exact<{ [key: string]: never; }>;


export type MyRolesQuery = { __typename?: 'Query', me: { __typename?: 'Me', id: string, roles: Array<{ __typename?: 'OwnRole', id: string, name: string, public: boolean, invisible: boolean, color?: string | null }> } };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { __typename?: 'Query', me: { __typename?: 'Me', id: string, createdAt: string, permissions: Array<string>, displayPlatform?: Platform | null, identities: Array<{ __typename?: 'Identity', platform: Platform, platformUserId: string, login: string, displayName: string, avatar?: string | null }> } };

export type AccountActivityQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  filter?: InputMaybe<ActivityFilter>;
}>;


export type AccountActivityQuery = { __typename?: 'Query', accountActivityPage: { __typename?: 'ActivityPage', next?: string | null, entries: Array<{ __typename?: 'ActivityEntry', id: string, kind: ActivityKind, fromName?: string | null, toName?: string | null, createdAt: string, actor: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, editor?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, set?: { __typename?: 'ActivitySet', id: string, name: string } | null, emote?: { __typename?: 'ActivityEmote', id: string, name: string, host: string, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null, badge?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null, paint?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null }> } };

export type EmoteActivityQueryVariables = Exact<{
  emoteId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
}>;


export type EmoteActivityQuery = { __typename?: 'Query', emoteActivityPage: { __typename?: 'ActivityPage', next?: string | null, entries: Array<{ __typename?: 'ActivityEntry', id: string, kind: ActivityKind, fromName?: string | null, toName?: string | null, createdAt: string, actor: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, set?: { __typename?: 'ActivitySet', id: string, name: string } | null, emote?: { __typename?: 'ActivityEmote', id: string, name: string, host: string, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null, badge?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null, paint?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null }> } };

export type UserQueryVariables = Exact<{
  platform: Platform;
  login: Scalars['String']['input'];
}>;


export type UserQuery = { __typename?: 'Query', channelByLogin?: { __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } | null };

export type UserDetailsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
  status: Scalars['Boolean']['input'];
  roles: Scalars['Boolean']['input'];
}>;


export type UserDetailsQuery = { __typename?: 'Query', accountStatus?: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null }, accountRoles?: Array<{ __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, color?: string | null, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type AccountPermissionsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountPermissionsQuery = { __typename?: 'Query', accountPermissions: Array<{ __typename?: 'PermissionDecision', permission: string, granted: boolean, effect?: PermissionEffect | null, roleId?: string | null, roleName?: string | null, owner: boolean }> };

export type AllRolesQueryVariables = Exact<{ [key: string]: never; }>;


export type AllRolesQuery = { __typename?: 'Query', roles: Array<{ __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, color?: string | null, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type PlaceRestrictionMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  input: RestrictionInput;
}>;


export type PlaceRestrictionMutation = { __typename?: 'Mutation', placeRestriction: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null } };

export type LiftRestrictionMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  kind: RestrictionKind;
  reason?: InputMaybe<Scalars['String']['input']>;
}>;


export type LiftRestrictionMutation = { __typename?: 'Mutation', liftRestriction: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null } };

export type AccountRestrictionsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountRestrictionsQuery = { __typename?: 'Query', accountRestrictions: Array<{ __typename?: 'Restriction', id: string, kind: RestrictionKind, reason?: string | null, endsAt?: string | null, createdBy?: string | null, createdAt: string, liftedAt?: string | null, liftedBy?: string | null, active: boolean }> };

export type AssignRoleMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
}>;


export type AssignRoleMutation = { __typename?: 'Mutation', assignRole: boolean };

export type UnassignRoleMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
}>;


export type UnassignRoleMutation = { __typename?: 'Mutation', unassignRole: boolean };

export type AuditQueryVariables = Exact<{
  targetKind?: InputMaybe<Scalars['String']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
  action?: InputMaybe<Scalars['String']['input']>;
  since?: InputMaybe<Scalars['String']['input']>;
  until?: InputMaybe<Scalars['String']['input']>;
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AuditQuery = { __typename?: 'Query', auditLog: Array<{ __typename?: 'AuditEntry', id: string, actorId?: string | null, action: string, targetKind: string, targetId: string, before?: string | null, after?: string | null, createdAt: string }> };

export type ChannelQueryVariables = Exact<{
  platform: Platform;
  login: Scalars['String']['input'];
}>;


export type ChannelQuery = { __typename?: 'Query', channelByLogin?: { __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } | null };

export type CosmeticImageFieldsFragment = { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> };

export type BadgeFieldsFragment = { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } };

export type PaintFieldsFragment = { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null };

export type CosmeticsQueryVariables = Exact<{ [key: string]: never; }>;


export type CosmeticsQuery = { __typename?: 'Query', cosmetics: { __typename?: 'CosmeticCatalog', badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type UserCosmeticsQueryVariables = Exact<{
  platform: Platform;
  userIds: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type UserCosmeticsQuery = { __typename?: 'Query', userCosmetics: Array<{ __typename?: 'UserCosmetics', platformUserId: string, badgeId?: string | null, paintId?: string | null }> };

export type AccountCosmeticsQueryVariables = Exact<{
  accountIds: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type AccountCosmeticsQuery = { __typename?: 'Query', accountCosmetics: Array<{ __typename?: 'AccountCosmetics', accountId: string, badge?: { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } } | null, paint?: { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null } | null }> };

export type AccountProfileQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountProfileQuery = { __typename?: 'Query', accountProfile?: { __typename?: 'AccountProfile', roleColor?: string | null, createdAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }>, roles: Array<{ __typename?: 'PublicRole', id: string, name: string, color?: string | null }>, activeSets: Array<{ __typename?: 'ActiveChannelSet', platform: Platform, login: string, setId: string, setName: string }>, editors: Array<{ __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }> } | null };

export type MyCosmeticsQueryVariables = Exact<{
  accountId?: InputMaybe<Scalars['String']['input']>;
}>;


export type MyCosmeticsQuery = { __typename?: 'Query', myCosmetics: { __typename?: 'OwnedCosmetics', badgeId?: string | null, paintId?: string | null, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type SelectCosmeticsMutationVariables = Exact<{
  badgeId?: InputMaybe<Scalars['String']['input']>;
  paintId?: InputMaybe<Scalars['String']['input']>;
  ownerId?: InputMaybe<Scalars['String']['input']>;
}>;


export type SelectCosmeticsMutation = { __typename?: 'Mutation', selectCosmetics: { __typename?: 'OwnedCosmetics', badgeId?: string | null, paintId?: string | null } };

export type CosmeticImageQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type CosmeticImageQuery = { __typename?: 'Query', cosmeticImage?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null };

export type EntitlementsQueryVariables = Exact<{
  kind: CosmeticKind;
  accountId?: InputMaybe<Scalars['String']['input']>;
  cosmeticId?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type EntitlementsQuery = { __typename?: 'Query', cosmeticEntitlements: { __typename?: 'EntitlementPage', total: number, items: Array<{ __typename?: 'Entitlement', kind: CosmeticKind, cosmeticId: string, source: EntitlementSource, reason?: string | null, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }> } };

export type CreateBadgeMutationVariables = Exact<{
  input: CreateBadgeInput;
}>;


export type CreateBadgeMutation = { __typename?: 'Mutation', createBadge: { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } } };

export type UpdateBadgeMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateBadgeInput;
}>;


export type UpdateBadgeMutation = { __typename?: 'Mutation', updateBadge: { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } } };

export type RemakeCosmeticImageMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type RemakeCosmeticImageMutation = { __typename?: 'Mutation', remakeCosmeticImage: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } };

export type DeleteBadgeMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteBadgeMutation = { __typename?: 'Mutation', deleteBadge: boolean };

export type DeletePaintMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeletePaintMutation = { __typename?: 'Mutation', deletePaint: boolean };

export type GrantCosmeticMutationVariables = Exact<{
  input: GrantCosmeticInput;
}>;


export type GrantCosmeticMutation = { __typename?: 'Mutation', grantCosmetic: { __typename?: 'Entitlement', cosmeticId: string } };

export type RevokeCosmeticMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  kind: CosmeticKind;
  cosmeticId: Scalars['String']['input'];
  reason?: InputMaybe<Scalars['String']['input']>;
}>;


export type RevokeCosmeticMutation = { __typename?: 'Mutation', revokeCosmetic: boolean };

export type MyApiKeysQueryVariables = Exact<{ [key: string]: never; }>;


export type MyApiKeysQuery = { __typename?: 'Query', myApiKeys: Array<{ __typename?: 'ApiKey', id: string, name: string, prefix: string, createdAt: string, lastUsedAt?: string | null, usage: Array<{ __typename?: 'ApiKeyDay', day: string, requests: number }> }> };

export type CreateApiKeyMutationVariables = Exact<{
  name: Scalars['String']['input'];
}>;


export type CreateApiKeyMutation = { __typename?: 'Mutation', createApiKey: { __typename?: 'CreatedApiKey', key: string, apiKey: { __typename?: 'ApiKey', id: string, name: string, prefix: string, createdAt: string, lastUsedAt?: string | null, usage: Array<{ __typename?: 'ApiKeyDay', day: string, requests: number }> } } };

export type RevokeApiKeyMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type RevokeApiKeyMutation = { __typename?: 'Mutation', revokeApiKey: boolean };

export type WebhookFieldsFragment = { __typename?: 'Webhook', id: string, url: string, events: Array<EventType>, enabled: boolean, disabledReason?: string | null, failedDeliveries: number, createdAt: string };

export type ChannelWebhooksQueryVariables = Exact<{
  platform: Platform;
  id: Scalars['String']['input'];
}>;


export type ChannelWebhooksQuery = { __typename?: 'Query', channelWebhooks: Array<{ __typename?: 'Webhook', id: string, url: string, events: Array<EventType>, enabled: boolean, disabledReason?: string | null, failedDeliveries: number, createdAt: string }> };

export type CreateWebhookMutationVariables = Exact<{
  platform: Platform;
  id: Scalars['String']['input'];
  input: CreateWebhookInput;
}>;


export type CreateWebhookMutation = { __typename?: 'Mutation', createWebhook: { __typename?: 'CreatedWebhook', secret: string, webhook: { __typename?: 'Webhook', id: string, url: string, events: Array<EventType>, enabled: boolean, disabledReason?: string | null, failedDeliveries: number, createdAt: string } } };

export type UpdateWebhookMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateWebhookInput;
}>;


export type UpdateWebhookMutation = { __typename?: 'Mutation', updateWebhook: { __typename?: 'Webhook', id: string, url: string, events: Array<EventType>, enabled: boolean, disabledReason?: string | null, failedDeliveries: number, createdAt: string } };

export type DeleteWebhookMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteWebhookMutation = { __typename?: 'Mutation', deleteWebhook: boolean };

export type RotateWebhookSecretMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type RotateWebhookSecretMutation = { __typename?: 'Mutation', rotateWebhookSecret: { __typename?: 'CreatedWebhook', secret: string, webhook: { __typename?: 'Webhook', id: string, url: string, events: Array<EventType>, enabled: boolean, disabledReason?: string | null, failedDeliveries: number, createdAt: string } } };

export type TestWebhookMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type TestWebhookMutation = { __typename?: 'Mutation', testWebhook: { __typename?: 'WebhookDelivery', id: string, state: string } };

export type WebhookDeliveriesQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type WebhookDeliveriesQuery = { __typename?: 'Query', webhookDeliveries: Array<{ __typename?: 'WebhookDelivery', id: string, eventType: string, eventId?: string | null, state: string, attempts: number, responseStatus?: number | null, error?: string | null, durationMs?: number | null, createdAt: string, finishedAt?: string | null }> };

export type AccountEditorFieldsFragment = { __typename?: 'AccountEditor', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null };

export type EditorsQueryVariables = Exact<{ [key: string]: never; }>;


export type EditorsQuery = { __typename?: 'Query', accountEditors: Array<{ __typename?: 'AccountEditor', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }>, mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, owner?: { __typename?: 'AccountSummary', id: string } | null }> };

export type AddAccountEditorMutationVariables = Exact<{
  input: AddAccountEditorInput;
}>;


export type AddAccountEditorMutation = { __typename?: 'Mutation', addAccountEditor: { __typename?: 'AccountEditor', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } };

export type UpdateAccountEditorMutationVariables = Exact<{
  ownerId?: InputMaybe<Scalars['String']['input']>;
  accountId: Scalars['String']['input'];
  permissions: Array<EditorPermission> | EditorPermission;
  setIds?: InputMaybe<Array<Scalars['String']['input']> | Scalars['String']['input']>;
}>;


export type UpdateAccountEditorMutation = { __typename?: 'Mutation', updateAccountEditor: { __typename?: 'AccountEditor', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } };

export type RemoveAccountEditorMutationVariables = Exact<{
  ownerId?: InputMaybe<Scalars['String']['input']>;
  accountId: Scalars['String']['input'];
}>;


export type RemoveAccountEditorMutation = { __typename?: 'Mutation', removeAccountEditor: boolean };

export type ReportEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type ReportEmoteMutation = { __typename?: 'Mutation', reportEmote: { __typename?: 'Report', id: string } };

export type MySetNamesQueryVariables = Exact<{ [key: string]: never; }>;


export type MySetNamesQuery = { __typename?: 'Query', mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, emotes: Array<{ __typename?: 'SetEmote', emote: { __typename?: 'Emote', id: string } }> }> };

export type SearchByOwnerQueryVariables = Exact<{
  owner: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  sort?: InputMaybe<EmoteSort>;
  after?: InputMaybe<Scalars['String']['input']>;
}>;


export type SearchByOwnerQuery = { __typename?: 'Query', searchEmotes: { __typename?: 'SearchResults', total: number, page: number, perPage: number, next?: string | null, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type AdminAccountQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AdminAccountQuery = { __typename?: 'Query', adminAccount: { __typename?: 'AdminAccount', id: string, displayName: string, avatar?: string | null, createdAt: string, uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null, identities: Array<{ __typename?: 'Identity', platform: Platform, platformUserId: string, login: string, displayName: string, avatar?: string | null }> } };

export type TransferEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
  toAccountId: Scalars['String']['input'];
}>;


export type TransferEmoteMutation = { __typename?: 'Mutation', transferEmote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type AccountEmotesQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  after?: InputMaybe<Scalars['String']['input']>;
}>;


export type AccountEmotesQuery = { __typename?: 'Query', accountEmotes: { __typename?: 'EmotePage', total: number, next?: string | null, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type AccountReportsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountReportsQuery = { __typename?: 'Query', accountReports: { __typename?: 'AccountReports', about: Array<{ __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, resolution?: string | null, createdAt: string, resolvedAt?: string | null, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }>, by: Array<{ __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, resolution?: string | null, createdAt: string, resolvedAt?: string | null, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }> } };

export type PublicAccountQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type PublicAccountQuery = { __typename?: 'Query', account?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null };

export type EmoteChannelsQueryVariables = Exact<{
  id: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
  offset?: InputMaybe<Scalars['Int']['input']>;
}>;


export type EmoteChannelsQuery = { __typename?: 'Query', emoteChannels: Array<{ __typename?: 'Channel', platform: Platform, login: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }> };

export type EmoteReviewApprovalsMutationVariables = Exact<{
  ids: Array<Scalars['String']['input']> | Scalars['String']['input'];
  input: ApprovalsInput;
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
}>;


export type EmoteReviewApprovalsMutation = { __typename?: 'Mutation', reviewApprovals: { __typename?: 'BatchOutcome', done: Array<string>, failed: Array<{ __typename?: 'BatchFailure', id: string, code: string, message: string }> } };

export type EmoteFieldsFragment = { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> };

export type SearchQueryVariables = Exact<{
  query: Scalars['String']['input'];
  animated?: InputMaybe<Scalars['Boolean']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  sort?: InputMaybe<EmoteSort>;
  flags?: InputMaybe<Array<EmoteFlag> | EmoteFlag>;
  tags?: InputMaybe<Array<Scalars['String']['input']> | Scalars['String']['input']>;
  exact?: InputMaybe<Scalars['Boolean']['input']>;
  window?: InputMaybe<TrendingWindow>;
  after?: InputMaybe<Scalars['String']['input']>;
}>;


export type SearchQuery = { __typename?: 'Query', searchEmotes: { __typename?: 'SearchResults', total: number, page: number, perPage: number, next?: string | null, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type FindPeopleQueryVariables = Exact<{
  query: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  after?: InputMaybe<Scalars['String']['input']>;
}>;


export type FindPeopleQuery = { __typename?: 'Query', searchAccounts: { __typename?: 'AccountResults', total: number, page: number, perPage: number, next?: string | null, items: Array<{ __typename?: 'AccountHit', account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> } };

export type EmoteQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type EmoteQuery = { __typename?: 'Query', emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type EmoteTrendingQueryVariables = Exact<{
  id: Scalars['String']['input'];
  window?: InputMaybe<TrendingWindow>;
}>;


export type EmoteTrendingQuery = { __typename?: 'Query', emoteTrending?: { __typename?: 'EmoteTrending', rank: number, channels: number, window: TrendingWindow } | null };

export type MyChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type MyChannelsQuery = { __typename?: 'Query', myChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string } }>, mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, owner?: { __typename?: 'AccountSummary', id: string } | null }>, editorChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string } }>, editorSets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, owner?: { __typename?: 'AccountSummary', id: string } | null }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string } }> };

export type UpdateChannelMutationVariables = Exact<{
  platform: Platform;
  id: Scalars['String']['input'];
  input: UpdateChannelSettingsInput;
}>;


export type UpdateChannelMutation = { __typename?: 'Mutation', updateChannelSettings: { __typename?: 'Channel', activeSetId?: string | null, allowPersonalEmotes: boolean } };

export type MyEmotesQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type MyEmotesQuery = { __typename?: 'Query', myEmotes: { __typename?: 'EmotePage', total: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type UpdateEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateEmoteInput;
}>;


export type UpdateEmoteMutation = { __typename?: 'Mutation', updateEmote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type DeleteEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteEmoteMutation = { __typename?: 'Mutation', deleteEmote: boolean };

export type PersonalListsQueryVariables = Exact<{ [key: string]: never; }>;


export type PersonalListsQuery = { __typename?: 'Query', favouriteEmoteIds: Array<string>, hiddenEmotes: Array<{ __typename?: 'HiddenEmote', emoteId: string }>, hiddenCreators: Array<{ __typename?: 'HiddenCreator', accountId: string }> };

export type HiddenListsQueryVariables = Exact<{ [key: string]: never; }>;


export type HiddenListsQuery = { __typename?: 'Query', hiddenEmotes: Array<{ __typename?: 'HiddenEmote', emoteId: string, hiddenAt: string, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }>, hiddenCreators: Array<{ __typename?: 'HiddenCreator', accountId: string, hiddenAt: string, account?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null }> };

export type MyFavouritesQueryVariables = Exact<{
  query?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type MyFavouritesQuery = { __typename?: 'Query', myFavourites: { __typename?: 'EmotePage', total: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type HideEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type HideEmoteMutation = { __typename?: 'Mutation', hideEmote: boolean };

export type UnhideEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type UnhideEmoteMutation = { __typename?: 'Mutation', unhideEmote: boolean };

export type HideCreatorMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type HideCreatorMutation = { __typename?: 'Mutation', hideCreator: boolean };

export type UnhideCreatorMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type UnhideCreatorMutation = { __typename?: 'Mutation', unhideCreator: boolean };

export type FavouriteEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type FavouriteEmoteMutation = { __typename?: 'Mutation', favouriteEmote: boolean };

export type UnfavouriteEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type UnfavouriteEmoteMutation = { __typename?: 'Mutation', unfavouriteEmote: boolean };

export type AccountRolesQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountRolesQuery = { __typename?: 'Query', accountRoles: Array<{ __typename?: 'Role', id: string, name: string, position: number, color?: string | null }> };

export type AccountEditorsQueryVariables = Exact<{
  accountId?: InputMaybe<Scalars['String']['input']>;
}>;


export type AccountEditorsQuery = { __typename?: 'Query', accountEditors: Array<{ __typename?: 'AccountEditor', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }> };

export type EditorOfQueryVariables = Exact<{ [key: string]: never; }>;


export type EditorOfQuery = { __typename?: 'Query', editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, allSets: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }> };

export type AccountSetsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountSetsQuery = { __typename?: 'Query', accountSets: Array<{ __typename?: 'SetSummary', id: string, name: string, kind: SetKind, capacity: number, slotsUsed: number, active: boolean, preview: Array<{ __typename?: 'SetEmote', emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ProfileMineQueryVariables = Exact<{ [key: string]: never; }>;


export type ProfileMineQuery = { __typename?: 'Query', myChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null }>, mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ProfileEditedQueryVariables = Exact<{ [key: string]: never; }>;


export type ProfileEditedQuery = { __typename?: 'Query', editorChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, account: { __typename?: 'AccountSummary', id: string } }>, editorSets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ReportFieldsFragment = { __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, resolution?: string | null, createdAt: string, resolvedAt?: string | null, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type ReportQueueQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type ReportQueueQuery = { __typename?: 'Query', reportQueue: { __typename?: 'ReportQueue', total: number, items: Array<{ __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, createdAt: string, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }> } };

export type ResolveReportMutationVariables = Exact<{
  id: Scalars['String']['input'];
  action: ReportAction;
  note?: InputMaybe<Scalars['String']['input']>;
}>;


export type ResolveReportMutation = { __typename?: 'Mutation', resolveReport: { __typename?: 'Report', id: string, state: ReportState } };

export type ReviewEmotesMutationVariables = Exact<{
  ids: Array<Scalars['String']['input']> | Scalars['String']['input'];
  verdict: ReviewVerdict;
  visibility?: InputMaybe<EmoteVisibility>;
  reason?: InputMaybe<Scalars['String']['input']>;
}>;


export type ReviewEmotesMutation = { __typename?: 'Mutation', reviewEmotes: { __typename?: 'BatchOutcome', done: Array<string>, failed: Array<{ __typename?: 'BatchFailure', id: string, code: string, message: string }> } };

export type ReviewApprovalsMutationVariables = Exact<{
  ids: Array<Scalars['String']['input']> | Scalars['String']['input'];
  input: ApprovalsInput;
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
}>;


export type ReviewApprovalsMutation = { __typename?: 'Mutation', reviewApprovals: { __typename?: 'BatchOutcome', done: Array<string>, failed: Array<{ __typename?: 'BatchFailure', id: string, code: string, message: string }> } };

export type RequeueEmotesMutationVariables = Exact<{
  ids: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type RequeueEmotesMutation = { __typename?: 'Mutation', requeueEmotes: { __typename?: 'BatchOutcome', done: Array<string>, failed: Array<{ __typename?: 'BatchFailure', id: string, code: string, message: string }> } };

export type SetEmoteFlagsMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: EmoteFlagsInput;
}>;


export type SetEmoteFlagsMutation = { __typename?: 'Mutation', setEmoteFlags: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type DismissReportsMutationVariables = Exact<{
  ids: Array<Scalars['String']['input']> | Scalars['String']['input'];
  note?: InputMaybe<Scalars['String']['input']>;
}>;


export type DismissReportsMutation = { __typename?: 'Mutation', dismissReports: { __typename?: 'BatchOutcome', done: Array<string>, failed: Array<{ __typename?: 'BatchFailure', id: string, code: string, message: string }> } };

export type EmoteDuplicatesQueryVariables = Exact<{
  emoteId: Scalars['String']['input'];
}>;


export type EmoteDuplicatesQuery = { __typename?: 'Query', emoteDuplicates: Array<{ __typename?: 'DuplicateCandidate', sameFile: boolean, similarity: number, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> };

export type ReviewReasonsQueryVariables = Exact<{ [key: string]: never; }>;


export type ReviewReasonsQuery = { __typename?: 'Query', reviewReasons: Array<string> };

export type SetReviewReasonsMutationVariables = Exact<{
  reasons: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type SetReviewReasonsMutation = { __typename?: 'Mutation', setReviewReasons: Array<string> };

export type AccountsByLoginQueryVariables = Exact<{
  logins: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type AccountsByLoginQuery = { __typename?: 'Query', accountsByLogin: Array<{ __typename?: 'LoginMatch', login: string, problem?: string | null, account?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null }> };

export type EmotesWithSameFileQueryVariables = Exact<{
  sha256: Scalars['String']['input'];
}>;


export type EmotesWithSameFileQuery = { __typename?: 'Query', emotesWithSameFile: Array<{ __typename?: 'Emote', id: string, name: string, owner?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }> };

export type AdminCountsQueryVariables = Exact<{
  review: Scalars['Boolean']['input'];
  reports: Scalars['Boolean']['input'];
}>;


export type AdminCountsQuery = { __typename?: 'Query', reviewQueue?: { __typename?: 'ReviewQueue', total: number }, reportQueue?: { __typename?: 'ReportQueue', total: number }, globalSet: { __typename?: 'EmoteSet', id: string } };

export type ReviewQueueQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  filter?: InputMaybe<ReviewQueueFilter>;
}>;


export type ReviewQueueQuery = { __typename?: 'Query', reviewQueue: { __typename?: 'ReviewQueue', total: number, counts: { __typename?: 'ReviewCounts', public: number, limited: number, personal: number }, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type ApproveMutationVariables = Exact<{
  id: Scalars['String']['input'];
  visibility?: InputMaybe<EmoteVisibility>;
}>;


export type ApproveMutation = { __typename?: 'Mutation', approveEmote: { __typename?: 'Emote', id: string, state: EmoteState, visibility: EmoteVisibility } };

export type EmoteHistoryQueryVariables = Exact<{
  emoteId: Scalars['String']['input'];
}>;


export type EmoteHistoryQuery = { __typename?: 'Query', emoteHistory: Array<{ __typename?: 'AuditEntry', id: string, actorId?: string | null, action: string, before?: string | null, after?: string | null, createdAt: string }> };

export type EmoteNotesQueryVariables = Exact<{
  emoteId: Scalars['String']['input'];
}>;


export type EmoteNotesQuery = { __typename?: 'Query', emoteNotes: Array<{ __typename?: 'EmoteNote', id: string, authorId?: string | null, body: string, createdAt: string }> };

export type AddEmoteNoteMutationVariables = Exact<{
  emoteId: Scalars['String']['input'];
  body: Scalars['String']['input'];
}>;


export type AddEmoteNoteMutation = { __typename?: 'Mutation', addEmoteNote: { __typename?: 'EmoteNote', id: string, authorId?: string | null, body: string, createdAt: string } };

export type DeleteEmoteNoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteEmoteNoteMutation = { __typename?: 'Mutation', deleteEmoteNote: boolean };

export type RejectMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type RejectMutation = { __typename?: 'Mutation', rejectEmote: { __typename?: 'Emote', id: string, state: EmoteState } };

export type RoleFieldsFragment = { __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, color?: string | null, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean };

export type RolesQueryVariables = Exact<{ [key: string]: never; }>;


export type RolesQuery = { __typename?: 'Query', permissions: Array<string>, roles: Array<{ __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, color?: string | null, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type RoleLimitFieldsFragment = { __typename?: 'RoleLimit', key: string, group: LimitGroup, default: number, max: number, windowSeconds?: number | null, value?: number | null };

export type LimitDefaultsQueryVariables = Exact<{ [key: string]: never; }>;


export type LimitDefaultsQuery = { __typename?: 'Query', limitDefaults: Array<{ __typename?: 'RoleLimit', key: string, group: LimitGroup, default: number, max: number, windowSeconds?: number | null, value?: number | null }> };

export type RoleLimitsQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type RoleLimitsQuery = { __typename?: 'Query', roleLimits: Array<{ __typename?: 'RoleLimit', key: string, group: LimitGroup, default: number, max: number, windowSeconds?: number | null, value?: number | null }> };

export type SetRoleLimitsMutationVariables = Exact<{
  id: Scalars['String']['input'];
  limits: Array<RoleLimitInput> | RoleLimitInput;
}>;


export type SetRoleLimitsMutation = { __typename?: 'Mutation', setRoleLimits: Array<{ __typename?: 'RoleLimit', key: string, group: LimitGroup, default: number, max: number, windowSeconds?: number | null, value?: number | null }> };

export type UploadQuotaQueryVariables = Exact<{
  ownerId?: InputMaybe<Scalars['String']['input']>;
}>;


export type UploadQuotaQuery = { __typename?: 'Query', uploadQuota: { __typename?: 'UploadQuota', accountId: string, pending: number, pendingMax?: number | null, uploadsThisHour: number, uploadsPerHour: number, maxBytes: number, maxSide: number, maxFrames: number, maxFps: number, maxAspect: number } };

export type MediaLimitFieldsFragment = { __typename?: 'MediaLimits', maxBytes: number, maxSide: number, maxFrames: number, maxFps?: number | null, maxTotalPixels: number, maxDurationMs?: number | null, minSide?: number | null, maxAspect?: number | null, maxServedSide: number };

export type PublicLimitsQueryVariables = Exact<{ [key: string]: never; }>;


export type PublicLimitsQuery = { __typename?: 'Query', limits: { __typename?: 'PublicLimits', emote: { __typename?: 'MediaLimits', maxBytes: number, maxSide: number, maxFrames: number, maxFps?: number | null, maxTotalPixels: number, maxDurationMs?: number | null, minSide?: number | null, maxAspect?: number | null, maxServedSide: number }, badge: { __typename?: 'MediaLimits', maxBytes: number, maxSide: number, maxFrames: number, maxFps?: number | null, maxTotalPixels: number, maxDurationMs?: number | null, minSide?: number | null, maxAspect?: number | null, maxServedSide: number }, paint: { __typename?: 'MediaLimits', maxBytes: number, maxSide: number, maxFrames: number, maxFps?: number | null, maxTotalPixels: number, maxDurationMs?: number | null, minSide?: number | null, maxAspect?: number | null, maxServedSide: number }, names: { __typename?: 'NameLimits', emote: { __typename?: 'TextLimit', min: number, max: number }, set: { __typename?: 'TextLimit', min: number, max: number }, badge: { __typename?: 'TextLimit', min: number, max: number }, paint: { __typename?: 'TextLimit', min: number, max: number }, tooltip: { __typename?: 'TextLimit', min: number, max: number } }, tags: { __typename?: 'TagLimits', max: number, maxLen: number }, rates: Array<{ __typename?: 'RateInfo', key: string, count: number, windowSeconds: number }>, account: Array<{ __typename?: 'EffectiveLimit', key: string, default: number, effective: number, windowSeconds?: number | null }> } };

export type CreateRoleMutationVariables = Exact<{
  input: CreateRoleInput;
}>;


export type CreateRoleMutation = { __typename?: 'Mutation', createRole: { __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, color?: string | null, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean } };

export type UpdateRoleMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateRoleInput;
}>;


export type UpdateRoleMutation = { __typename?: 'Mutation', updateRole: { __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, color?: string | null, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean } };

export type DeleteRoleMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteRoleMutation = { __typename?: 'Mutation', deleteRole: boolean };

export type QuickSearchQueryVariables = Exact<{
  query: Scalars['String']['input'];
}>;


export type QuickSearchQuery = { __typename?: 'Query', quickSearch: { __typename?: 'QuickSearch', emotes: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }>, users: Array<{ __typename?: 'AccountHit', account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> } };

export type EditableChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type EditableChannelsQuery = { __typename?: 'Query', myEditableChannels: Array<{ __typename?: 'EditableChannel', role: ChannelRole, channel: { __typename?: 'Channel', platform: Platform, login: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } }> };

export type SetFieldsFragment = { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> };

export type MySetsQueryVariables = Exact<{ [key: string]: never; }>;


export type MySetsQuery = { __typename?: 'Query', mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }>, editorSets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string, displayName: string } }> };

export type EditSetQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type EditSetQuery = { __typename?: 'Query', set?: { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, slotsUsed: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } | null, setEditors: Array<{ __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, account: { __typename?: 'AccountSummary', id: string } }>, myChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, account: { __typename?: 'AccountSummary', id: string } }>, editorChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, account: { __typename?: 'AccountSummary', id: string } }> };

export type SetEmotesQueryVariables = Exact<{
  id: Scalars['String']['input'];
  query?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type SetEmotesQuery = { __typename?: 'Query', setEmotes: { __typename?: 'SetEmotePage', total: number, items: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } };

export type SetMembersQueryVariables = Exact<{
  id: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type SetMembersQuery = { __typename?: 'Query', setEmotes: { __typename?: 'SetEmotePage', total: number, items: Array<{ __typename?: 'SetEmote', name: string, emote: { __typename?: 'Emote', id: string } }> } };

export type CreateSetMutationVariables = Exact<{
  input: CreateSetInput;
}>;


export type CreateSetMutation = { __typename?: 'Mutation', createSet: { __typename?: 'EmoteSet', id: string } };

export type UpdateSetMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateSetInput;
}>;


export type UpdateSetMutation = { __typename?: 'Mutation', updateSet: { __typename?: 'EmoteSet', id: string, name: string, visibility: SetVisibility, capacity: number } };

export type DeleteSetMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteSetMutation = { __typename?: 'Mutation', deleteSet: boolean };

export type SetCapacityRangeQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type SetCapacityRangeQuery = { __typename?: 'Query', setCapacityRange?: { __typename?: 'SetCapacityRange', min: number, max: number, ownerMax: number, used: number, staff: boolean } | null };

export type SetMenuQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type SetMenuQuery = { __typename?: 'Query', setCapacityRange?: { __typename?: 'SetCapacityRange', min: number, max: number, ownerMax: number, used: number, staff: boolean } | null, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, account: { __typename?: 'AccountSummary', id: string } }>, myChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, account: { __typename?: 'AccountSummary', id: string } }>, editorChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, account: { __typename?: 'AccountSummary', id: string } }> };

export type AddSetEmoteMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
  input?: InputMaybe<AddSetEmoteInput>;
}>;


export type AddSetEmoteMutation = { __typename?: 'Mutation', addSetEmote: { __typename?: 'SetEmote', name: string } };

export type UpdateSetEmoteMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
  input: UpdateSetEmoteInput;
}>;


export type UpdateSetEmoteMutation = { __typename?: 'Mutation', updateSetEmote: { __typename?: 'SetEmote', name: string, zeroWidth: boolean } };

export type RemoveSetEmoteMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
}>;


export type RemoveSetEmoteMutation = { __typename?: 'Mutation', removeSetEmote: boolean };

export type RemoveSetEditorMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  accountId: Scalars['String']['input'];
}>;


export type RemoveSetEditorMutation = { __typename?: 'Mutation', removeSetEditor: boolean };

export type PublicSetQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type PublicSetQuery = { __typename?: 'Query', set?: { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, slotsUsed: number, createdAt: string, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, approvals: { __typename?: 'EmoteApprovals', publicListing: ApprovalState, personalUse: ApprovalState }, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, private: boolean, wide: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } | null };

export type SetEditorsQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type SetEditorsQuery = { __typename?: 'Query', setEditors: Array<{ __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }> };

export type CloneSetMutationVariables = Exact<{
  id: Scalars['String']['input'];
  name?: InputMaybe<Scalars['String']['input']>;
  visibility?: InputMaybe<SetVisibility>;
}>;


export type CloneSetMutation = { __typename?: 'Mutation', cloneSet: { __typename?: 'ClonedSet', copied: number, skipped: number, set: { __typename?: 'EmoteSet', id: string, name: string } } };

export type ShopItemFieldsFragment = { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> };

export type ShopItemsQueryVariables = Exact<{
  all?: InputMaybe<Scalars['Boolean']['input']>;
}>;


export type ShopItemsQuery = { __typename?: 'Query', shopItems: Array<{ __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> }> };

export type ShopItemQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type ShopItemQuery = { __typename?: 'Query', shopItem?: { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } | null };

export type OrderFieldsFragment = { __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null };

export type StartCheckoutMutationVariables = Exact<{
  itemId: Scalars['String']['input'];
  recipient?: InputMaybe<RecipientInput>;
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
}>;


export type StartCheckoutMutation = { __typename?: 'Mutation', startCheckout: { __typename?: 'Checkout', transactionId: string, checkoutUrl?: string | null, clientToken?: string | null, environment: string, order: { __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } } };

export type OrderQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type OrderQuery = { __typename?: 'Query', order?: { __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } | null };

export type MyOrdersQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type MyOrdersQuery = { __typename?: 'Query', myOrders: { __typename?: 'OrderPage', total: number, items: Array<{ __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null }> } };

export type RedeemCodeMutationVariables = Exact<{
  code: Scalars['String']['input'];
  idempotencyKey?: InputMaybe<Scalars['String']['input']>;
}>;


export type RedeemCodeMutation = { __typename?: 'Mutation', redeemCode: { __typename?: 'RedeemResult', name: string, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type CreateShopItemMutationVariables = Exact<{
  input: ShopItemInput;
}>;


export type CreateShopItemMutation = { __typename?: 'Mutation', createShopItem: { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type UpdateShopItemMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: ShopItemInput;
}>;


export type UpdateShopItemMutation = { __typename?: 'Mutation', updateShopItem: { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type DeleteShopItemMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteShopItemMutation = { __typename?: 'Mutation', deleteShopItem: boolean };

export type AdminOrdersQueryVariables = Exact<{
  status?: InputMaybe<OrderStatus>;
  accountId?: InputMaybe<Scalars['String']['input']>;
  itemId?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AdminOrdersQuery = { __typename?: 'Query', orders: { __typename?: 'OrderPage', total: number, items: Array<{ __typename?: 'Order', providerTransactionId?: string | null, refundRejectedAt?: string | null, id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null }> } };

export type RefundOrderMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type RefundOrderMutation = { __typename?: 'Mutation', refundOrder: { __typename?: 'Order', providerTransactionId?: string | null, refundRejectedAt?: string | null, id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } };

export type CodeBatchFieldsFragment = { __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null };

export type CodeBatchesQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type CodeBatchesQuery = { __typename?: 'Query', codeBatches: { __typename?: 'CodeBatchPage', total: number, items: Array<{ __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }> } };

export type CreateCodeBatchMutationVariables = Exact<{
  input: CodeBatchInput;
}>;


export type CreateCodeBatchMutation = { __typename?: 'Mutation', createCodeBatch: { __typename?: 'CreatedCodeBatch', codes: Array<string>, batch: { __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } } };

export type DisableCodeBatchMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DisableCodeBatchMutation = { __typename?: 'Mutation', disableCodeBatch: { __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } };

export type ShopCountsQueryVariables = Exact<{
  items: Scalars['Boolean']['input'];
  orders: Scalars['Boolean']['input'];
  codes: Scalars['Boolean']['input'];
}>;


export type ShopCountsQuery = { __typename?: 'Query', shopItems?: Array<{ __typename?: 'ShopItem', id: string }>, orders?: { __typename?: 'OrderPage', total: number }, codeBatches?: { __typename?: 'CodeBatchPage', total: number } };

export type AnnouncementFieldsFragment = { __typename?: 'Announcement', id: string, kind: AnnouncementKind, text: string, linkUrl?: string | null, linkLabel?: string | null, badgeId?: string | null, startsAt?: string | null, endsAt?: string | null, endedAt?: string | null, createdAt: string, updatedAt: string, live: boolean, emote?: { __typename?: 'Emote', id: string, name: string, host: string, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null };

export type ActiveAnnouncementQueryVariables = Exact<{ [key: string]: never; }>;


export type ActiveAnnouncementQuery = { __typename?: 'Query', activeAnnouncement?: { __typename?: 'Announcement', id: string, kind: AnnouncementKind, text: string, linkUrl?: string | null, linkLabel?: string | null, badgeId?: string | null, startsAt?: string | null, endsAt?: string | null, endedAt?: string | null, createdAt: string, updatedAt: string, live: boolean, emote?: { __typename?: 'Emote', id: string, name: string, host: string, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null } | null };

export type AnnouncementsQueryVariables = Exact<{ [key: string]: never; }>;


export type AnnouncementsQuery = { __typename?: 'Query', announcements: Array<{ __typename?: 'Announcement', id: string, kind: AnnouncementKind, text: string, linkUrl?: string | null, linkLabel?: string | null, badgeId?: string | null, startsAt?: string | null, endsAt?: string | null, endedAt?: string | null, createdAt: string, updatedAt: string, live: boolean, emote?: { __typename?: 'Emote', id: string, name: string, host: string, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> };

export type CreateAnnouncementMutationVariables = Exact<{
  input: AnnouncementInput;
}>;


export type CreateAnnouncementMutation = { __typename?: 'Mutation', createAnnouncement: { __typename?: 'Announcement', id: string, kind: AnnouncementKind, text: string, linkUrl?: string | null, linkLabel?: string | null, badgeId?: string | null, startsAt?: string | null, endsAt?: string | null, endedAt?: string | null, createdAt: string, updatedAt: string, live: boolean, emote?: { __typename?: 'Emote', id: string, name: string, host: string, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null } };

export type UpdateAnnouncementMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: AnnouncementInput;
}>;


export type UpdateAnnouncementMutation = { __typename?: 'Mutation', updateAnnouncement: { __typename?: 'Announcement', id: string, kind: AnnouncementKind, text: string, linkUrl?: string | null, linkLabel?: string | null, badgeId?: string | null, startsAt?: string | null, endsAt?: string | null, endedAt?: string | null, createdAt: string, updatedAt: string, live: boolean, emote?: { __typename?: 'Emote', id: string, name: string, host: string, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null } };

export type EndAnnouncementMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type EndAnnouncementMutation = { __typename?: 'Mutation', endAnnouncement: { __typename?: 'Announcement', id: string, kind: AnnouncementKind, text: string, linkUrl?: string | null, linkLabel?: string | null, badgeId?: string | null, startsAt?: string | null, endsAt?: string | null, endedAt?: string | null, createdAt: string, updatedAt: string, live: boolean, emote?: { __typename?: 'Emote', id: string, name: string, host: string, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null } };

export type NotificationPrefsQueryVariables = Exact<{ [key: string]: never; }>;


export type NotificationPrefsQuery = { __typename?: 'Query', myNotificationPrefs: Array<{ __typename?: 'NotificationPref', kind: string, notify: boolean, toast: boolean, required: boolean }> };

export type SetNotificationPrefsMutationVariables = Exact<{
  prefs: Array<NotificationPrefInput> | NotificationPrefInput;
}>;


export type SetNotificationPrefsMutation = { __typename?: 'Mutation', setNotificationPrefs: Array<{ __typename?: 'NotificationPref', kind: string, notify: boolean, toast: boolean, required: boolean }> };

export type StaffNotesQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type StaffNotesQuery = { __typename?: 'Query', staffNotes: Array<{ __typename?: 'StaffNote', id: string, authorId?: string | null, body: string, createdAt: string }> };

export type AddStaffNoteMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  body: Scalars['String']['input'];
}>;


export type AddStaffNoteMutation = { __typename?: 'Mutation', addStaffNote: { __typename?: 'StaffNote', id: string, authorId?: string | null, body: string, createdAt: string } };

export type DeleteStaffNoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteStaffNoteMutation = { __typename?: 'Mutation', deleteStaffNote: boolean };

export type AccountSessionsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountSessionsQuery = { __typename?: 'Query', accountSessions: Array<{ __typename?: 'AccountSession', id: string, kind: SessionKind, device: string, createdAt: string, lastSeenAt: string, expiresAt: string }> };

export type AccountSubscriptionsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountSubscriptionsQuery = { __typename?: 'Query', accountSubscriptions: Array<{ __typename?: 'AccountSubscription', id: string, plan: string, state: SubscriptionState, startsAt: string, endsAt?: string | null, source: string }> };

export type GrantFieldsFragment = { __typename?: 'EntitlementGrant', kind: GrantKind, itemId: string, itemName: string, accountId: string, source: GrantSource, reason?: string | null, grantedBy?: string | null, grantedAt?: string | null, orderId?: string | null, orderName?: string | null, orderStatus?: string | null, codeBatchId?: string | null, codeBatchName?: string | null, revokedAt?: string | null, revokedBy?: string | null, revokeReason?: string | null, active: boolean };

export type EntitlementGraphQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type EntitlementGraphQuery = { __typename?: 'Query', entitlementGraph: Array<{ __typename?: 'EntitlementGrant', kind: GrantKind, itemId: string, itemName: string, accountId: string, source: GrantSource, reason?: string | null, grantedBy?: string | null, grantedAt?: string | null, orderId?: string | null, orderName?: string | null, orderStatus?: string | null, codeBatchId?: string | null, codeBatchName?: string | null, revokedAt?: string | null, revokedBy?: string | null, revokeReason?: string | null, active: boolean }> };

export type EntitlementHoldersQueryVariables = Exact<{
  kind: GrantKind;
  id: Scalars['String']['input'];
}>;


export type EntitlementHoldersQuery = { __typename?: 'Query', entitlementHolders: Array<{ __typename?: 'EntitlementGrant', kind: GrantKind, itemId: string, itemName: string, accountId: string, source: GrantSource, reason?: string | null, grantedBy?: string | null, grantedAt?: string | null, orderId?: string | null, orderName?: string | null, orderStatus?: string | null, codeBatchId?: string | null, codeBatchName?: string | null, revokedAt?: string | null, revokedBy?: string | null, revokeReason?: string | null, active: boolean }> };

export type PersonCountFieldsFragment = { __typename?: 'PersonCount', count: number, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> };

export type ActivityTopActorsQueryVariables = Exact<{
  accountId?: InputMaybe<Scalars['String']['input']>;
  emoteId?: InputMaybe<Scalars['String']['input']>;
  filter?: InputMaybe<ActivityFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type ActivityTopActorsQuery = { __typename?: 'Query', activityTopActors: Array<{ __typename?: 'PersonCount', count: number, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> };

export type AuditTopPeopleQueryVariables = Exact<{
  by?: InputMaybe<AuditPersonRole>;
  targetKind?: InputMaybe<Scalars['String']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
  action?: InputMaybe<Scalars['String']['input']>;
  since?: InputMaybe<Scalars['String']['input']>;
  until?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AuditTopPeopleQuery = { __typename?: 'Query', auditTopPeople: Array<{ __typename?: 'PersonCount', count: number, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> };

export type OrderTopBuyersQueryVariables = Exact<{
  status?: InputMaybe<OrderStatus>;
  itemId?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type OrderTopBuyersQuery = { __typename?: 'Query', orderTopBuyers: Array<{ __typename?: 'PersonCount', count: number, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> };

export type ReviewTopUploadersQueryVariables = Exact<{
  filter?: InputMaybe<ReviewQueueFilter>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type ReviewTopUploadersQuery = { __typename?: 'Query', reviewTopUploaders: Array<{ __typename?: 'PersonCount', count: number, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> };

export type EntitlementTopHoldersQueryVariables = Exact<{
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type EntitlementTopHoldersQuery = { __typename?: 'Query', entitlementTopHolders: Array<{ __typename?: 'PersonCount', count: number, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> };

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
export const StudioPaintFieldsFragmentDoc = new TypedDocumentString(`
    fragment StudioPaintFields on Paint {
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
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}
    `, {"fragmentName":"StudioPaintFields"}) as unknown as TypedDocumentString<StudioPaintFieldsFragment, unknown>;
export const WebhookFieldsFragmentDoc = new TypedDocumentString(`
    fragment WebhookFields on Webhook {
  id
  url
  events
  enabled
  disabledReason
  failedDeliveries
  createdAt
}
    `, {"fragmentName":"WebhookFields"}) as unknown as TypedDocumentString<WebhookFieldsFragment, unknown>;
export const AccountEditorFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"AccountEditorFields"}) as unknown as TypedDocumentString<AccountEditorFieldsFragment, unknown>;
export const EmoteFieldsFragmentDoc = new TypedDocumentString(`
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
    `, {"fragmentName":"EmoteFields"}) as unknown as TypedDocumentString<EmoteFieldsFragment, unknown>;
export const ReportFieldsFragmentDoc = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`, {"fragmentName":"ReportFields"}) as unknown as TypedDocumentString<ReportFieldsFragment, unknown>;
export const RoleFieldsFragmentDoc = new TypedDocumentString(`
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  denies
  public
  invisible
  color
  builtIn
  editable
  assignable
}
    `, {"fragmentName":"RoleFields"}) as unknown as TypedDocumentString<RoleFieldsFragment, unknown>;
export const RoleLimitFieldsFragmentDoc = new TypedDocumentString(`
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  max
  windowSeconds
  value
}
    `, {"fragmentName":"RoleLimitFields"}) as unknown as TypedDocumentString<RoleLimitFieldsFragment, unknown>;
export const MediaLimitFieldsFragmentDoc = new TypedDocumentString(`
    fragment MediaLimitFields on MediaLimits {
  maxBytes
  maxSide
  maxFrames
  maxFps
  maxTotalPixels
  maxDurationMs
  minSide
  maxAspect
  maxServedSide
}
    `, {"fragmentName":"MediaLimitFields"}) as unknown as TypedDocumentString<MediaLimitFieldsFragment, unknown>;
export const SetFieldsFragmentDoc = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`, {"fragmentName":"SetFields"}) as unknown as TypedDocumentString<SetFieldsFragment, unknown>;
export const CosmeticImageFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"CosmeticImageFields"}) as unknown as TypedDocumentString<CosmeticImageFieldsFragment, unknown>;
export const BadgeFieldsFragmentDoc = new TypedDocumentString(`
    fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`, {"fragmentName":"BadgeFields"}) as unknown as TypedDocumentString<BadgeFieldsFragment, unknown>;
export const PaintFieldsFragmentDoc = new TypedDocumentString(`
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
}`, {"fragmentName":"PaintFields"}) as unknown as TypedDocumentString<PaintFieldsFragment, unknown>;
export const ShopItemFieldsFragmentDoc = new TypedDocumentString(`
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`, {"fragmentName":"ShopItemFields"}) as unknown as TypedDocumentString<ShopItemFieldsFragment, unknown>;
export const OrderFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"OrderFields"}) as unknown as TypedDocumentString<OrderFieldsFragment, unknown>;
export const CodeBatchFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"CodeBatchFields"}) as unknown as TypedDocumentString<CodeBatchFieldsFragment, unknown>;
export const AnnouncementFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"AnnouncementFields"}) as unknown as TypedDocumentString<AnnouncementFieldsFragment, unknown>;
export const GrantFieldsFragmentDoc = new TypedDocumentString(`
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
    `, {"fragmentName":"GrantFields"}) as unknown as TypedDocumentString<GrantFieldsFragment, unknown>;
export const PersonCountFieldsFragmentDoc = new TypedDocumentString(`
    fragment PersonCountFields on PersonCount {
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
  count
}
    `, {"fragmentName":"PersonCountFields"}) as unknown as TypedDocumentString<PersonCountFieldsFragment, unknown>;
export const StudioPaintsDocument = new TypedDocumentString(`
    query StudioPaints {
  cosmetics {
    paints {
      ...StudioPaintFields
    }
  }
}
    fragment StudioPaintFields on Paint {
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
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}`) as unknown as TypedDocumentString<StudioPaintsQuery, StudioPaintsQueryVariables>;
export const StudioCreatePaintDocument = new TypedDocumentString(`
    mutation StudioCreatePaint($input: PaintInput!) {
  createPaint(input: $input) {
    ...StudioPaintFields
  }
}
    fragment StudioPaintFields on Paint {
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
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}`) as unknown as TypedDocumentString<StudioCreatePaintMutation, StudioCreatePaintMutationVariables>;
export const StudioUpdatePaintDocument = new TypedDocumentString(`
    mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {
  updatePaint(id: $id, input: $input) {
    ...StudioPaintFields
  }
}
    fragment StudioPaintFields on Paint {
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
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}`) as unknown as TypedDocumentString<StudioUpdatePaintMutation, StudioUpdatePaintMutationVariables>;
export const StudioCosmeticImageDocument = new TypedDocumentString(`
    query StudioCosmeticImage($id: String!) {
  cosmeticImage(id: $id) {
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
}
    `) as unknown as TypedDocumentString<StudioCosmeticImageQuery, StudioCosmeticImageQueryVariables>;
export const SetDisplayPlatformDocument = new TypedDocumentString(`
    mutation SetDisplayPlatform($platform: Platform!) {
  setDisplayPlatform(platform: $platform) {
    id
    displayPlatform
  }
}
    `) as unknown as TypedDocumentString<SetDisplayPlatformMutation, SetDisplayPlatformMutationVariables>;
export const SignOutEverywhereDocument = new TypedDocumentString(`
    mutation SignOutEverywhere {
  signOutEverywhere
}
    `) as unknown as TypedDocumentString<SignOutEverywhereMutation, SignOutEverywhereMutationVariables>;
export const UnlinkDocument = new TypedDocumentString(`
    mutation Unlink($platform: Platform!) {
  unlinkIdentity(platform: $platform) {
    id
  }
}
    `) as unknown as TypedDocumentString<UnlinkMutation, UnlinkMutationVariables>;
export const MyNotificationsDocument = new TypedDocumentString(`
    query MyNotifications {
  myNotifications {
    id
    kind
    payload
    createdAt
    read
  }
}
    `) as unknown as TypedDocumentString<MyNotificationsQuery, MyNotificationsQueryVariables>;
export const MarkReadDocument = new TypedDocumentString(`
    mutation MarkRead($ids: [String!]) {
  markNotificationsRead(ids: $ids)
}
    `) as unknown as TypedDocumentString<MarkReadMutation, MarkReadMutationVariables>;
export const UnreadCountDocument = new TypedDocumentString(`
    query UnreadCount {
  unreadNotificationCount
}
    `) as unknown as TypedDocumentString<UnreadCountQuery, UnreadCountQueryVariables>;
export const MySessionsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<MySessionsQuery, MySessionsQueryVariables>;
export const MyConsoleTokenDocument = new TypedDocumentString(`
    query MyConsoleToken {
  myConsoleToken {
    id
    createdAt
    expiresAt
  }
}
    `) as unknown as TypedDocumentString<MyConsoleTokenQuery, MyConsoleTokenQueryVariables>;
export const RevokeSessionDocument = new TypedDocumentString(`
    mutation RevokeSession($id: String!) {
  revokeSession(id: $id)
}
    `) as unknown as TypedDocumentString<RevokeSessionMutation, RevokeSessionMutationVariables>;
export const RevokeOtherSessionsDocument = new TypedDocumentString(`
    mutation RevokeOtherSessions {
  revokeOtherSessions
}
    `) as unknown as TypedDocumentString<RevokeOtherSessionsMutation, RevokeOtherSessionsMutationVariables>;
export const ExportMyDataDocument = new TypedDocumentString(`
    mutation ExportMyData {
  exportMyData
}
    `) as unknown as TypedDocumentString<ExportMyDataMutation, ExportMyDataMutationVariables>;
export const DeleteAccountDocument = new TypedDocumentString(`
    mutation DeleteAccount($confirm: String!) {
  deleteAccount(confirm: $confirm)
}
    `) as unknown as TypedDocumentString<DeleteAccountMutation, DeleteAccountMutationVariables>;
export const PendingDeletionDocument = new TypedDocumentString(`
    query PendingDeletion {
  pendingDeletion {
    requestedAt
    eraseAfter
    displayName
    avatar
  }
}
    `) as unknown as TypedDocumentString<PendingDeletionQuery, PendingDeletionQueryVariables>;
export const RestoreAccountDocument = new TypedDocumentString(`
    mutation RestoreAccount {
  restoreAccount
}
    `) as unknown as TypedDocumentString<RestoreAccountMutation, RestoreAccountMutationVariables>;
export const MyRolesDocument = new TypedDocumentString(`
    query MyRoles {
  me {
    id
    roles {
      id
      name
      public
      invisible
      color
    }
  }
}
    `) as unknown as TypedDocumentString<MyRolesQuery, MyRolesQueryVariables>;
export const MeDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<MeQuery, MeQueryVariables>;
export const AccountActivityDocument = new TypedDocumentString(`
    query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {
  accountActivityPage(
    accountId: $accountId
    before: $before
    limit: $limit
    filter: $filter
  ) {
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
    `) as unknown as TypedDocumentString<AccountActivityQuery, AccountActivityQueryVariables>;
export const EmoteActivityDocument = new TypedDocumentString(`
    query EmoteActivity($emoteId: String!, $before: String, $limit: Int, $actorId: String) {
  emoteActivityPage(
    emoteId: $emoteId
    before: $before
    limit: $limit
    actorId: $actorId
  ) {
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
    `) as unknown as TypedDocumentString<EmoteActivityQuery, EmoteActivityQueryVariables>;
export const UserDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<UserQuery, UserQueryVariables>;
export const UserDetailsDocument = new TypedDocumentString(`
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
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  denies
  public
  invisible
  color
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<UserDetailsQuery, UserDetailsQueryVariables>;
export const AccountPermissionsDocument = new TypedDocumentString(`
    query AccountPermissions($accountId: String!) {
  accountPermissions(accountId: $accountId) {
    permission
    granted
    effect
    roleId
    roleName
    owner
  }
}
    `) as unknown as TypedDocumentString<AccountPermissionsQuery, AccountPermissionsQueryVariables>;
export const AllRolesDocument = new TypedDocumentString(`
    query AllRoles {
  roles {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  denies
  public
  invisible
  color
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<AllRolesQuery, AllRolesQueryVariables>;
export const PlaceRestrictionDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<PlaceRestrictionMutation, PlaceRestrictionMutationVariables>;
export const LiftRestrictionDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<LiftRestrictionMutation, LiftRestrictionMutationVariables>;
export const AccountRestrictionsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AccountRestrictionsQuery, AccountRestrictionsQueryVariables>;
export const AssignRoleDocument = new TypedDocumentString(`
    mutation AssignRole($accountId: String!, $roleId: String!) {
  assignRole(accountId: $accountId, roleId: $roleId)
}
    `) as unknown as TypedDocumentString<AssignRoleMutation, AssignRoleMutationVariables>;
export const UnassignRoleDocument = new TypedDocumentString(`
    mutation UnassignRole($accountId: String!, $roleId: String!) {
  unassignRole(accountId: $accountId, roleId: $roleId)
}
    `) as unknown as TypedDocumentString<UnassignRoleMutation, UnassignRoleMutationVariables>;
export const AuditDocument = new TypedDocumentString(`
    query Audit($targetKind: String, $targetId: String, $actorId: String, $action: String, $since: String, $until: String, $before: String, $limit: Int) {
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
    `) as unknown as TypedDocumentString<AuditQuery, AuditQueryVariables>;
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
export const CosmeticsDocument = new TypedDocumentString(`
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<CosmeticsQuery, CosmeticsQueryVariables>;
export const UserCosmeticsDocument = new TypedDocumentString(`
    query UserCosmetics($platform: Platform!, $userIds: [String!]!) {
  userCosmetics(platform: $platform, userIds: $userIds) {
    platformUserId
    badgeId
    paintId
  }
}
    `) as unknown as TypedDocumentString<UserCosmeticsQuery, UserCosmeticsQueryVariables>;
export const AccountCosmeticsDocument = new TypedDocumentString(`
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<AccountCosmeticsQuery, AccountCosmeticsQueryVariables>;
export const AccountProfileDocument = new TypedDocumentString(`
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
      color
    }
    roleColor
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
  }
}
    `) as unknown as TypedDocumentString<AccountProfileQuery, AccountProfileQueryVariables>;
export const MyCosmeticsDocument = new TypedDocumentString(`
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<MyCosmeticsQuery, MyCosmeticsQueryVariables>;
export const SelectCosmeticsDocument = new TypedDocumentString(`
    mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {
  selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {
    badgeId
    paintId
  }
}
    `) as unknown as TypedDocumentString<SelectCosmeticsMutation, SelectCosmeticsMutationVariables>;
export const CosmeticImageDocument = new TypedDocumentString(`
    query CosmeticImage($id: String!) {
  cosmeticImage(id: $id) {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<CosmeticImageQuery, CosmeticImageQueryVariables>;
export const EntitlementsDocument = new TypedDocumentString(`
    query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {
  cosmeticEntitlements(
    kind: $kind
    accountId: $accountId
    cosmeticId: $cosmeticId
    page: $page
    perPage: $perPage
  ) {
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
    `) as unknown as TypedDocumentString<EntitlementsQuery, EntitlementsQueryVariables>;
export const CreateBadgeDocument = new TypedDocumentString(`
    mutation CreateBadge($input: CreateBadgeInput!) {
  createBadge(input: $input) {
    ...BadgeFields
  }
}
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<CreateBadgeMutation, CreateBadgeMutationVariables>;
export const UpdateBadgeDocument = new TypedDocumentString(`
    mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {
  updateBadge(id: $id, input: $input) {
    ...BadgeFields
  }
}
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<UpdateBadgeMutation, UpdateBadgeMutationVariables>;
export const RemakeCosmeticImageDocument = new TypedDocumentString(`
    mutation RemakeCosmeticImage($id: String!) {
  remakeCosmeticImage(id: $id) {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<RemakeCosmeticImageMutation, RemakeCosmeticImageMutationVariables>;
export const DeleteBadgeDocument = new TypedDocumentString(`
    mutation DeleteBadge($id: String!) {
  deleteBadge(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteBadgeMutation, DeleteBadgeMutationVariables>;
export const DeletePaintDocument = new TypedDocumentString(`
    mutation DeletePaint($id: String!) {
  deletePaint(id: $id)
}
    `) as unknown as TypedDocumentString<DeletePaintMutation, DeletePaintMutationVariables>;
export const GrantCosmeticDocument = new TypedDocumentString(`
    mutation GrantCosmetic($input: GrantCosmeticInput!) {
  grantCosmetic(input: $input) {
    cosmeticId
  }
}
    `) as unknown as TypedDocumentString<GrantCosmeticMutation, GrantCosmeticMutationVariables>;
export const RevokeCosmeticDocument = new TypedDocumentString(`
    mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {
  revokeCosmetic(
    accountId: $accountId
    kind: $kind
    cosmeticId: $cosmeticId
    reason: $reason
  )
}
    `) as unknown as TypedDocumentString<RevokeCosmeticMutation, RevokeCosmeticMutationVariables>;
export const MyApiKeysDocument = new TypedDocumentString(`
    query MyApiKeys {
  myApiKeys {
    id
    name
    prefix
    createdAt
    lastUsedAt
    usage {
      day
      requests
    }
  }
}
    `) as unknown as TypedDocumentString<MyApiKeysQuery, MyApiKeysQueryVariables>;
export const CreateApiKeyDocument = new TypedDocumentString(`
    mutation CreateApiKey($name: String!) {
  createApiKey(name: $name) {
    key
    apiKey {
      id
      name
      prefix
      createdAt
      lastUsedAt
      usage {
        day
        requests
      }
    }
  }
}
    `) as unknown as TypedDocumentString<CreateApiKeyMutation, CreateApiKeyMutationVariables>;
export const RevokeApiKeyDocument = new TypedDocumentString(`
    mutation RevokeApiKey($id: String!) {
  revokeApiKey(id: $id)
}
    `) as unknown as TypedDocumentString<RevokeApiKeyMutation, RevokeApiKeyMutationVariables>;
export const ChannelWebhooksDocument = new TypedDocumentString(`
    query ChannelWebhooks($platform: Platform!, $id: String!) {
  channelWebhooks(platform: $platform, id: $id) {
    ...WebhookFields
  }
}
    fragment WebhookFields on Webhook {
  id
  url
  events
  enabled
  disabledReason
  failedDeliveries
  createdAt
}`) as unknown as TypedDocumentString<ChannelWebhooksQuery, ChannelWebhooksQueryVariables>;
export const CreateWebhookDocument = new TypedDocumentString(`
    mutation CreateWebhook($platform: Platform!, $id: String!, $input: CreateWebhookInput!) {
  createWebhook(platform: $platform, id: $id, input: $input) {
    secret
    webhook {
      ...WebhookFields
    }
  }
}
    fragment WebhookFields on Webhook {
  id
  url
  events
  enabled
  disabledReason
  failedDeliveries
  createdAt
}`) as unknown as TypedDocumentString<CreateWebhookMutation, CreateWebhookMutationVariables>;
export const UpdateWebhookDocument = new TypedDocumentString(`
    mutation UpdateWebhook($id: String!, $input: UpdateWebhookInput!) {
  updateWebhook(id: $id, input: $input) {
    ...WebhookFields
  }
}
    fragment WebhookFields on Webhook {
  id
  url
  events
  enabled
  disabledReason
  failedDeliveries
  createdAt
}`) as unknown as TypedDocumentString<UpdateWebhookMutation, UpdateWebhookMutationVariables>;
export const DeleteWebhookDocument = new TypedDocumentString(`
    mutation DeleteWebhook($id: String!) {
  deleteWebhook(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteWebhookMutation, DeleteWebhookMutationVariables>;
export const RotateWebhookSecretDocument = new TypedDocumentString(`
    mutation RotateWebhookSecret($id: String!) {
  rotateWebhookSecret(id: $id) {
    secret
    webhook {
      ...WebhookFields
    }
  }
}
    fragment WebhookFields on Webhook {
  id
  url
  events
  enabled
  disabledReason
  failedDeliveries
  createdAt
}`) as unknown as TypedDocumentString<RotateWebhookSecretMutation, RotateWebhookSecretMutationVariables>;
export const TestWebhookDocument = new TypedDocumentString(`
    mutation TestWebhook($id: String!) {
  testWebhook(id: $id) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<TestWebhookMutation, TestWebhookMutationVariables>;
export const WebhookDeliveriesDocument = new TypedDocumentString(`
    query WebhookDeliveries($id: String!) {
  webhookDeliveries(id: $id) {
    id
    eventType
    eventId
    state
    attempts
    responseStatus
    error
    durationMs
    createdAt
    finishedAt
  }
}
    `) as unknown as TypedDocumentString<WebhookDeliveriesQuery, WebhookDeliveriesQueryVariables>;
export const EditorsDocument = new TypedDocumentString(`
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
}`) as unknown as TypedDocumentString<EditorsQuery, EditorsQueryVariables>;
export const AddAccountEditorDocument = new TypedDocumentString(`
    mutation AddAccountEditor($input: AddAccountEditorInput!) {
  addAccountEditor(input: $input) {
    ...AccountEditorFields
  }
}
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
}`) as unknown as TypedDocumentString<AddAccountEditorMutation, AddAccountEditorMutationVariables>;
export const UpdateAccountEditorDocument = new TypedDocumentString(`
    mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) {
  updateAccountEditor(
    ownerId: $ownerId
    accountId: $accountId
    permissions: $permissions
    setIds: $setIds
  ) {
    ...AccountEditorFields
  }
}
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
}`) as unknown as TypedDocumentString<UpdateAccountEditorMutation, UpdateAccountEditorMutationVariables>;
export const RemoveAccountEditorDocument = new TypedDocumentString(`
    mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {
  removeAccountEditor(ownerId: $ownerId, accountId: $accountId)
}
    `) as unknown as TypedDocumentString<RemoveAccountEditorMutation, RemoveAccountEditorMutationVariables>;
export const ReportEmoteDocument = new TypedDocumentString(`
    mutation ReportEmote($id: String!, $reason: String!) {
  reportEmote(id: $id, reason: $reason) {
    id
  }
}
    `) as unknown as TypedDocumentString<ReportEmoteMutation, ReportEmoteMutationVariables>;
export const MySetNamesDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<MySetNamesQuery, MySetNamesQueryVariables>;
export const SearchByOwnerDocument = new TypedDocumentString(`
    query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort, $after: String) {
  searchEmotes(
    query: ""
    owner: $owner
    page: $page
    perPage: $perPage
    sort: $sort
    after: $after
  ) {
    total
    page
    perPage
    next
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<SearchByOwnerQuery, SearchByOwnerQueryVariables>;
export const AdminAccountDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AdminAccountQuery, AdminAccountQueryVariables>;
export const TransferEmoteDocument = new TypedDocumentString(`
    mutation TransferEmote($id: String!, $toAccountId: String!) {
  transferEmote(id: $id, toAccountId: $toAccountId) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<TransferEmoteMutation, TransferEmoteMutationVariables>;
export const AccountEmotesDocument = new TypedDocumentString(`
    query AccountEmotes($accountId: String!, $page: Int, $perPage: Int, $after: String) {
  accountEmotes(
    accountId: $accountId
    page: $page
    perPage: $perPage
    after: $after
  ) {
    total
    next
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<AccountEmotesQuery, AccountEmotesQueryVariables>;
export const AccountReportsDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<AccountReportsQuery, AccountReportsQueryVariables>;
export const PublicAccountDocument = new TypedDocumentString(`
    query PublicAccount($id: String!) {
  account(id: $id) {
    id
    displayName
    avatar
  }
}
    `) as unknown as TypedDocumentString<PublicAccountQuery, PublicAccountQueryVariables>;
export const EmoteChannelsDocument = new TypedDocumentString(`
    query EmoteChannels($id: String!, $limit: Int, $offset: Int) {
  emoteChannels(id: $id, limit: $limit, offset: $offset) {
    platform
    login
    account {
      id
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<EmoteChannelsQuery, EmoteChannelsQueryVariables>;
export const EmoteReviewApprovalsDocument = new TypedDocumentString(`
    mutation EmoteReviewApprovals($ids: [String!]!, $input: ApprovalsInput!, $idempotencyKey: String) {
  reviewApprovals(ids: $ids, input: $input, idempotencyKey: $idempotencyKey) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<EmoteReviewApprovalsMutation, EmoteReviewApprovalsMutationVariables>;
export const SearchDocument = new TypedDocumentString(`
    query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort, $flags: [EmoteFlag!], $tags: [String!], $exact: Boolean, $window: TrendingWindow, $after: String) {
  searchEmotes(
    query: $query
    animated: $animated
    page: $page
    perPage: $perPage
    sort: $sort
    flags: $flags
    tags: $tags
    exact: $exact
    window: $window
    after: $after
  ) {
    total
    page
    perPage
    next
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<SearchQuery, SearchQueryVariables>;
export const FindPeopleDocument = new TypedDocumentString(`
    query FindPeople($query: String!, $page: Int, $perPage: Int, $after: String) {
  searchAccounts(query: $query, page: $page, perPage: $perPage, after: $after) {
    total
    page
    perPage
    next
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
    `) as unknown as TypedDocumentString<FindPeopleQuery, FindPeopleQueryVariables>;
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<EmoteQuery, EmoteQueryVariables>;
export const EmoteTrendingDocument = new TypedDocumentString(`
    query EmoteTrending($id: String!, $window: TrendingWindow) {
  emoteTrending(id: $id, window: $window) {
    rank
    channels
    window
  }
}
    `) as unknown as TypedDocumentString<EmoteTrendingQuery, EmoteTrendingQueryVariables>;
export const MyChannelsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<MyChannelsQuery, MyChannelsQueryVariables>;
export const UpdateChannelDocument = new TypedDocumentString(`
    mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {
  updateChannelSettings(platform: $platform, id: $id, input: $input) {
    activeSetId
    allowPersonalEmotes
  }
}
    `) as unknown as TypedDocumentString<UpdateChannelMutation, UpdateChannelMutationVariables>;
export const MyEmotesDocument = new TypedDocumentString(`
    query MyEmotes($page: Int, $perPage: Int) {
  myEmotes(page: $page, perPage: $perPage) {
    total
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<MyEmotesQuery, MyEmotesQueryVariables>;
export const UpdateEmoteDocument = new TypedDocumentString(`
    mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {
  updateEmote(id: $id, input: $input) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<UpdateEmoteMutation, UpdateEmoteMutationVariables>;
export const DeleteEmoteDocument = new TypedDocumentString(`
    mutation DeleteEmote($id: String!) {
  deleteEmote(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteEmoteMutation, DeleteEmoteMutationVariables>;
export const PersonalListsDocument = new TypedDocumentString(`
    query PersonalLists {
  hiddenEmotes {
    emoteId
  }
  hiddenCreators {
    accountId
  }
  favouriteEmoteIds
}
    `) as unknown as TypedDocumentString<PersonalListsQuery, PersonalListsQueryVariables>;
export const HiddenListsDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<HiddenListsQuery, HiddenListsQueryVariables>;
export const MyFavouritesDocument = new TypedDocumentString(`
    query MyFavourites($query: String, $page: Int, $perPage: Int) {
  myFavourites(query: $query, page: $page, perPage: $perPage) {
    total
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<MyFavouritesQuery, MyFavouritesQueryVariables>;
export const HideEmoteDocument = new TypedDocumentString(`
    mutation HideEmote($id: String!) {
  hideEmote(id: $id)
}
    `) as unknown as TypedDocumentString<HideEmoteMutation, HideEmoteMutationVariables>;
export const UnhideEmoteDocument = new TypedDocumentString(`
    mutation UnhideEmote($id: String!) {
  unhideEmote(id: $id)
}
    `) as unknown as TypedDocumentString<UnhideEmoteMutation, UnhideEmoteMutationVariables>;
export const HideCreatorDocument = new TypedDocumentString(`
    mutation HideCreator($accountId: String!) {
  hideCreator(accountId: $accountId)
}
    `) as unknown as TypedDocumentString<HideCreatorMutation, HideCreatorMutationVariables>;
export const UnhideCreatorDocument = new TypedDocumentString(`
    mutation UnhideCreator($accountId: String!) {
  unhideCreator(accountId: $accountId)
}
    `) as unknown as TypedDocumentString<UnhideCreatorMutation, UnhideCreatorMutationVariables>;
export const FavouriteEmoteDocument = new TypedDocumentString(`
    mutation FavouriteEmote($id: String!) {
  favouriteEmote(id: $id)
}
    `) as unknown as TypedDocumentString<FavouriteEmoteMutation, FavouriteEmoteMutationVariables>;
export const UnfavouriteEmoteDocument = new TypedDocumentString(`
    mutation UnfavouriteEmote($id: String!) {
  unfavouriteEmote(id: $id)
}
    `) as unknown as TypedDocumentString<UnfavouriteEmoteMutation, UnfavouriteEmoteMutationVariables>;
export const AccountRolesDocument = new TypedDocumentString(`
    query AccountRoles($accountId: String!) {
  accountRoles(accountId: $accountId) {
    id
    name
    position
    color
  }
}
    `) as unknown as TypedDocumentString<AccountRolesQuery, AccountRolesQueryVariables>;
export const AccountEditorsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AccountEditorsQuery, AccountEditorsQueryVariables>;
export const EditorOfDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<EditorOfQuery, EditorOfQueryVariables>;
export const AccountSetsDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<AccountSetsQuery, AccountSetsQueryVariables>;
export const ProfileMineDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<ProfileMineQuery, ProfileMineQueryVariables>;
export const ProfileEditedDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<ProfileEditedQuery, ProfileEditedQueryVariables>;
export const ReportQueueDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<ReportQueueQuery, ReportQueueQueryVariables>;
export const ResolveReportDocument = new TypedDocumentString(`
    mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {
  resolveReport(id: $id, action: $action, note: $note) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<ResolveReportMutation, ResolveReportMutationVariables>;
export const ReviewEmotesDocument = new TypedDocumentString(`
    mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {
  reviewEmotes(
    ids: $ids
    verdict: $verdict
    visibility: $visibility
    reason: $reason
  ) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<ReviewEmotesMutation, ReviewEmotesMutationVariables>;
export const ReviewApprovalsDocument = new TypedDocumentString(`
    mutation ReviewApprovals($ids: [String!]!, $input: ApprovalsInput!, $idempotencyKey: String) {
  reviewApprovals(ids: $ids, input: $input, idempotencyKey: $idempotencyKey) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<ReviewApprovalsMutation, ReviewApprovalsMutationVariables>;
export const RequeueEmotesDocument = new TypedDocumentString(`
    mutation RequeueEmotes($ids: [String!]!) {
  requeueEmotes(ids: $ids) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<RequeueEmotesMutation, RequeueEmotesMutationVariables>;
export const SetEmoteFlagsDocument = new TypedDocumentString(`
    mutation SetEmoteFlags($id: String!, $input: EmoteFlagsInput!) {
  setEmoteFlags(id: $id, input: $input) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<SetEmoteFlagsMutation, SetEmoteFlagsMutationVariables>;
export const DismissReportsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<DismissReportsMutation, DismissReportsMutationVariables>;
export const EmoteDuplicatesDocument = new TypedDocumentString(`
    query EmoteDuplicates($emoteId: String!) {
  emoteDuplicates(emoteId: $emoteId) {
    sameFile
    similarity
    emote {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<EmoteDuplicatesQuery, EmoteDuplicatesQueryVariables>;
export const ReviewReasonsDocument = new TypedDocumentString(`
    query ReviewReasons {
  reviewReasons
}
    `) as unknown as TypedDocumentString<ReviewReasonsQuery, ReviewReasonsQueryVariables>;
export const SetReviewReasonsDocument = new TypedDocumentString(`
    mutation SetReviewReasons($reasons: [String!]!) {
  setReviewReasons(reasons: $reasons)
}
    `) as unknown as TypedDocumentString<SetReviewReasonsMutation, SetReviewReasonsMutationVariables>;
export const AccountsByLoginDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AccountsByLoginQuery, AccountsByLoginQueryVariables>;
export const EmotesWithSameFileDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<EmotesWithSameFileQuery, EmotesWithSameFileQueryVariables>;
export const AdminCountsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AdminCountsQuery, AdminCountsQueryVariables>;
export const ReviewQueueDocument = new TypedDocumentString(`
    query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {
  reviewQueue(page: $page, perPage: $perPage, filter: $filter) {
    total
    counts {
      public
      limited
      personal
    }
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<ReviewQueueQuery, ReviewQueueQueryVariables>;
export const ApproveDocument = new TypedDocumentString(`
    mutation Approve($id: String!, $visibility: EmoteVisibility) {
  approveEmote(id: $id, visibility: $visibility) {
    id
    state
    visibility
  }
}
    `) as unknown as TypedDocumentString<ApproveMutation, ApproveMutationVariables>;
export const EmoteHistoryDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<EmoteHistoryQuery, EmoteHistoryQueryVariables>;
export const EmoteNotesDocument = new TypedDocumentString(`
    query EmoteNotes($emoteId: String!) {
  emoteNotes(emoteId: $emoteId) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<EmoteNotesQuery, EmoteNotesQueryVariables>;
export const AddEmoteNoteDocument = new TypedDocumentString(`
    mutation AddEmoteNote($emoteId: String!, $body: String!) {
  addEmoteNote(emoteId: $emoteId, body: $body) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<AddEmoteNoteMutation, AddEmoteNoteMutationVariables>;
export const DeleteEmoteNoteDocument = new TypedDocumentString(`
    mutation DeleteEmoteNote($id: String!) {
  deleteEmoteNote(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteEmoteNoteMutation, DeleteEmoteNoteMutationVariables>;
export const RejectDocument = new TypedDocumentString(`
    mutation Reject($id: String!, $reason: String!) {
  rejectEmote(id: $id, reason: $reason) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<RejectMutation, RejectMutationVariables>;
export const RolesDocument = new TypedDocumentString(`
    query Roles {
  roles {
    ...RoleFields
  }
  permissions
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  denies
  public
  invisible
  color
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<RolesQuery, RolesQueryVariables>;
export const LimitDefaultsDocument = new TypedDocumentString(`
    query LimitDefaults {
  limitDefaults {
    ...RoleLimitFields
  }
}
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  max
  windowSeconds
  value
}`) as unknown as TypedDocumentString<LimitDefaultsQuery, LimitDefaultsQueryVariables>;
export const RoleLimitsDocument = new TypedDocumentString(`
    query RoleLimits($id: String!) {
  roleLimits(id: $id) {
    ...RoleLimitFields
  }
}
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  max
  windowSeconds
  value
}`) as unknown as TypedDocumentString<RoleLimitsQuery, RoleLimitsQueryVariables>;
export const SetRoleLimitsDocument = new TypedDocumentString(`
    mutation SetRoleLimits($id: String!, $limits: [RoleLimitInput!]!) {
  setRoleLimits(id: $id, limits: $limits) {
    ...RoleLimitFields
  }
}
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  max
  windowSeconds
  value
}`) as unknown as TypedDocumentString<SetRoleLimitsMutation, SetRoleLimitsMutationVariables>;
export const UploadQuotaDocument = new TypedDocumentString(`
    query UploadQuota($ownerId: String) {
  uploadQuota(ownerId: $ownerId) {
    accountId
    pending
    pendingMax
    uploadsThisHour
    uploadsPerHour
    maxBytes
    maxSide
    maxFrames
    maxFps
    maxAspect
  }
}
    `) as unknown as TypedDocumentString<UploadQuotaQuery, UploadQuotaQueryVariables>;
export const PublicLimitsDocument = new TypedDocumentString(`
    query PublicLimits {
  limits {
    emote {
      ...MediaLimitFields
    }
    badge {
      ...MediaLimitFields
    }
    paint {
      ...MediaLimitFields
    }
    names {
      emote {
        min
        max
      }
      set {
        min
        max
      }
      badge {
        min
        max
      }
      paint {
        min
        max
      }
      tooltip {
        min
        max
      }
    }
    tags {
      max
      maxLen
    }
    rates {
      key
      count
      windowSeconds
    }
    account {
      key
      default
      effective
      windowSeconds
    }
  }
}
    fragment MediaLimitFields on MediaLimits {
  maxBytes
  maxSide
  maxFrames
  maxFps
  maxTotalPixels
  maxDurationMs
  minSide
  maxAspect
  maxServedSide
}`) as unknown as TypedDocumentString<PublicLimitsQuery, PublicLimitsQueryVariables>;
export const CreateRoleDocument = new TypedDocumentString(`
    mutation CreateRole($input: CreateRoleInput!) {
  createRole(input: $input) {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  denies
  public
  invisible
  color
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<CreateRoleMutation, CreateRoleMutationVariables>;
export const UpdateRoleDocument = new TypedDocumentString(`
    mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {
  updateRole(id: $id, input: $input) {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  denies
  public
  invisible
  color
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<UpdateRoleMutation, UpdateRoleMutationVariables>;
export const DeleteRoleDocument = new TypedDocumentString(`
    mutation DeleteRole($id: String!) {
  deleteRole(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteRoleMutation, DeleteRoleMutationVariables>;
export const QuickSearchDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<QuickSearchQuery, QuickSearchQueryVariables>;
export const EditableChannelsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<EditableChannelsQuery, EditableChannelsQueryVariables>;
export const MySetsDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<MySetsQuery, MySetsQueryVariables>;
export const EditSetDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<EditSetQuery, EditSetQueryVariables>;
export const SetEmotesDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<SetEmotesQuery, SetEmotesQueryVariables>;
export const SetMembersDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<SetMembersQuery, SetMembersQueryVariables>;
export const CreateSetDocument = new TypedDocumentString(`
    mutation CreateSet($input: CreateSetInput!) {
  createSet(input: $input) {
    id
  }
}
    `) as unknown as TypedDocumentString<CreateSetMutation, CreateSetMutationVariables>;
export const UpdateSetDocument = new TypedDocumentString(`
    mutation UpdateSet($id: String!, $input: UpdateSetInput!) {
  updateSet(id: $id, input: $input) {
    id
    name
    visibility
    capacity
  }
}
    `) as unknown as TypedDocumentString<UpdateSetMutation, UpdateSetMutationVariables>;
export const DeleteSetDocument = new TypedDocumentString(`
    mutation DeleteSet($id: String!) {
  deleteSet(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteSetMutation, DeleteSetMutationVariables>;
export const SetCapacityRangeDocument = new TypedDocumentString(`
    query SetCapacityRange($id: String!) {
  setCapacityRange(id: $id) {
    min
    max
    ownerMax
    used
    staff
  }
}
    `) as unknown as TypedDocumentString<SetCapacityRangeQuery, SetCapacityRangeQueryVariables>;
export const SetMenuDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<SetMenuQuery, SetMenuQueryVariables>;
export const AddSetEmoteDocument = new TypedDocumentString(`
    mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {
  addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {
    name
  }
}
    `) as unknown as TypedDocumentString<AddSetEmoteMutation, AddSetEmoteMutationVariables>;
export const UpdateSetEmoteDocument = new TypedDocumentString(`
    mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {
  updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {
    name
    zeroWidth
  }
}
    `) as unknown as TypedDocumentString<UpdateSetEmoteMutation, UpdateSetEmoteMutationVariables>;
export const RemoveSetEmoteDocument = new TypedDocumentString(`
    mutation RemoveSetEmote($setId: String!, $emoteId: String!) {
  removeSetEmote(setId: $setId, emoteId: $emoteId)
}
    `) as unknown as TypedDocumentString<RemoveSetEmoteMutation, RemoveSetEmoteMutationVariables>;
export const RemoveSetEditorDocument = new TypedDocumentString(`
    mutation RemoveSetEditor($setId: String!, $accountId: String!) {
  removeSetEditor(setId: $setId, accountId: $accountId)
}
    `) as unknown as TypedDocumentString<RemoveSetEditorMutation, RemoveSetEditorMutationVariables>;
export const PublicSetDocument = new TypedDocumentString(`
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
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  approvals {
    publicListing
    personalUse
  }
  flags {
    zeroWidth
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
}`) as unknown as TypedDocumentString<PublicSetQuery, PublicSetQueryVariables>;
export const SetEditorsDocument = new TypedDocumentString(`
    query SetEditors($id: String!) {
  setEditors(setId: $id) {
    id
    displayName
    avatar
  }
}
    `) as unknown as TypedDocumentString<SetEditorsQuery, SetEditorsQueryVariables>;
export const CloneSetDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<CloneSetMutation, CloneSetMutationVariables>;
export const ShopItemsDocument = new TypedDocumentString(`
    query ShopItems($all: Boolean) {
  shopItems(all: $all) {
    ...ShopItemFields
  }
}
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<ShopItemsQuery, ShopItemsQueryVariables>;
export const ShopItemDocument = new TypedDocumentString(`
    query ShopItem($id: String!) {
  shopItem(id: $id) {
    ...ShopItemFields
  }
}
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<ShopItemQuery, ShopItemQueryVariables>;
export const StartCheckoutDocument = new TypedDocumentString(`
    mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {
  startCheckout(
    itemId: $itemId
    recipient: $recipient
    idempotencyKey: $idempotencyKey
  ) {
    order {
      ...OrderFields
    }
    transactionId
    checkoutUrl
    clientToken
    environment
  }
}
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
}`) as unknown as TypedDocumentString<StartCheckoutMutation, StartCheckoutMutationVariables>;
export const OrderDocument = new TypedDocumentString(`
    query Order($id: String!) {
  order(id: $id) {
    ...OrderFields
  }
}
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
}`) as unknown as TypedDocumentString<OrderQuery, OrderQueryVariables>;
export const MyOrdersDocument = new TypedDocumentString(`
    query MyOrders($page: Int, $perPage: Int) {
  myOrders(page: $page, perPage: $perPage) {
    total
    items {
      ...OrderFields
    }
  }
}
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
}`) as unknown as TypedDocumentString<MyOrdersQuery, MyOrdersQueryVariables>;
export const RedeemCodeDocument = new TypedDocumentString(`
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<RedeemCodeMutation, RedeemCodeMutationVariables>;
export const CreateShopItemDocument = new TypedDocumentString(`
    mutation CreateShopItem($input: ShopItemInput!) {
  createShopItem(input: $input) {
    ...ShopItemFields
  }
}
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<CreateShopItemMutation, CreateShopItemMutationVariables>;
export const UpdateShopItemDocument = new TypedDocumentString(`
    mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {
  updateShopItem(id: $id, input: $input) {
    ...ShopItemFields
  }
}
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
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
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
}`) as unknown as TypedDocumentString<UpdateShopItemMutation, UpdateShopItemMutationVariables>;
export const DeleteShopItemDocument = new TypedDocumentString(`
    mutation DeleteShopItem($id: String!) {
  deleteShopItem(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteShopItemMutation, DeleteShopItemMutationVariables>;
export const AdminOrdersDocument = new TypedDocumentString(`
    query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {
  orders(
    status: $status
    accountId: $accountId
    itemId: $itemId
    page: $page
    perPage: $perPage
  ) {
    total
    items {
      ...OrderFields
      providerTransactionId
      refundRejectedAt
    }
  }
}
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
}`) as unknown as TypedDocumentString<AdminOrdersQuery, AdminOrdersQueryVariables>;
export const RefundOrderDocument = new TypedDocumentString(`
    mutation RefundOrder($id: String!, $reason: String!) {
  refundOrder(id: $id, reason: $reason) {
    ...OrderFields
    providerTransactionId
    refundRejectedAt
  }
}
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
}`) as unknown as TypedDocumentString<RefundOrderMutation, RefundOrderMutationVariables>;
export const CodeBatchesDocument = new TypedDocumentString(`
    query CodeBatches($page: Int, $perPage: Int) {
  codeBatches(page: $page, perPage: $perPage) {
    total
    items {
      ...CodeBatchFields
    }
  }
}
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
}`) as unknown as TypedDocumentString<CodeBatchesQuery, CodeBatchesQueryVariables>;
export const CreateCodeBatchDocument = new TypedDocumentString(`
    mutation CreateCodeBatch($input: CodeBatchInput!) {
  createCodeBatch(input: $input) {
    batch {
      ...CodeBatchFields
    }
    codes
  }
}
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
}`) as unknown as TypedDocumentString<CreateCodeBatchMutation, CreateCodeBatchMutationVariables>;
export const DisableCodeBatchDocument = new TypedDocumentString(`
    mutation DisableCodeBatch($id: String!) {
  disableCodeBatch(id: $id) {
    ...CodeBatchFields
  }
}
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
}`) as unknown as TypedDocumentString<DisableCodeBatchMutation, DisableCodeBatchMutationVariables>;
export const ShopCountsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<ShopCountsQuery, ShopCountsQueryVariables>;
export const ActiveAnnouncementDocument = new TypedDocumentString(`
    query ActiveAnnouncement {
  activeAnnouncement {
    ...AnnouncementFields
  }
}
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
}`) as unknown as TypedDocumentString<ActiveAnnouncementQuery, ActiveAnnouncementQueryVariables>;
export const AnnouncementsDocument = new TypedDocumentString(`
    query Announcements {
  announcements {
    ...AnnouncementFields
  }
}
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
}`) as unknown as TypedDocumentString<AnnouncementsQuery, AnnouncementsQueryVariables>;
export const CreateAnnouncementDocument = new TypedDocumentString(`
    mutation CreateAnnouncement($input: AnnouncementInput!) {
  createAnnouncement(input: $input) {
    ...AnnouncementFields
  }
}
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
}`) as unknown as TypedDocumentString<CreateAnnouncementMutation, CreateAnnouncementMutationVariables>;
export const UpdateAnnouncementDocument = new TypedDocumentString(`
    mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {
  updateAnnouncement(id: $id, input: $input) {
    ...AnnouncementFields
  }
}
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
}`) as unknown as TypedDocumentString<UpdateAnnouncementMutation, UpdateAnnouncementMutationVariables>;
export const EndAnnouncementDocument = new TypedDocumentString(`
    mutation EndAnnouncement($id: String!) {
  endAnnouncement(id: $id) {
    ...AnnouncementFields
  }
}
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
}`) as unknown as TypedDocumentString<EndAnnouncementMutation, EndAnnouncementMutationVariables>;
export const NotificationPrefsDocument = new TypedDocumentString(`
    query NotificationPrefs {
  myNotificationPrefs {
    kind
    notify
    toast
    required
  }
}
    `) as unknown as TypedDocumentString<NotificationPrefsQuery, NotificationPrefsQueryVariables>;
export const SetNotificationPrefsDocument = new TypedDocumentString(`
    mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {
  setNotificationPrefs(prefs: $prefs) {
    kind
    notify
    toast
    required
  }
}
    `) as unknown as TypedDocumentString<SetNotificationPrefsMutation, SetNotificationPrefsMutationVariables>;
export const StaffNotesDocument = new TypedDocumentString(`
    query StaffNotes($accountId: String!) {
  staffNotes(accountId: $accountId) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<StaffNotesQuery, StaffNotesQueryVariables>;
export const AddStaffNoteDocument = new TypedDocumentString(`
    mutation AddStaffNote($accountId: String!, $body: String!) {
  addStaffNote(accountId: $accountId, body: $body) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<AddStaffNoteMutation, AddStaffNoteMutationVariables>;
export const DeleteStaffNoteDocument = new TypedDocumentString(`
    mutation DeleteStaffNote($id: String!) {
  deleteStaffNote(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteStaffNoteMutation, DeleteStaffNoteMutationVariables>;
export const AccountSessionsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AccountSessionsQuery, AccountSessionsQueryVariables>;
export const AccountSubscriptionsDocument = new TypedDocumentString(`
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
    `) as unknown as TypedDocumentString<AccountSubscriptionsQuery, AccountSubscriptionsQueryVariables>;
export const EntitlementGraphDocument = new TypedDocumentString(`
    query EntitlementGraph($accountId: String!) {
  entitlementGraph(accountId: $accountId) {
    ...GrantFields
  }
}
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
}`) as unknown as TypedDocumentString<EntitlementGraphQuery, EntitlementGraphQueryVariables>;
export const EntitlementHoldersDocument = new TypedDocumentString(`
    query EntitlementHolders($kind: GrantKind!, $id: String!) {
  entitlementHolders(kind: $kind, id: $id) {
    ...GrantFields
  }
}
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
}`) as unknown as TypedDocumentString<EntitlementHoldersQuery, EntitlementHoldersQueryVariables>;
export const ActivityTopActorsDocument = new TypedDocumentString(`
    query ActivityTopActors($accountId: String, $emoteId: String, $filter: ActivityFilter, $limit: Int) {
  activityTopActors(
    accountId: $accountId
    emoteId: $emoteId
    filter: $filter
    limit: $limit
  ) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<ActivityTopActorsQuery, ActivityTopActorsQueryVariables>;
export const AuditTopPeopleDocument = new TypedDocumentString(`
    query AuditTopPeople($by: AuditPersonRole, $targetKind: String, $targetId: String, $actorId: String, $action: String, $since: String, $until: String, $limit: Int) {
  auditTopPeople(
    by: $by
    targetKind: $targetKind
    targetId: $targetId
    actorId: $actorId
    action: $action
    since: $since
    until: $until
    limit: $limit
  ) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<AuditTopPeopleQuery, AuditTopPeopleQueryVariables>;
export const OrderTopBuyersDocument = new TypedDocumentString(`
    query OrderTopBuyers($status: OrderStatus, $itemId: String, $limit: Int) {
  orderTopBuyers(status: $status, itemId: $itemId, limit: $limit) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<OrderTopBuyersQuery, OrderTopBuyersQueryVariables>;
export const ReviewTopUploadersDocument = new TypedDocumentString(`
    query ReviewTopUploaders($filter: ReviewQueueFilter, $limit: Int) {
  reviewTopUploaders(filter: $filter, limit: $limit) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<ReviewTopUploadersQuery, ReviewTopUploadersQueryVariables>;
export const EntitlementTopHoldersDocument = new TypedDocumentString(`
    query EntitlementTopHolders($limit: Int) {
  entitlementTopHolders(limit: $limit) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<EntitlementTopHoldersQuery, EntitlementTopHoldersQueryVariables>;